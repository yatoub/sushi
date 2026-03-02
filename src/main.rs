use std::{collections::HashSet, io, process, time::Duration};

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

use susshi::app::{App, AppMode, CmdState, ConfigItem, ScpState, TunnelOverlayState};
use susshi::config::{Config, ConnectionMode, IncludeWarning, ResolvedServer, undefined_vars};
use susshi::handlers::{get_layout, handle_mouse_event, is_in_rect};
use susshi::import;
use susshi::probe::ProbeState;
use susshi::ssh::client::build_ssh_args;
use susshi::ssh::sftp::ScpDirection;
use susshi::state;
use susshi::ui;

// ─── CLI ─────────────────────────────────────────────────────────────────────

/// 🍣 susshi — terminal SSH connection manager
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Chemin vers le fichier de configuration (défaut : ~/.susshi.yml)
    #[arg(short, long, value_name = "FILE")]
    config: Option<String>,

    /// Connexion directe sans TUI : [user@]host[:port]
    #[arg(long, value_name = "[USER@]HOST[:PORT]", conflicts_with_all = ["jump", "wallix"])]
    direct: Option<String>,

    /// Connexion via jump host sans TUI : [user@]host[:port]
    #[arg(long, value_name = "[USER@]HOST[:PORT]", conflicts_with_all = ["direct", "wallix"])]
    jump: Option<String>,

    /// Connexion via wallix sans TUI : [user@]host[:port]
    #[arg(long, value_name = "[USER@]HOST[:PORT]", conflicts_with_all = ["direct", "jump"])]
    wallix: Option<String>,

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

    /// Valider la configuration et quitter (code 0 = OK, 1 = erreur bloquante).
    #[arg(long)]
    validate: bool,

    /// Importer ~/.ssh/config et générer un YAML susshi.
    #[arg(long, conflicts_with_all = ["validate", "direct", "jump", "wallix"])]
    import_ssh_config: bool,

    /// Chemin du fichier ssh_config à importer (défaut : ~/.ssh/config).
    #[arg(long, value_name = "FILE", requires = "import_ssh_config")]
    ssh_config_path: Option<String>,

    /// Fichier de sortie pour --import-ssh-config (défaut : stdout).
    #[arg(long, value_name = "FILE", requires = "import_ssh_config")]
    output: Option<String>,

    /// Afficher le résultat sans écrire de fichier (pour --import-ssh-config).
    #[arg(long, requires = "import_ssh_config")]
    dry_run: bool,

    /// Exporter la configuration vers un format externe : "ansible".
    #[arg(long, value_name = "FORMAT", conflicts_with_all = ["validate", "direct", "jump", "wallix", "import_ssh_config"])]
    export: Option<String>,

    /// Fichier de sortie pour --export (défaut : stdout).
    #[arg(long = "export-output", value_name = "FILE", requires = "export")]
    export_output: Option<String>,

    /// Filtre pour --export : texte et/ou #tag (même syntaxe que la recherche TUI).
    #[arg(long = "export-filter", value_name = "QUERY", requires = "export")]
    export_filter: Option<String>,
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

/// Valide le fichier de configuration, affiche les diagnostics et quitte le processus.
///
/// Code de sortie :
/// - `0` : configuration valide (avec ou sans avertissements)
/// - `1` : fichier introuvable ou erreur de parsing
fn validate_config(config_path: &std::path::Path) {
    if !config_path.exists() {
        eprintln!(
            "ERREUR : fichier de configuration introuvable : {}",
            config_path.display()
        );
        process::exit(1);
    }

    let mut stack = HashSet::new();
    match Config::load_merged(config_path, &mut stack) {
        Err(e) => {
            eprintln!("ERREUR : {e}");
            process::exit(1);
        }
        Ok((config, inc_warnings, val_warnings)) => {
            let mut has_error = false;

            for w in &inc_warnings {
                match w {
                    IncludeWarning::LoadError { label, path, error } => {
                        eprintln!("[ERREUR include] {label} ({path}): {error}");
                        has_error = true;
                    }
                    IncludeWarning::Circular { label, path } => {
                        eprintln!("[WARN  circular] {label} ({path})");
                    }
                }
            }
            for w in &val_warnings {
                eprintln!("[WARN  yaml]    {w}");
            }
            // Vérification des variables de template non définies
            let empty = std::collections::HashMap::new();
            let mut vars_warnings: usize = 0;
            if let Ok(resolved_servers) = config.resolve() {
                for srv in &resolved_servers {
                    let fields = [
                        ("name", srv.name.as_str()),
                        ("host", srv.host.as_str()),
                        ("user", srv.user.as_str()),
                        ("ssh_key", srv.ssh_key.as_str()),
                    ];
                    for (field_name, value) in fields {
                        for var in undefined_vars(value, &empty) {
                            eprintln!(
                                "[WARN  vars]    {} ({}/{}): champ \u{ab} {} \u{bb} contient \u{ab} {{{{ {} }}}} \u{bb} non d\u{e9}fini",
                                srv.name, srv.namespace, srv.group_name, field_name, var
                            );
                            vars_warnings += 1;
                        }
                    }
                }
            }
            let total_warnings = inc_warnings.len() + val_warnings.len() + vars_warnings;
            if has_error {
                process::exit(1);
            } else if total_warnings == 0 {
                println!("Configuration valide \u{2713}");
            } else {
                println!(
                    "Configuration valide avec {} avertissement(s)",
                    total_warnings
                );
            }
            process::exit(0);
        }
    }
}

