use std::{io, time::Duration};

use clap::Parser;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseButton,
        MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend, layout::Rect};

use sushi::app::{App, AppMode, ConfigItem};
use sushi::config::{Config, ConnectionMode, ResolvedServer};
use sushi::handlers::{get_layout, handle_mouse_event, is_in_rect};
use sushi::ssh::client::build_ssh_args;
use sushi::state;
use sushi::ui;

// ─── CLI ─────────────────────────────────────────────────────────────────────

/// 🍣 Sushi — terminal SSH connection manager
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Chemin vers le fichier de configuration (défaut : ~/.sushi.yml)
    #[arg(short, long, value_name = "FILE")]
    config: Option<String>,

    /// Connexion directe sans TUI : [user@]host[:port]
    #[arg(long, value_name = "[USER@]HOST[:PORT]", conflicts_with_all = ["rebond", "bastion"])]
    direct: Option<String>,

    /// Connexion via jump host sans TUI : [user@]host[:port]
    #[arg(long, value_name = "[USER@]HOST[:PORT]", conflicts_with_all = ["direct", "bastion"])]
    rebond: Option<String>,

    /// Connexion via bastion sans TUI : [user@]host[:port]
    #[arg(long, value_name = "[USER@]HOST[:PORT]", conflicts_with_all = ["direct", "rebond"])]
    bastion: Option<String>,

    /// Forcer un utilisateur SSH (remplace la config et le user@host)
    #[arg(short, long, value_name = "USER")]
    user: Option<String>,

    /// Forcer un port SSH
    #[arg(short, long, value_name = "PORT")]
    port: Option<u16>,

    /// Forcer une clé SSH
    #[arg(short, long, value_name = "PATH")]
    key: Option<String>,

    /// Activer le mode verbeux SSH (-v)
    #[arg(short, long)]
    verbose: bool,
}

// ─── Config par défaut ───────────────────────────────────────────────────────

const DEFAULT_CONFIG: &str = r#"
defaults:
  user: "admin"
  ssh_key: "~/.ssh/id_rsa"
  ssh_options:
    - "StrictHostKeyChecking=no"
    - "UserKnownHostsFile=/dev/null"

groups:
  - name: "Example Project"
    user: "dev"
    environments:
      - name: "Production"
        servers:
          - name: "web-01"
            host: "192.168.1.10"
          - name: "db-01"
            host: "192.168.1.11"
      - name: "Staging"
        servers:
          - name: "web-stg"
            host: "192.168.1.20"
            mode: "jump"
"#;

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Décompose `[user@]host[:port]` en ses parties.
fn parse_target(s: &str) -> (Option<String>, String, Option<u16>) {
    let (user, rest) = if let Some((u, r)) = s.split_once('@') {
        (Some(u.to_string()), r)
    } else {
        (None, s)
    };
    let (host, port) = if let Some((h, p)) = rest.split_once(':') {
        (h.to_string(), p.parse().ok())
    } else {
        (rest.to_string(), None)
    };
    (user, host, port)
}

/// Construit un `ResolvedServer` minimal pour une connexion sans TUI.
fn build_adhoc_server(
    target: &str,
    mode: ConnectionMode,
    cli: &Cli,
    config: &Config,
) -> ResolvedServer {
    let (parsed_user, host, parsed_port) = parse_target(target);
    let d = config.defaults.clone().unwrap_or_default();

    let user = cli
        .user
        .clone()
        .or(parsed_user)
        .or(d.user.clone())
        .unwrap_or_else(|| "root".to_string());
    let port = cli.port.or(parsed_port).or(d.ssh_port).unwrap_or(22);
    let ssh_key = cli
        .key
        .clone()
        .or(d.ssh_key.clone())
        .unwrap_or_else(|| "~/.ssh/id_rsa".to_string());
    let ssh_options = d.ssh_options.clone().unwrap_or_default();

    let jump_host = d.rebond.as_ref().map(|jumps| {
        jumps
            .iter()
            .map(|j| {
                let h = j.host.as_deref().unwrap_or("");
                let u = j.user.as_deref().unwrap_or(&user);
                format!("{u}@{h}")
            })
            .collect::<Vec<_>>()
            .join(",")
    });
    let bastion_host = d.bastion.as_ref().and_then(|b| b.host.clone());
    let bastion_user = d.bastion.as_ref().and_then(|b| b.user.clone());
    let bastion_template = d
        .bastion
        .as_ref()
        .and_then(|b| b.template.clone())
        .unwrap_or_else(|| "{target_user}@%n:SSH:{bastion_user}".to_string());

    ResolvedServer {
        group_name: String::new(),
        env_name: String::new(),
        name: host.clone(),
        host,
        user,
        port,
        ssh_key,
        ssh_options,
        default_mode: mode,
        jump_host,
        bastion_host,
        bastion_user,
        bastion_template,
        use_system_ssh_config: false,
    }
}

