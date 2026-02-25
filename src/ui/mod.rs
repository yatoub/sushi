pub mod theme;
pub mod widgets;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap, Tabs},
    Frame,
};

use crate::app::{App, ConfigItem};
use crate::ui::theme::CATPPUCCIN_MOCHA;

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search bar
            Constraint::Length(3), // Connection Type Tabs
            Constraint::Min(0),    // Main content
            Constraint::Length(1), // Status bar
        ])
        .split(f.area());

    draw_search_bar(f, app, chunks[0]);
    draw_tabs(f, app, chunks[1]);
    
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(67), // Tree view (2/3)
            Constraint::Min(0),         // Details (1/3)
        ])
        .split(chunks[2]);

    draw_tree(f, app, main_chunks[0]);
    draw_details(f, app, main_chunks[1]);
    
    // draw_status_bar(f, app, chunks[3]); -> Correct index
    draw_status_bar(f, app, chunks[3]);
}

fn draw_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles = vec!["Direct", "Rebond", "Bastion"];
    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Mode de Connexion (Tab to switch) ")
                .border_style(Style::default().fg(CATPPUCCIN_MOCHA.border))
        )
        .select(app.connection_mode)
        .style(Style::default().fg(CATPPUCCIN_MOCHA.subtext0))
        .highlight_style(Style::default().bg(CATPPUCCIN_MOCHA.sky).fg(CATPPUCCIN_MOCHA.bg).add_modifier(Modifier::BOLD));
    f.render_widget(tabs, area);
}

fn draw_search_bar(f: &mut Frame, app: &App, area: Rect) {
    let visible_items = app.get_visible_items();
    let server_count = visible_items.iter().filter(|item| matches!(item, ConfigItem::Server(_))).count();
    let total_servers = app.resolved_servers.len();
    
    let (search_text, title) = if app.is_searching {
        let cursor = "│";
        let text = if app.search_query.is_empty() {
            format!("{}  (search by name or host, ESC to cancel)", cursor)
        } else {
            format!("{}{}", app.search_query, cursor)
        };
        
        let title_text = if app.search_query.is_empty() {
            format!(" 🔍 Search by name/host ({} servers) ", total_servers)
        } else if server_count == 0 {
            format!(" 🔍 No results for '{}' ", app.search_query)
        } else if server_count == total_servers {
            format!(" 🔍 All {} servers match ", server_count)
        } else {
            format!(" 🔍 {} / {} servers ", server_count, total_servers)
        };
        
        (text, title_text)
    } else {
        let text = if app.search_query.is_empty() {
            "Press / to search...".to_string()
        } else {
            let title_text = if server_count == total_servers {
                format!(" ✓ Showing all {} servers ", server_count)
            } else {
                format!(" ✓ {} / {} servers match '{}' ", server_count, total_servers, app.search_query)
            };
            return draw_search_with_results(f, area, &app.search_query, &title_text, server_count);
        };
        (text, " Search (press /) ".to_string())
    };

    let border_color = if app.is_searching {
        CATPPUCCIN_MOCHA.sapphire
    } else {
        CATPPUCCIN_MOCHA.border
    };
    
    let text_color = if app.is_searching {
        CATPPUCCIN_MOCHA.fg
    } else {
        CATPPUCCIN_MOCHA.subtext0
    };

    let search = Paragraph::new(search_text)
        .style(Style::default().fg(text_color))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color))
                .title(title)
        );
    f.render_widget(search, area);
}

fn draw_search_with_results(f: &mut Frame, area: Rect, query: &str, title: &str, count: usize) {
    let border_color = if count > 0 {
        CATPPUCCIN_MOCHA.green
    } else {
        CATPPUCCIN_MOCHA.red
    };
    
    let search = Paragraph::new(query)
        .style(Style::default().fg(CATPPUCCIN_MOCHA.fg))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color))
                .title(title)
        );
    f.render_widget(search, area);
}