/// Importe `~/.ssh/config` et écrit (ou affiche) le YAML susshi généré.
fn run_import_ssh_config(cli: &Cli) {
    let default_path = shellexpand::tilde("~/.ssh/config").into_owned();
    let path_str = cli.ssh_config_path.as_deref().unwrap_or(&default_path);
    let path = std::path::Path::new(path_str);

    let result = import::import_ssh_config(path);

    for w in &result.warnings {
        eprintln!("[WARN] {w}");
    }

    if result.entries.is_empty() {
        eprintln!("Aucune entrée trouvée dans {path_str}");
        process::exit(1);
    }

    let yaml = import::import_to_yaml(&result.entries);

    if cli.dry_run {
        print!("{yaml}");
        eprintln!(
            "{} entrée(s) importée(s) (dry-run, rien n\'a été écrit).",
            result.entries.len()
        );
        process::exit(0);
    }

    match &cli.output {
        Some(out_path) => {
            if let Err(e) = std::fs::write(out_path, &yaml) {
                eprintln!("Erreur écriture {out_path} : {e}");
                process::exit(1);
            }
            println!(
                "{} entrée(s) importée(s) → {out_path}",
                result.entries.len()
            );
        }
        None => {
            print!("{yaml}");
            eprintln!("{} entrée(s) importée(s).", result.entries.len());
        }
    }
    process::exit(0);
}

