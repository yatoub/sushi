use std::{io, thread, time::Duration};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

mod app;
mod config;
mod ui;
mod ssh;

use app::{App, ConfigItem};
use config::Config;
use ssh::client::SshClient;

const DEFAULT_CONFIG: &str = r#"
defaults:
  user: "admin"
  ssh_key: "~/.ssh/id_rsa"

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
"#;

fn main() -> io::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Load or create configuration
    let config_path_cow = shellexpand::tilde("~/.sushi.yml");
    let config_path = std::path::Path::new(config_path_cow.as_ref());

    if !config_path.exists() {
        if let Err(e) = std::fs::write(config_path, DEFAULT_CONFIG) {
            // Restore terminal before panicking/printing error
            disable_raw_mode()?;
            execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
            eprintln!("Failed to create default config at {:?}: {}", config_path, e);
            return Err(e);
        }
    }

    let config_content = match std::fs::read_to_string(config_path) {
        Ok(c) => c,
        Err(e) => {
            disable_raw_mode()?;
            execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
            eprintln!("Failed to read config at {:?}: {}", config_path, e);
            return Err(e);
        }
    };

    let config: Config = match serde_yaml::from_str(&config_content) {
        Ok(c) => c,
        Err(e) => {
            disable_raw_mode()?;
            execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
            eprintln!("Failed to parse YAML config at {:?}: {}", config_path, e);
            return Err(io::Error::new(io::ErrorKind::InvalidData, e));
        }
    };

    let mut app = App::new(config);

    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if app.is_searching {
                    match key.code {
                        KeyCode::Enter | KeyCode::Esc => {
                            app.is_searching = false;
                        }
                        KeyCode::Char(c) => {
                            app.search_query.push(c);
                            app.selected_index = 0;
                        }
                        KeyCode::Backspace => {
                            app.search_query.pop();
                            app.selected_index = 0;
                        }
                        _ => {}
                    }
                } else {
                    match key.code {
                        KeyCode::Char('q') => {
                            return Ok(());
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            app.next();
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            app.previous();
                        }
                        KeyCode::Char('/') => {
                            app.is_searching = true;
                        }
                        KeyCode::Enter => {
                            let items = app.get_visible_items();
                            if let Some(ConfigItem::Server(server)) = items.get(app.selected_index) {
                                // Connect!
                                
                                // Suspend TUI
                                disable_raw_mode()?;
                                execute!(
                                    terminal.backend_mut(),
                                    LeaveAlternateScreen,
                                    DisableMouseCapture
                                )?;
                                terminal.show_cursor()?;

                                let client = SshClient::new(server.clone());
                                if let Err(e) = client.connect() {
                                    eprintln!("SSH Error: {}", e);
                                    // Give user time to read error
                                    thread::sleep(Duration::from_secs(3));
                                }

                                // Restore TUI
                                enable_raw_mode()?;
                                execute!(
                                    terminal.backend_mut(),
                                    EnterAlternateScreen,
                                    EnableMouseCapture
                                )?;
                                terminal.hide_cursor()?;
                                terminal.clear()?;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
