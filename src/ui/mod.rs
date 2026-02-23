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
    let titles = vec!["Direct [1]", "Rebond [2]", "Bastion [3]"];
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
        .highlight_style(Style::default().fg(CATPPUCCIN_MOCHA.yellow).add_modifier(Modifier::BOLD));
    f.render_widget(tabs, area);
}

fn draw_search_bar(f: &mut Frame, app: &App, area: Rect) {
    let search_text = if app.search_query.is_empty() {
        "Type / to search..."
    } else {
        &app.search_query
    };

    let search = Paragraph::new(search_text)
        .style(Style::default().fg(CATPPUCCIN_MOCHA.search_text))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(CATPPUCCIN_MOCHA.border))
                .title(" Search ")
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
             ConfigItem::Server(server) => vec![
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
                    Span::styled("SSH Path: ", Style::default().add_modifier(Modifier::BOLD).fg(CATPPUCCIN_MOCHA.fg)),
                    Span::raw(&server.ssh_key),
                ]),
                Line::from(vec![
                    Span::styled("SSH Options: ", Style::default().add_modifier(Modifier::BOLD).fg(CATPPUCCIN_MOCHA.fg)),
                    Span::raw(server.ssh_options.join(" ")),
                ]),
             ],
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

fn draw_status_bar(f: &mut Frame, _app: &App, area: Rect) {
    let text = "Navigate: ↑/↓ | Search: / | Quit: q";
    let paragraph = Paragraph::new(text)
        .style(Style::default().bg(CATPPUCCIN_MOCHA.selection_bg).fg(CATPPUCCIN_MOCHA.fg));
    f.render_widget(paragraph, area);
}