fn draw_tree(f: &mut Frame, app: &mut App, area: Rect) {
    let visible_items = app.get_visible_items();
    let mut list_items = Vec::new();

    for item in visible_items.iter() {
        let content = match item {
            ConfigItem::Group(name) => {
                let id = format!("Group:{}", name);
                let icon = if app.expanded_items.contains(&id) || !app.search_query.is_empty() { "📂" } else { "📁" }; 
                Line::from(vec![
                    Span::styled(format!("{} {}", icon, name), Style::default().fg(CATPPUCCIN_MOCHA.group_header).add_modifier(Modifier::BOLD)),
                ])
            },
            ConfigItem::Environment(g, name) => {
                let id = format!("Env:{}:{}", g, name);
                let icon = if app.expanded_items.contains(&id) || !app.search_query.is_empty() { "🌩️" } else { "☁️" };
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(format!("{} {}", icon, name), Style::default().fg(CATPPUCCIN_MOCHA.env_header)),
                ])
            },
            ConfigItem::Server(server) => {
                let indent = if server.group_name.is_empty() {
                    "" // Root level
                } else if server.env_name.is_empty() {
                    "  " // Under group
                } else {
                    "    " // Under env
                };
                Line::from(vec![
                    Span::raw(indent),
                    Span::styled(format!("🖥️ {}", server.name), Style::default().fg(CATPPUCCIN_MOCHA.server_item)),
                ])
            },
        };

        list_items.push(ListItem::new(content));
    }

    let list = List::new(list_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Servers ")
                .border_style(Style::default().fg(CATPPUCCIN_MOCHA.border))
        )
        .highlight_style(Style::default().bg(CATPPUCCIN_MOCHA.selection_bg).fg(CATPPUCCIN_MOCHA.selection_fg).add_modifier(Modifier::BOLD))
        .highlight_symbol("▎ ");
    
    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn draw_details(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Details ")
        .border_style(Style::default().fg(CATPPUCCIN_MOCHA.border));

    let visible_items = app.get_visible_items();
    let text = if let Some(item) = visible_items.get(app.selected_index) {
        match item {
             ConfigItem::Server(server) => {
                let mut lines = vec![
                    Line::from(vec![
                        Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD).fg(CATPPUCCIN_MOCHA.fg)),
                        Span::raw(&server.name),
                    ]),
                    Line::from(vec![
                        Span::styled("Host: ", Style::default().add_modifier(Modifier::BOLD).fg(CATPPUCCIN_MOCHA.fg)),
                        Span::raw(&server.host),
                    ]),
                    Line::from(vec![
                        Span::styled("User: ", Style::default().add_modifier(Modifier::BOLD).fg(CATPPUCCIN_MOCHA.fg)),
                        Span::raw(&server.user),
                    ]),
                    Line::from(vec![
                        Span::styled("IdentityFile: ", Style::default().add_modifier(Modifier::BOLD).fg(CATPPUCCIN_MOCHA.fg)),
                        Span::raw(&server.ssh_key),
                    ]),
                    Line::from(vec![
                        Span::styled("SSH Options:", Style::default().add_modifier(Modifier::BOLD).fg(CATPPUCCIN_MOCHA.fg)),
                    ]),
                ];
                
                // Add each SSH option as a separate line with indentation
                for option in &server.ssh_options {
                    lines.push(Line::from(vec![
                        Span::raw("  • "),
                        Span::styled(option, Style::default().fg(CATPPUCCIN_MOCHA.subtext0)),
                    ]));
                }
                
                lines
             },
             ConfigItem::Group(name) => vec![Line::from(format!("Group: {}", name))],
             ConfigItem::Environment(g, e) => vec![Line::from(format!("Environment: {} / {}", g, e))],
        }
    } else {
        vec![Line::from("Select a server to view details.")]
    };

    let paragraph = Paragraph::new(text)
        .block(block)
        .style(Style::default().fg(CATPPUCCIN_MOCHA.fg))
        .wrap(Wrap { trim: true });
        
    f.render_widget(paragraph, area);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let text = if app.is_searching {
        "Search Mode: Type to filter | ESC: Cancel | Enter: Apply"
    } else if !app.search_query.is_empty() {
        "Navigate: ↑/↓ | Clear: ESC | New search: / | Enter: Connect | q: Quit"
    } else {
        "Navigate: ↑/↓ | Expand: Space/Enter | Search: / | Mode: Tab/1-3 | q: Quit"
    };
    let paragraph = Paragraph::new(text)
        .style(Style::default().bg(CATPPUCCIN_MOCHA.selection_bg).fg(CATPPUCCIN_MOCHA.fg));
    f.render_widget(paragraph, area);
}