// ─── main ─────────────────────────────────────────────────────────────────────

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    // Résolution du chemin de config
    let config_path_str = cli
        .config
        .clone()
        .unwrap_or_else(|| shellexpand::tilde("~/.sushi.yml").into_owned());
    let config_path = std::path::Path::new(&config_path_str);

    if !config_path.exists()
        && let Err(e) = std::fs::write(config_path, DEFAULT_CONFIG)
    {
        eprintln!("Failed to create default config: {}", e);
        return Err(e);
    }

    let config_content = match std::fs::read_to_string(config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to read config: {}", e);
            return Err(e);
        }
    };

    let mut config: Config = match serde_yaml::from_str(&config_content) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to parse YAML config: {}", e);
            return Err(io::Error::new(io::ErrorKind::InvalidData, e));
        }
    };
    config.sort();

    // ── Connexion directe sans TUI ──────────────────────────────────────────
    let cli_mode_target: Option<(ConnectionMode, String)> = cli
        .direct
        .as_deref()
        .map(|t| (ConnectionMode::Direct, t.to_string()))
        .or_else(|| {
            cli.rebond
                .as_deref()
                .map(|t| (ConnectionMode::Jump, t.to_string()))
        })
        .or_else(|| {
            cli.bastion
                .as_deref()
                .map(|t| (ConnectionMode::Bastion, t.to_string()))
        });

    if let Some((mode, target)) = cli_mode_target {
        let server = build_adhoc_server(&target, mode, &cli, &config);
        if let Err(e) = sushi::ssh::client::connect(&server, mode, cli.verbose) {
            eprintln!("SSH Connection Error: {}", e);
            return Err(io::Error::other(e.to_string()));
        }
        return Ok(()); // exec() remplace le process ; on n'arrive jamais ici
    }

    // ── Mode TUI normal ─────────────────────────────────────────────────────
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(config).map_err(io::Error::other)?;

    let res = run_app(&mut terminal, &mut app);

    // Persiste l'état d'expansion avant de quitter la TUI
    state::save_state(&app.to_app_state());

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    match res {
        Ok(AppResult::Exit) => {}
        Ok(AppResult::Connect(server, mode, verbose)) => {
            if let Err(e) = sushi::ssh::client::connect(&server, mode, verbose) {
                eprintln!("SSH Connection Error: {}", e);
            }
        }
        Err(err) => {
            eprintln!("Application Error: {:?}", err);
        }
    }

    Ok(())
}

// ─── TUI ─────────────────────────────────────────────────────────────────────