/// Exporte la configuration susshi vers un inventaire au format `format`.
///
/// Actuellement, seul `"ansible"` est supporté.
fn run_export(cli: &Cli, config: &Config) {
    use susshi::export::ansible;

    let format = cli.export.as_deref().unwrap_or("");
    if format != "ansible" {
        eprintln!("Format d'export inconnu : {format}. Formats supportés : ansible");
        process::exit(1);
    }

    let servers = match config.resolve() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Erreur lors de la résolution de la configuration : {e}");
            process::exit(1);
        }
    };

    let filter = cli.export_filter.as_deref().unwrap_or("");
    let filtered = ansible::filter_servers(&servers, filter);

    if filtered.is_empty() {
        eprintln!("Aucun serveur ne correspond au filtre {:?}.", filter);
        process::exit(1);
    }

    let yaml = ansible::to_ansible_yaml(&filtered);

    match &cli.export_output {
        Some(path) => {
            if let Err(e) = std::fs::write(path, &yaml) {
                eprintln!("Erreur écriture {path} : {e}");
                process::exit(1);
            }
            eprintln!("{} serveur(s) exporté(s) → {path}", filtered.len());
        }
        None => {
            print!("{yaml}");
            eprintln!("{} serveur(s) exporté(s).", filtered.len());
        }
    }
    process::exit(0);
}

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

    let jump_host = d.jump.as_ref().map(|jumps| {
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
    let bastion_host = d.wallix.as_ref().and_then(|b| b.host.clone());
    let bastion_user = d.wallix.as_ref().and_then(|b| b.user.clone());
    let bastion_template = d
        .wallix
        .as_ref()
        .and_then(|b| b.template.clone())
        .unwrap_or_else(|| "{target_user}@%n:SSH:{bastion_user}".to_string());

    ResolvedServer {
        namespace: String::new(),
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
        use_system_ssh_config: d.use_system_ssh_config.unwrap_or(false),
        probe_filesystems: vec![],
        tunnels: vec![],
        tags: vec![],
        control_master: false,
        control_path: String::new(),
        control_persist: "10m".to_string(),
        pre_connect_hook: d
            .pre_connect_hook
            .as_deref()
            .map(|h| shellexpand::tilde(h).into_owned()),
        post_disconnect_hook: d
            .post_disconnect_hook
            .as_deref()
            .map(|h| shellexpand::tilde(h).into_owned()),
        hook_timeout_secs: d.hook_timeout_secs.unwrap_or(5),
    }
}

// ─── main ─────────────────────────────────────────────────────────────────────

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    // Résolution du chemin de config
    let config_path_str = cli
        .config
        .clone()
        .unwrap_or_else(|| shellexpand::tilde("~/.susshi.yml").into_owned());
    let config_path = std::path::Path::new(&config_path_str);

    // ── Mode validation ─────────────────────────────────────────────────────
    if cli.validate {
        validate_config(config_path);
        // validate_config appelle process::exit() — on ne revient jamais ici.
    }
    // ── Mode import ssh_config ───────────────────────────────────────────────────
    if cli.import_ssh_config {
        run_import_ssh_config(&cli);
        // run_import_ssh_config appelle process::exit()
    }
    if !config_path.exists()
        && let Err(e) = std::fs::write(config_path, DEFAULT_CONFIG)
    {
        eprintln!("Failed to create default config: {}", e);
        return Err(e);
    }

    let (config, warnings, val_warnings) =
        match Config::load_merged(config_path, &mut HashSet::new()) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to load config: {}", e);
                return Err(io::Error::new(io::ErrorKind::InvalidData, e.to_string()));
            }
        };

    // ── Connexion directe sans TUI ──────────────────────────────────────────
    if cli.export.is_some() {
        run_export(&cli, &config);
        // run_export appelle process::exit()
    }

    let cli_mode_target: Option<(ConnectionMode, String)> = cli
        .direct
        .as_deref()
        .map(|t| (ConnectionMode::Direct, t.to_string()))
        .or_else(|| {
            cli.jump
                .as_deref()
                .map(|t| (ConnectionMode::Jump, t.to_string()))
        })
        .or_else(|| {
            cli.wallix
                .as_deref()
                .map(|t| (ConnectionMode::Wallix, t.to_string()))
        });

    if let Some((mode, target)) = cli_mode_target {
        let server = build_adhoc_server(&target, mode, &cli, &config);
        if let Err(e) =
            susshi::hooks::run_hook(server.pre_connect_hook.as_deref().unwrap_or(""), &server)
        {
            eprintln!("Hook pre_connect a annulé la connexion : {e}");
            return Err(io::Error::other(e.to_string()));
        }
        // post_disconnect_hook non supporté ici : exec() remplace le processus.
        if let Err(e) = susshi::ssh::client::connect(&server, mode, cli.verbose) {
            eprintln!("SSH Connection Error: {}", e);
            return Err(io::Error::other(e.to_string()));
        }
        return Ok(()); // exec() remplace le process ; on n'arrive jamais ici
    }

    // ── Mode TUI normal ─────────────────────────────────────────────────────
    let mut app = App::new(config, warnings, config_path.to_path_buf(), val_warnings)
        .map_err(io::Error::other)?;

    loop {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let res = run_app(&mut terminal, &mut app);

        // Persiste l'état avant de quitter la TUI
        state::save_state(&app.to_app_state());

        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        match res {
            Ok(AppResult::Exit) => break,
            Ok(AppResult::Connect(server, mode, verbose)) => {
                if app.keep_open {
                    // Connexion bloquante : SSH tourne comme sous-processus,
                    // la TUI redémarre automatiquement après la déconnexion.
                    if let Err(e) = susshi::hooks::run_hook(
                        server.pre_connect_hook.as_deref().unwrap_or(""),
                        &server,
                    ) {
                        eprintln!("Hook pre_connect a annulé la connexion : {e}");
                    } else {
                        if let Err(e) =
                            susshi::ssh::client::connect_blocking(&server, mode, verbose)
                        {
                            eprintln!("SSH Connection Error: {}", e);
                        }
                        let _ = susshi::hooks::run_hook(
                            server.post_disconnect_hook.as_deref().unwrap_or(""),
                            &server,
                        );
                    }
                    // Boucle → ré-ouvre la TUI
                } else {
                    // Comportement historique : exec() remplace le processus.
                    if let Err(e) = susshi::hooks::run_hook(
                        server.pre_connect_hook.as_deref().unwrap_or(""),
                        &server,
                    ) {
                        eprintln!("Hook pre_connect a annulé la connexion : {e}");
                    } else {
                        // post_disconnect_hook non supporté ici : exec() remplace le processus.
                        if let Err(e) = susshi::ssh::client::connect(&server, mode, verbose) {
                            eprintln!("SSH Connection Error: {}", e);
                        }
                    }
                    break;
                }
            }
            Err(err) => {
                eprintln!("Application Error: {:?}", err);
                break;
            }
        }
    }

    Ok(())
}

