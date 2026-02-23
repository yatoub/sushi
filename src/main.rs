use std::{io, time::Duration};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseEventKind, MouseButton},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal, layout::Rect};

use sushi::app::{App, ConfigItem};
use sushi::config::Config;
use sushi::ui;
use sushi::handlers::{handle_mouse_event, get_layout, is_in_rect};

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

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let config_path_cow = shellexpand::tilde("~/.sushi.yml");
    let config_path = std::path::Path::new(config_path_cow.as_ref());

    if !config_path.exists() {
        if let Err(e) = std::fs::write(config_path, DEFAULT_CONFIG) {
            disable_raw_mode()?;
            execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
            eprintln!("Failed to create default config: {}", e);
            return Err(e);
        }
    }

    let config_content = match std::fs::read_to_string(config_path) {
        Ok(c) => c,
        Err(e) => {
            disable_raw_mode()?;
            execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
            eprintln!("Failed to read config: {}", e);
            return Err(e);
        }
    };

    let mut config: Config = match serde_yaml::from_str(&config_content) {
        Ok(c) => c,
        Err(e) => {
            disable_raw_mode()?;
            execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
            eprintln!("Failed to parse YAML config: {}", e);
            return Err(io::Error::new(io::ErrorKind::InvalidData, e));
        }
    };

    config.sort();

    let mut app = App::new(config);

    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    match res {
        Ok(AppResult::Exit) => {},
        Ok(AppResult::Connect(server, mode)) => {
            if let Err(e) = sushi::ssh::client::connect(&server, mode) {
                 eprintln!("SSH Connection Error: {}", e);
            }
        },
        Err(err) => {
            eprintln!("Application Error: {:?}", err);
        }
    }

    Ok(())
}

pub enum AppResult {
    Exit,
    Connect(sushi::config::ResolvedServer, usize),
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> io::Result<AppResult> {
    let mut last_click_time = std::time::Instant::now();
    let mut last_click_pos = (0, 0);
    
    loop {
        let size_obj = terminal.size()?;
        let size = Rect::new(0, 0, size_obj.width, size_obj.height);

        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key) => {
                    if app.is_searching {
                        match key.code {
                            KeyCode::Enter | KeyCode::Esc => {
                                app.is_searching = false;
                            }
                            KeyCode::Char(c) => {
                                app.search_query.push(c);
                                app.selected_index = 0;
                                app.list_state.select(Some(0));
                            }
                            KeyCode::Backspace => {
                                app.search_query.pop();
                                app.selected_index = 0;
                                app.list_state.select(Some(0));
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
                            KeyCode::Char(' ') => {
                                app.toggle_expansion();
                            }
                            KeyCode::Enter => {
                                let items = app.get_visible_items();
                                if let Some(item) = items.get(app.selected_index) {
                                    match item {
                                        ConfigItem::Server(server) => {
                                            return Ok(AppResult::Connect(server.clone(), app.connection_mode));
                                        }
                                        _ => {
                                            app.toggle_expansion();
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Event::Mouse(mouse) => {
                    match mouse.kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                            let handled = handle_mouse_event(mouse, app, size)?;
                            
                            let now = std::time::Instant::now();
                            if handled && now.duration_since(last_click_time) < Duration::from_millis(400) && last_click_pos == (mouse.column, mouse.row) {
                                let layout = get_layout(size);
                                if is_in_rect(mouse.column, mouse.row, layout.list_area) {
                                     let items = app.get_visible_items();
                                     if let Some(ConfigItem::Server(server)) = items.get(app.selected_index) {
                                         return Ok(AppResult::Connect(server.clone(), app.connection_mode));
                                     }
                                }
                            }
                            last_click_time = now;
                            last_click_pos = (mouse.column, mouse.row);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }
}
