mod overlays;
mod panels;
pub mod theme;
pub mod widgets;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
};

use crate::app::{App, AppMode, ScpState};

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search bar
            Constraint::Length(3), // Connection Type Tabs
            Constraint::Min(0),    // Main content
            Constraint::Length(2), // Status bar (2 lignes de hints)
        ])
        .split(f.area());

    panels::draw_search_bar(f, app, chunks[0]);
    panels::draw_connection_mode_area(f, app, chunks[1]);

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(67), // Tree view (2/3)
            Constraint::Min(0),         // Details (1/3)
        ])
        .split(chunks[2]);

    panels::draw_tree(f, app, main_chunks[0]);
    panels::draw_details(f, app, main_chunks[1]);

    panels::draw_status_bar(f, app, chunks[3]);

    // Overlay tunnels — affiché au-dessus de l'interface normale
    if app.tunnel_overlay.is_some() {
        overlays::draw_tunnel_overlay(f, app, f.area());
    }

    // Overlay SCP — affiché par-dessus les tunnels si actif
    if !matches!(app.scp_state, ScpState::Idle | ScpState::Running { .. }) {
        overlays::draw_scp_overlay(f, app, f.area());
    }

    // Overlay Wallix — affiché au-dessus des panneaux normaux
    if app.wallix_selector.is_some() {
        overlays::draw_wallix_selector_overlay(f, app, f.area());
    }

    // Overlay erreur — rendu en dernier pour être au-dessus de tout
    if let AppMode::Error(msg) = &app.app_mode {
        overlays::draw_error_overlay(f, msg.clone(), f.area(), app.theme, app.lang);
    }
}