// ─── TUI ─────────────────────────────────────────────────────────────────────

pub enum AppResult {
    Exit,
    Connect(Box<susshi::config::ResolvedServer>, ConnectionMode, bool),
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

        // Lit le résultat du diagnostic si un thread tourne
        if let Some(rx) = &app.probe_rx
            && let Ok(result) = rx.try_recv()
        {
            app.probe_state = match result {
                Ok(probe) => ProbeState::Done(probe),
                Err(msg) => ProbeState::Error(msg),
            };
            app.probe_rx = None;
        }

        // Lit le résultat de la commande ad-hoc si un thread tourne
        app.poll_cmd();

        // Sonde l'état des tunnels SSH actifs (détecte les fins inopinées)
        app.poll_tunnel_events();

        // Sonde les évènements du transfert SCP en cours
        app.poll_scp_events();

        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key) => {
                    if app.app_mode != AppMode::Normal {
                        // En mode erreur : n'importe quelle touche ferme le panneau
                        match key.code {
                            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => app.clear_error(),
                            _ => {}
                        }
                    } else if matches!(app.cmd_state, CmdState::Prompting(_)) {
                        // Mode saisie commande ad-hoc
                        match key.code {
                            KeyCode::Esc => {
                                app.reset_cmd();
                            }
                            KeyCode::Enter => {
                                if let CmdState::Prompting(buf) = app.cmd_state.clone() {
                                    if !buf.trim().is_empty() {
                                        if let Some(server) = app.selected_server() {
                                            app.start_cmd(&server, buf.trim().to_string());
                                        }
                                    } else {
                                        app.reset_cmd();
                                    }
                                }
                            }
                            KeyCode::Char(c) => {
                                if let CmdState::Prompting(ref mut buf) = app.cmd_state {
                                    buf.push(c);
                                }
                            }
                            KeyCode::Backspace => {
                                if let CmdState::Prompting(ref mut buf) = app.cmd_state {
                                    buf.pop();
                                }
                            }
                            _ => {}
                        }
                    } else if matches!(app.scp_state, ScpState::SelectingDirection) {
                        // Sélection de la direction SCP
                        match key.code {
                            KeyCode::Up | KeyCode::Char('u') | KeyCode::Char('U') => {
                                app.scp_select_direction(ScpDirection::Upload);
                            }
                            KeyCode::Down | KeyCode::Char('d') | KeyCode::Char('D') => {
                                app.scp_select_direction(ScpDirection::Download);
                            }
                            KeyCode::Esc | KeyCode::Char('q') => {
                                app.close_scp_overlay();
                            }
                            _ => {}
                        }
                    } else if matches!(app.scp_state, ScpState::FillingForm { .. }) {
                        // Formulaire SCP
                        match key.code {
                            KeyCode::Char(c) => app.scp_form_char(c),
                            KeyCode::Backspace => app.scp_form_backspace(),
                            KeyCode::Tab | KeyCode::BackTab => app.scp_form_next_field(),
                            KeyCode::Enter => app.scp_form_submit(),
                            KeyCode::Esc => app.close_scp_overlay(),
                            _ => {}
                        }
                    } else if matches!(app.scp_state, ScpState::Done { .. } | ScpState::Error(_)) {
                        // Résultat SCP — n'importe quelle touche ferme
                        match key.code {
                            KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q') => {
                                app.dismiss_scp_result();
                            }
                            _ => {}
                        }
                    } else if matches!(&app.tunnel_overlay, Some(TunnelOverlayState::Form(_))) {
                        // Mode formulaire d'édition / création de tunnel
                        match key.code {
                            KeyCode::Char(c) => app.tunnel_form_char(c),
                            KeyCode::Backspace => app.tunnel_form_backspace(),
                            KeyCode::Tab => app.tunnel_form_next_field(),
                            KeyCode::BackTab => app.tunnel_form_prev_field(),
                            KeyCode::Enter => app.tunnel_form_submit(),
                            KeyCode::Esc => app.tunnel_form_cancel(),
                            _ => {}
                        }
                    } else if matches!(&app.tunnel_overlay, Some(TunnelOverlayState::List { .. })) {
                        // Mode liste de tunnels
                        match key.code {
                            KeyCode::Down | KeyCode::Char('j') => app.tunnel_overlay_next(),
                            KeyCode::Up | KeyCode::Char('k') => app.tunnel_overlay_previous(),
                            KeyCode::Enter => app.tunnel_overlay_toggle(),
                            KeyCode::Delete => app.tunnel_overlay_delete(),
                            KeyCode::Char('e') => app.open_tunnel_form_edit(),
                            KeyCode::Char('a') => app.open_tunnel_form_add(),
                            KeyCode::Char('q') | KeyCode::Esc => app.close_tunnel_overlay(),
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
                                app.connection_mode = ConnectionMode::Wallix;
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
                                                Some(Ok(_)) => app.set_status_message(
                                                    app.lang.copied.replacen("{}", &cmd, 1),
                                                ),
                                                Some(Err(e)) => app.set_status_message(
                                                    app.lang.clipboard_error.replacen(
                                                        "{}",
                                                        &e.to_string(),
                                                        1,
                                                    ),
                                                ),
                                                None => app.set_status_message(
                                                    app.lang.clipboard_unavailable.to_string(),
                                                ),
                                            }
                                        }
                                        Err(e) => app.set_status_message(
                                            app.lang.ssh_error.replacen("{}", &e.to_string(), 1),
                                        ),
                                    }
                                }
                            }
                            KeyCode::Char('/') => {
                                app.is_searching = true;
                            }
                            KeyCode::Char('r') => match app.reload() {
                                Ok(()) => {}
                                Err(e) => app.set_status_message(
                                    app.lang
                                        .config_reload_error
                                        .replacen("{}", &e.to_string(), 1),
                                ),
                            },
                            KeyCode::Char('f') => {
                                app.toggle_favorite();
                            }
                            KeyCode::Char('F') => {
                                app.toggle_favorites_view();
                            }
                            KeyCode::Char('C') => {
                                app.collapse_all();
                            }
                            KeyCode::Char('H') => {
                                app.sort_by_recent = !app.sort_by_recent;
                                app.items_dirty = true;
                                let msg = if app.sort_by_recent {
                                    app.lang.sort_recent_on
                                } else {
                                    app.lang.sort_recent_off
                                };
                                app.set_status_message(msg);
                            }
                            KeyCode::Char('x') => {
                                // Lance la saisie de commande ad-hoc
                                let items = app.get_visible_items();
                                if matches!(
                                    items.get(app.selected_index),
                                    Some(ConfigItem::Server(_))
                                ) {
                                    app.cmd_state = CmdState::Prompting(String::new());
                                }
                            }
                            KeyCode::Char('T') => {
                                // Ouvre l'overlay des tunnels SSH pour le serveur sélectionné
                                let items = app.get_visible_items();
                                if matches!(
                                    items.get(app.selected_index),
                                    Some(ConfigItem::Server(_))
                                ) {
                                    app.open_tunnel_overlay();
                                }
                            }
                            KeyCode::Char('s') => {
                                // Ouvre le transfert SCP pour le serveur sélectionné
                                let items = app.get_visible_items();
                                if matches!(
                                    items.get(app.selected_index),
                                    Some(ConfigItem::Server(_))
                                ) {
                                    app.open_scp_select_direction();
                                }
                            }
                            KeyCode::Esc
                                if matches!(
                                    app.cmd_state,
                                    CmdState::Done { .. } | CmdState::Error(_)
                                ) =>
                            {
                                app.reset_cmd();
                            }
                            KeyCode::Char('d') => {
                                let items = app.get_visible_items();
                                if let Some(ConfigItem::Server(server)) =
                                    items.get(app.selected_index)
                                {
                                    let server_clone = (**server).clone();
                                    let mode = app.connection_mode;
                                    if mode == ConnectionMode::Wallix {
                                        app.set_status_message(
                                            app.lang.probe_wallix_error.to_string(),
                                        );
                                    } else {
                                        let (tx, rx) = std::sync::mpsc::channel();
                                        app.probe_rx = Some(rx);
                                        app.probe_state = ProbeState::Running;
                                        std::thread::spawn(move || {
                                            let result = susshi::probe::probe(&server_clone, mode)
                                                .map_err(|e| e.to_string());
                                            let _ = tx.send(result);
                                        });
                                    }
                                }
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
                                        app.record_connection(&server);
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
                                        app.record_connection(&server);
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
