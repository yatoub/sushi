pub mod theme;
pub mod widgets;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, ConfigItem};
use crate::ui::theme::CATPPUCCIN_MOCHA;

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search bar
            Constraint::Min(0),    // Main content
            Constraint::Length(1), // Status bar
        ])
        .split(f.area());

    draw_search_bar(f, app, chunks[0]);
    
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30), // Tree view
            Constraint::Percentage(70), // Details
        ])
        .split(chunks[1]);

    draw_tree(f, app, main_chunks[0]);
    draw_details(f, app, main_chunks[1]);
    
    draw_status_bar(f, app, chunks[2]);
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
                .border_style(Style::default().fg(CATPPUCCIN_MOCHA.border))
                .title(" Search ")
        );
    f.render_widget(search, area);
}

fn draw_tree(f: &mut Frame, app: &mut App, area: Rect) {
    let visible_items = app.get_visible_items();
    let mut list_items = Vec::new();

    for (i, item) in visible_items.iter().enumerate() {
        let is_selected = i == app.selected_index;
        
        let (content, mut style) = match item {
            ConfigItem::Group(name) => (
                Line::from(vec![
                    Span::styled(format!("📁 {}", name), Style::default().fg(CATPPUCCIN_MOCHA.group_header).add_modifier(Modifier::BOLD)),
                ]),
                Style::default()
            ),
            ConfigItem::Environment(_, name) => (
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(format!("☁️ {}", name), Style::default().fg(CATPPUCCIN_MOCHA.env_header)),
                ]),
                Style::default()
            ),
            ConfigItem::Server(server) => (
                Line::from(vec![
                    Span::raw("    "),
                    Span::styled(format!("🖥️ {}", server.name), Style::default().fg(CATPPUCCIN_MOCHA.server_item)),
                ]),
                Style::default()
            ),
        };

        if is_selected {
            style = style.bg(CATPPUCCIN_MOCHA.selection_bg).fg(CATPPUCCIN_MOCHA.selection_fg).add_modifier(Modifier::BOLD);
        }
        
        // Apply selection style to the content if it's a server, or just the whole line?
        // Let's modify the line spans if selected? Or apply style to ListItem.
        // ListItem style serves as base.
        
        // Hack: Rebuild line with selection style if needed for specific parts, or rely on ListItem style
        // For simple highlighting, ListItem style is enough but might color unrelated padding.
        
        let mut styled_content = content;
        if is_selected {
            for span in &mut styled_content.spans {
                 span.style = span.style.patch(style);
            }
        }

        list_items.push(ListItem::new(styled_content).style(style));
    }

    let list = List::new(list_items)
        .block(Block::default().borders(Borders::ALL).title(" Servers ").border_style(Style::default().fg(CATPPUCCIN_MOCHA.border)));
    
    f.render_widget(list, area);
}

fn draw_details(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
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