pub enum AppResult {
    Exit,
    Connect(Box<sushi::config::ResolvedServer>, ConnectionMode, bool),
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<AppResult> {
    let mut last_click_time = std::time::Instant::now();
    let mut last_click_pos = (0, 0);

    loop {
        let size_obj = terminal.size()?;
        let size = Rect::new(0, 0, size_obj.width, size_obj.height);

        terminal.draw(|f| ui::draw(f, app))?;

        // Expire le message de statut après 3 secondes
        if let Some((_, ts)) = &app.status_message
            && ts.elapsed() > Duration::from_secs(3)
        {
            app.status_message = None;
        }

        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key) => {
                    if app.app_mode != AppMode::Normal {
                        // En mode erreur : n'importe quelle touche ferme le panneau
                        match key.code {
                            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => app.clear_error(),
                            _ => {}
                        }
                    } else if app.is_searching {
                        match key.code {
                            KeyCode::Enter | KeyCode::Esc => {
                                app.is_searching = false;
                            }
                            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                app.search_query.clear();
                                app.selected_index = 0;
                                app.list_state.select(Some(0));
                                app.invalidate_cache();
                            }
                            KeyCode::Char(c) => {
                                app.search_query.push(c);
                                app.selected_index = 0;
                                app.list_state.select(Some(0));
                                app.invalidate_cache();
                            }
                            KeyCode::Backspace => {
                                app.search_query.pop();
                                app.selected_index = 0;
                                app.list_state.select(Some(0));
                                app.invalidate_cache();
                            }
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q') => {
                                return Ok(AppResult::Exit);
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                app.next();
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                app.previous();
                            }
                            KeyCode::Tab => {
                                app.connection_mode = app.connection_mode.next();
                            }
                            KeyCode::Char('1') => {
                                app.connection_mode = ConnectionMode::Direct;
                            }
                            KeyCode::Char('2') => {
                                app.connection_mode = ConnectionMode::Jump;
                            }
                            KeyCode::Char('3') => {
                                app.connection_mode = ConnectionMode::Bastion;
                            }
                            KeyCode::Char('v') => {
                                app.verbose_mode = !app.verbose_mode;
                            }
                            KeyCode::Char('y') => {
                                let items = app.get_visible_items();
                                if let Some(ConfigItem::Server(server)) =
                                    items.get(app.selected_index)
                                {
                                    match build_ssh_args(
                                        server,
                                        app.connection_mode,
                                        app.verbose_mode,
                                    ) {
                                        Ok(args) => {
                                            let cmd = format!("ssh {}", args.join(" "));
                                            match app.clipboard.as_mut().map(|cb| cb.set_text(&cmd))
                                            {
                                                Some(Ok(_)) => {
                                                    app.set_status_message(format!("Copied: {cmd}"))
                                                }
                                                Some(Err(e)) => app.set_status_message(format!(
                                                    "Clipboard error: {e}"
                                                )),
                                                None => app.set_status_message(
                                                    "Clipboard unavailable".to_string(),
                                                ),
                                            }
                                        }
                                        Err(e) => app.set_status_message(format!("SSH error: {e}")),
                                    }
                                }
                            }
                            KeyCode::Char('/') => {
                                app.is_searching = true;
                            }
                            KeyCode::Char(' ') => {
                                app.toggle_expansion();
                            }
                            KeyCode::Enter => {
                                let action = {
                                    let items = app.get_visible_items();
                                    match items.get(app.selected_index) {
                                        Some(ConfigItem::Server(server)) => {
                                            match build_ssh_args(
                                                server,
                                                app.connection_mode,
                                                app.verbose_mode,
                                            ) {
                                                Ok(_) => Some(Ok(Box::new((**server).clone()))),
                                                Err(e) => Some(Err(format!("{e}"))),
                                            }
                                        }
                                        _ => None,
                                    }
                                };
                                match action {
                                    Some(Ok(server)) => {
                                        return Ok(AppResult::Connect(
                                            server,
                                            app.connection_mode,
                                            app.verbose_mode,
                                        ));
                                    }
                                    Some(Err(msg)) => app.set_error(msg),
                                    None => app.toggle_expansion(),
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Event::Mouse(mouse) => {
                    if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                        let handled = handle_mouse_event(mouse, app, size)?;

                        let now = std::time::Instant::now();
                        if handled
                            && now.duration_since(last_click_time) < Duration::from_millis(400)
                            && last_click_pos == (mouse.column, mouse.row)
                        {
                            let layout = get_layout(size);
                            if is_in_rect(mouse.column, mouse.row, layout.list_area) {
                                let action = {
                                    let items = app.get_visible_items();
                                    match items.get(app.selected_index) {
                                        Some(ConfigItem::Server(server)) => {
                                            match build_ssh_args(
                                                server,
                                                app.connection_mode,
                                                app.verbose_mode,
                                            ) {
                                                Ok(_) => Some(Ok(Box::new((**server).clone()))),
                                                Err(e) => Some(Err(format!("{e}"))),
                                            }
                                        }
                                        _ => None,
                                    }
                                };
                                match action {
                                    Some(Ok(server)) => {
                                        return Ok(AppResult::Connect(
                                            server,
                                            app.connection_mode,
                                            app.verbose_mode,
                                        ));
                                    }
                                    Some(Err(msg)) => app.set_error(msg),
                                    None => {}
                                }
                            }
                        }
                        last_click_time = now;
                        last_click_pos = (mouse.column, mouse.row);
                    }
                }
                _ => {}
            }
        }
    }
}
