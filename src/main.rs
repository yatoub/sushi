use std::{io, time::Duration};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
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
// use ssh::client::SshClient; removed


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

    // Restore terminal before doing anything else (especially for SSH handover)
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    match res {
        Ok(AppResult::Exit) => {
            // Normal exit
        },
        Ok(AppResult::Connect(server, mode)) => {
            // Handover to SSH
            // Since we restored the terminal, we can now exec the ssh command
            if let Err(e) = crate::ssh::client::connect(&server, mode) {
                 eprintln!("SSH Connection Error: {}", e);
                 // In case exec fails, we print error. 
                 // If exec succeeds, we never reach here.
            }
        },
        Err(err) => {
            eprintln!("Application Error: {:?}", err);
        }
    }

    Ok(())
}

// Enum for App Result to separate UI logic from business logic (connection)
pub enum AppResult {
    Exit,
    Connect(crate::config::ResolvedServer, usize), // Server, Connection Mode
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> io::Result<AppResult> {
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
                            return Ok(AppResult::Exit);
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            app.next();
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            app.previous();
                        }
                        KeyCode::Tab => {
                            app.connection_mode = (app.connection_mode + 1) % 3;
                        }
                        KeyCode::Char('/') => {
                            app.is_searching = true;
                        }
                        KeyCode::Enter => {
                            let items = app.get_visible_items();
                            if let Some(ConfigItem::Server(server)) = items.get(app.selected_index) {
                                return Ok(AppResult::Connect(server.clone(), app.connection_mode));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
