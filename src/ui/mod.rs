pub mod theme;
pub mod widgets;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Tabs, Wrap},
};

use crate::app::{
    App, AppMode, CmdState, ConfigItem, ScpFormField, ScpState, TunnelFormField,
    TunnelOverlayState, WallixSelectorState,
};
use crate::i18n::Strings;
use crate::probe::{ProbeProfile, ProbeState};
use crate::ssh::tunnel::TunnelStatus;
use crate::ui::theme::Theme;

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

    draw_search_bar(f, app, chunks[0]);
    draw_connection_mode_area(f, app, chunks[1]);

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

    // Overlay tunnels — affiché au-dessus de l'interface normale
    if app.tunnel_overlay.is_some() {
        draw_tunnel_overlay(f, app, f.area());
    }

    // Overlay SCP — affiché par-dessus les tunnels si actif
    if !matches!(app.scp_state, ScpState::Idle | ScpState::Running { .. }) {
        draw_scp_overlay(f, app, f.area());
    }

    // Overlay Wallix — affiché au-dessus des panneaux normaux
    if app.wallix_selector.is_some() {
        draw_wallix_selector_overlay(f, app, f.area());
    }

    // Overlay erreur — rendu en dernier pour être au-dessus de tout
    if let AppMode::Error(msg) = &app.app_mode {
        draw_error_overlay(f, msg.clone(), f.area(), app.theme, app.lang);
    }
}

/// Rectangle centré de taille fixe dans `area`.
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

/// Panneau d'erreur centré, affiché par-dessus l'interface normale.
fn draw_error_overlay(f: &mut Frame, msg: String, area: Rect, theme: &Theme, lang: &Strings) {
    // Calcule la hauteur selon le nombre de lignes du message
    let lines: Vec<&str> = msg.lines().collect();
    let inner_h = (lines.len() as u16).max(1);
    // bordure (2) + titre (0 = inclus dans bordure) + contenu + hint (1) + marges (1)
    let popup_h = inner_h + 5;
    let popup_w = (msg.lines().map(|l| l.len()).max().unwrap_or(20) as u16 + 6)
        .clamp(40, area.width.saturating_sub(4));

    let popup_area = centered_rect(popup_w, popup_h, area);

    // Efface la zone du popup
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(lang.error_title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.red))
        .style(Style::default().bg(theme.bg));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    // Splits inner: message + hint
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let text: Vec<Line> = lines
        .iter()
        .map(|l| Line::from(Span::styled(*l, Style::default().fg(theme.fg))))
        .collect();
    let paragraph = Paragraph::new(text).wrap(Wrap { trim: false });
    f.render_widget(paragraph, chunks[0]);

    let hint = Paragraph::new(lang.error_dismiss).style(Style::default().fg(theme.subtext0));
    f.render_widget(hint, chunks[1]);
}

fn draw_wallix_selector_overlay(f: &mut Frame, app: &mut App, area: Rect) {
    let Some(state) = &app.wallix_selector else {
        return;
    };

    let popup_area = centered_rect(86, 18, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Wallix Selection ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.theme.sapphire))
        .style(Style::default().bg(app.theme.bg));
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    match state {
        WallixSelectorState::Loading { server } => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2),
                    Constraint::Min(1),
                    Constraint::Length(1),
                ])
                .split(inner);
            f.render_widget(
                Paragraph::new(format!("Loading Wallix entries for {}…", server.name))
                    .style(Style::default().fg(app.theme.fg)),
                chunks[0],
            );
            f.render_widget(
                Paragraph::new("Contacting the bastion and reading the interactive menu.")
                    .style(Style::default().fg(app.theme.subtext0))
                    .wrap(Wrap { trim: true }),
                chunks[1],
            );
            f.render_widget(
                Paragraph::new("Esc/q: cancel").style(Style::default().fg(app.theme.subtext0)),
                chunks[2],
            );
        }
        WallixSelectorState::Error { server, message } => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2),
                    Constraint::Min(1),
                    Constraint::Length(1),
                ])
                .split(inner);
            f.render_widget(
                Paragraph::new(format!("Wallix selector error for {}", server.name)).style(
                    Style::default()
                        .fg(app.theme.red)
                        .add_modifier(Modifier::BOLD),
                ),
                chunks[0],
            );
            f.render_widget(
                Paragraph::new(message.clone())
                    .style(Style::default().fg(app.theme.fg))
                    .wrap(Wrap { trim: true }),
                chunks[1],
            );
            f.render_widget(
                Paragraph::new("Enter/Esc/q: close").style(Style::default().fg(app.theme.subtext0)),
                chunks[2],
            );
        }
        WallixSelectorState::List {
            server,
            entries,
            selected,
        } => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2),
                    Constraint::Min(6),
                    Constraint::Length(2),
                ])
                .split(inner);

            f.render_widget(
                Paragraph::new(format!(
                    "Select the Wallix entry for {} ({})",
                    server.name, server.host
                ))
                .style(Style::default().fg(app.theme.fg)),
                chunks[0],
            );

            let items: Vec<ListItem> = entries
                .iter()
                .enumerate()
                .map(|(index, entry)| {
                    let is_selected = index == *selected;
                    let bg = if is_selected {
                        app.theme.selection_bg
                    } else {
                        app.theme.bg
                    };
                    let fg = if is_selected {
                        app.theme.selection_fg
                    } else {
                        app.theme.fg
                    };
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!("#{:<3} ", entry.id),
                            Style::default().fg(app.theme.sapphire).bg(bg),
                        ),
                        Span::styled(entry.target.clone(), Style::default().fg(fg).bg(bg)),
                        Span::styled("  →  ", Style::default().fg(app.theme.subtext0).bg(bg)),
                        Span::styled(entry.group.clone(), Style::default().fg(fg).bg(bg)),
                    ]))
                })
                .collect();
            f.render_widget(List::new(items), chunks[1]);

            f.render_widget(
                Paragraph::new("↑/↓: navigate | Enter: connect | Esc/q: cancel")
                    .style(Style::default().fg(app.theme.subtext0)),
                chunks[2],
            );
        }
    }
}

/// Overlay flottant centré listant les tunnels SSH d'un serveur.
fn draw_tunnel_overlay(f: &mut Frame, app: &mut App, area: Rect) {
    // Dispatch selon l'état de l'overlay.
    match &app.tunnel_overlay {
        Some(TunnelOverlayState::Form(_)) => {
            draw_tunnel_form(f, app, area);
            return;
        }
        Some(TunnelOverlayState::List { .. }) => {}
        None => return,
    }

    // ────────────── Vue liste ──────────────────────────────────────────────────
    // Récupère le serveur sélectionné via la liste visible (déjà en cache).
    let items = app.get_visible_items();
    let server = match items.get(app.selected_index) {
        Some(ConfigItem::Server(s)) => (**s).clone(),
        _ => return,
    };
    let overlay_selected = match &app.tunnel_overlay {
        Some(TunnelOverlayState::List { selected }) => *selected,
        _ => return,
    };

    let tunnels = app.effective_tunnels(&server);
    let server_key = App::server_key(&server);
    let n_tunnels = tunnels.len();

    // ── Dimensions ────────────────────────────────────────────────────────────
    // Lignes : chaque tunnel + la ligne "+" + 1 blanc + 2 lignes de hints = n+4
    // + 2 bordures = n+6 ; minimum 8 lignes (même sans tunnel).
    let content_h = (n_tunnels as u16 + 1).max(1); // tunnels + "+"
    let popup_h = (content_h + 5).min(area.height.saturating_sub(4)); // +5 = 2 bords + 1 blanc + 2 hints
    let popup_w: u16 = 64.min(area.width.saturating_sub(4));
    let popup_area = centered_rect(popup_w, popup_h, area);

    f.render_widget(Clear, popup_area);

    let title = format!(" Tunnels — {} ", server.name);
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.theme.sapphire))
        .style(Style::default().bg(app.theme.bg));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    // ── Layout intérieur ──────────────────────────────────────────────────────
    // Alloue : liste (reste) / blanc / hints (2 lignes)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // liste
            Constraint::Length(1), // ligne vide
            Constraint::Length(2), // hints
        ])
        .split(inner);

    // ── Lignes de tunnels ─────────────────────────────────────────────────────
    let mut list_items: Vec<ListItem> = Vec::new();

    for (i, et) in tunnels.iter().enumerate() {
        let handle = app
            .active_tunnels
            .get(&server_key)
            .and_then(|hs| hs.iter().find(|h| h.user_idx == i));

        let (icon, icon_color) = match handle {
            Some(h) if h.is_running() => ("✔", app.theme.green),
            Some(h) if matches!(h.status, TunnelStatus::Dead(_)) => ("✗", app.theme.red),
            _ => ("✖", app.theme.subtext0),
        };

        let label = if et.config.label.is_empty() {
            format!("{}:{}", et.config.remote_host, et.config.remote_port)
        } else {
            et.config.label.clone()
        };
        let route = format!(
            "localhost:{} → {}:{}",
            et.config.local_port, et.config.remote_host, et.config.remote_port
        );

        let is_sel = i == overlay_selected;
        let bg = if is_sel {
            app.theme.selection_bg
        } else {
            app.theme.bg
        };
        let fg = if is_sel {
            app.theme.selection_fg
        } else {
            app.theme.fg
        };
        let route_fg = if is_sel {
            app.theme.selection_fg
        } else {
            app.theme.subtext0
        };

        let line = Line::from(vec![
            Span::styled(format!("{} ", icon), Style::default().fg(icon_color).bg(bg)),
            Span::styled(format!("{:<20}", label), Style::default().fg(fg).bg(bg)),
            Span::styled(route, Style::default().fg(route_fg).bg(bg)),
        ]);
        list_items.push(ListItem::new(line));
    }

    // Ligne « + nouveau tunnel »
    let plus_sel = overlay_selected == n_tunnels;
    let plus_bg = if plus_sel {
        app.theme.selection_bg
    } else {
        app.theme.bg
    };
    let plus_fg = if plus_sel {
        app.theme.selection_fg
    } else {
        app.theme.green
    };
    list_items.push(ListItem::new(Line::from(Span::styled(
        app.lang.tunnel_overlay_new,
        Style::default().fg(plus_fg).bg(plus_bg),
    ))));

    f.render_widget(List::new(list_items), chunks[0]);

    // ── Hints ─────────────────────────────────────────────────────────────────
    let hint_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(chunks[2]);

    let s = Style::default().fg(app.theme.subtext0);
    f.render_widget(
        Paragraph::new(app.lang.tunnel_overlay_hints1).style(s),
        hint_chunks[0],
    );
    f.render_widget(
        Paragraph::new(app.lang.tunnel_overlay_hints2).style(s),
        hint_chunks[1],
    );
}

/// Formulaire d'édition / création d'un tunnel SSH.
fn draw_tunnel_form(f: &mut Frame, app: &mut App, area: Rect) {
    // Récupère le serveur et le formulaire.
    let items = app.get_visible_items();
    let server = match items.get(app.selected_index) {
        Some(ConfigItem::Server(s)) => (**s).clone(),
        _ => return,
    };
    let form = match &app.tunnel_overlay {
        Some(TunnelOverlayState::Form(form)) => form.clone(),
        _ => return,
    };

    let is_edit = form.editing_index.is_some();
    let title = if is_edit {
        crate::i18n::fmt(app.lang.tunnel_form_edit_title, &[&server.name])
    } else {
        crate::i18n::fmt(app.lang.tunnel_form_new_title, &[&server.name])
    };

    // 4 champs + 1 blank + 2 hints + 2 bordures + 1 erreur = 10 lignes min.
    let popup_h: u16 = 11;
    let popup_w: u16 = 62.min(area.width.saturating_sub(4));
    let popup_area = centered_rect(popup_w, popup_h, area);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.theme.sapphire))
        .style(Style::default().bg(app.theme.bg));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    // ── Layout : 4 champs + 1 erreur + 1 blank + 1 hint ──────────────────────
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Label
            Constraint::Length(1), // Port local
            Constraint::Length(1), // Hôte distant
            Constraint::Length(1), // Port distant
            Constraint::Length(1), // Erreur
            Constraint::Length(1), // blanc
            Constraint::Length(1), // hint
        ])
        .split(inner);

    // ── Champs ────────────────────────────────────────────────────────────────
    let fields: &[(&str, &str, TunnelFormField)] = &[
        (
            app.lang.tunnel_form_field_label,
            &form.label,
            TunnelFormField::Label,
        ),
        (
            app.lang.tunnel_form_field_local_port,
            &form.local_port,
            TunnelFormField::LocalPort,
        ),
        (
            app.lang.tunnel_form_field_remote_host,
            &form.remote_host,
            TunnelFormField::RemoteHost,
        ),
        (
            app.lang.tunnel_form_field_remote_port,
            &form.remote_port,
            TunnelFormField::RemotePort,
        ),
    ];

    for (i, (label, value, field)) in fields.iter().enumerate() {
        let focused = *field == form.focus;
        let (label_fg, value_bg, cursor) = if focused {
            (app.theme.sapphire, app.theme.selection_bg, "█")
        } else {
            (app.theme.subtext0, app.theme.bg, "")
        };

        let line = Line::from(vec![
            Span::styled(*label, Style::default().fg(label_fg)),
            Span::styled(
                format!("{}{}", value, cursor),
                Style::default().fg(app.theme.fg).bg(value_bg),
            ),
        ]);
        f.render_widget(Paragraph::new(line), chunks[i]);
    }

    // ── Erreur ────────────────────────────────────────────────────────────────
    if !form.error.is_empty() {
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  ✗ ", Style::default().fg(app.theme.red)),
                Span::styled(form.error.as_str(), Style::default().fg(app.theme.red)),
            ])),
            chunks[4],
        );
    }

    // ── Hint ─────────────────────────────────────────────────────────────────
    f.render_widget(
        Paragraph::new(app.lang.tunnel_form_hint).style(Style::default().fg(app.theme.subtext0)),
        chunks[6],
    );
}

// ─── Overlay SCP ─────────────────────────────────────────────────────────────

/// Dispatch entre la sélection de direction et le formulaire SCP.
fn draw_scp_overlay(f: &mut Frame, app: &mut App, area: Rect) {
    match &app.scp_state {
        ScpState::SelectingDirection => draw_scp_direction_select(f, app, area),
        ScpState::FillingForm { .. } => draw_scp_form(f, app, area),
        ScpState::Done { .. } | ScpState::Error(_) => draw_scp_result(f, app, area),
        _ => {}
    }
}

/// Petit overlay de sélection de direction (Upload / Download).
fn draw_scp_direction_select(f: &mut Frame, app: &mut App, area: Rect) {
    let items = app.get_visible_items();
    let server = match items.get(app.selected_index) {
        Some(ConfigItem::Server(s)) => s.name.clone(),
        _ => return,
    };

    let popup_h: u16 = 7;
    let popup_w: u16 = 38.min(area.width.saturating_sub(4));
    let popup_area = centered_rect(popup_w, popup_h, area);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(crate::i18n::fmt(app.lang.scp_direction_title, &[&server]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.theme.sapphire))
        .style(Style::default().bg(app.theme.bg));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Upload
            Constraint::Length(1), // Download
            Constraint::Length(1), // vide
            Constraint::Length(1), // hint
        ])
        .split(inner);

    let s_active = Style::default()
        .fg(app.theme.fg)
        .add_modifier(Modifier::BOLD);
    let s_label = Style::default().fg(app.theme.sky);
    let s_sub = Style::default().fg(app.theme.subtext0);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  ↑  ", s_active),
            Span::styled("Upload  ", s_label),
            Span::styled(app.lang.scp_direction_upload, s_sub),
        ])),
        chunks[0],
    );
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  ↓  ", s_active),
            Span::styled("Download  ", s_label),
            Span::styled(app.lang.scp_direction_download, s_sub),
        ])),
        chunks[1],
    );
    f.render_widget(
        Paragraph::new(app.lang.scp_direction_hint).style(s_sub),
        chunks[3],
    );
}

/// Formulaire SCP (deux champs : Local et Distant).
fn draw_scp_form(f: &mut Frame, app: &mut App, area: Rect) {
    let (direction, local, remote, focus, error) = match &app.scp_state {
        ScpState::FillingForm {
            direction,
            local,
            remote,
            focus,
            error,
        } => (
            direction.clone(),
            local.clone(),
            remote.clone(),
            focus.clone(),
            error.clone(),
        ),
        _ => return,
    };
    let items = app.get_visible_items();
    let server_name = match items.get(app.selected_index) {
        Some(ConfigItem::Server(s)) => s.name.clone(),
        _ => return,
    };

    let dir_label = direction.label();
    let title = format!(" SCP {} — {} ", dir_label, server_name);

    // 2 champs + 1 erreur + 1 vide + 1 hint + 2 bordures
    let popup_h: u16 = 8;
    let popup_w: u16 = 64.min(area.width.saturating_sub(4));
    let popup_area = centered_rect(popup_w, popup_h, area);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.theme.sapphire))
        .style(Style::default().bg(app.theme.bg));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Local
            Constraint::Length(1), // Distant
            Constraint::Length(1), // Erreur
            Constraint::Length(1), // vide
            Constraint::Length(1), // hint
        ])
        .split(inner);

    let fields: &[(&str, &str, ScpFormField)] = &[
        (app.lang.scp_form_field_local, &local, ScpFormField::Local),
        (
            app.lang.scp_form_field_remote,
            &remote,
            ScpFormField::Remote,
        ),
    ];

    // Largeur disponible pour la valeur = intérieur popup − largeur label − curseur (1)
    let inner_w = popup_area.width.saturating_sub(2) as usize; // -2 bordures

    for (i, (label, value, field)) in fields.iter().enumerate() {
        let focused = *field == focus;
        let (label_fg, value_bg, cursor) = if focused {
            (app.theme.sapphire, app.theme.selection_bg, "█")
        } else {
            (app.theme.subtext0, app.theme.bg, "")
        };

        // Tronque le chemin par le début pour garder le nom de fichier visible.
        let label_w = label.chars().count();
        let cursor_w = if focused { 1 } else { 0 };
        let max_value_w = inner_w.saturating_sub(label_w + cursor_w);
        let display_value: String = if value.len() > max_value_w && max_value_w > 1 {
            format!(
                "\u{2026}{}",
                &value[value.len().saturating_sub(max_value_w.saturating_sub(1))..]
            )
        } else {
            value.to_string()
        };

        let line = Line::from(vec![
            Span::styled(*label, Style::default().fg(label_fg)),
            Span::styled(
                format!("{}{}", display_value, cursor),
                Style::default().fg(app.theme.fg).bg(value_bg),
            ),
        ]);
        f.render_widget(Paragraph::new(line), chunks[i]);
    }

    if !error.is_empty() {
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  ✗ ", Style::default().fg(app.theme.red)),
                Span::styled(error, Style::default().fg(app.theme.red)),
            ])),
            chunks[2],
        );
    }

    f.render_widget(
        Paragraph::new(app.lang.scp_form_hint).style(Style::default().fg(app.theme.subtext0)),
        chunks[4],
    );
}

/// Overlay de résultat SCP (Done / Error) — ferme avec Esc / Enter / q.
fn draw_scp_result(f: &mut Frame, app: &mut App, area: Rect) {
    let (icon, color, msg) = match &app.scp_state {
        ScpState::Done { direction, exit_ok } => {
            let icon = if *exit_ok { "✔" } else { "✗" };
            let color = if *exit_ok {
                app.theme.green
            } else {
                app.theme.red
            };
            let msg = if *exit_ok {
                crate::i18n::fmt(app.lang.scp_result_success, &[direction.label()])
            } else {
                crate::i18n::fmt(app.lang.scp_result_errors, &[direction.label()])
            };
            (icon, color, msg)
        }
        ScpState::Error(e) => (
            "✗",
            app.theme.red,
            crate::i18n::fmt(app.lang.scp_result_fail, &[e]),
        ),
        _ => return,
    };

    let popup_h: u16 = 5;
    let popup_w: u16 = (msg.len() as u16 + 8).clamp(36, area.width.saturating_sub(4));
    let popup_area = centered_rect(popup_w, popup_h, area);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(app.lang.scp_result_title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(color))
        .style(Style::default().bg(app.theme.bg));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!("  {} ", icon), Style::default().fg(color)),
            Span::styled(msg, Style::default().fg(app.theme.fg)),
        ])),
        chunks[0],
    );
    f.render_widget(
        Paragraph::new(app.lang.scp_result_hint).style(Style::default().fg(app.theme.subtext0)),
        chunks[2],
    );
}

fn draw_connection_mode_area(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70), // Tabs
            Constraint::Percentage(30), // Verbose option
        ])
        .split(area);

    draw_tabs(f, app, chunks[0]);
    draw_verbose_toggle(f, app, chunks[1]);
}

fn draw_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles = vec![app.lang.tab_direct, app.lang.tab_jump, app.lang.tab_wallix];
    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(app.lang.tab_title)
                .border_style(Style::default().fg(app.theme.border)),
        )
        .select(app.connection_mode.index())
        .style(Style::default().fg(app.theme.subtext0))
        .highlight_style(
            Style::default()
                .bg(app.theme.sky)
                .fg(app.theme.bg)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(tabs, area);
}

fn draw_verbose_toggle(f: &mut Frame, app: &App, area: Rect) {
    let checkbox = if app.verbose_mode { "☑" } else { "☐" };
    let text = format!("{} {}", checkbox, app.lang.verbose_label);

    let style = if app.verbose_mode {
        Style::default()
            .fg(app.theme.green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(app.theme.subtext0)
    };

    let verbose = Paragraph::new(text).style(style).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(app.lang.verbose_title)
            .border_style(Style::default().fg(app.theme.border)),
    );
    f.render_widget(verbose, area);
}

fn draw_search_bar(f: &mut Frame, app: &mut App, area: Rect) {
    let visible_items = app.get_visible_items();
    let server_count = visible_items
        .iter()
        .filter(|item| matches!(item, ConfigItem::Server(_)))
        .count();
    let total_servers = app.resolved_servers.len();

    let (search_text, title) = if app.is_searching {
        let cursor = "│";
        let text = if app.search_query.is_empty() {
            format!("{}  {}", cursor, app.lang.search_placeholder)
        } else {
            format!("{}{}", app.search_query, cursor)
        };

        let title_text = if app.search_query.is_empty() {
            app.lang
                .search_title_active
                .replacen("{}", &total_servers.to_string(), 1)
        } else if server_count == 0 {
            app.lang
                .search_no_results
                .replacen("{}", &app.search_query, 1)
        } else if server_count == total_servers {
            app.lang
                .search_all_match
                .replacen("{}", &server_count.to_string(), 1)
        } else {
            app.lang
                .search_partial
                .replacen("{}", &server_count.to_string(), 1)
                .replacen("{}", &total_servers.to_string(), 1)
        };

        (text, title_text)
    } else {
        let text = if app.search_query.is_empty() {
            app.lang.search_idle_hint.to_string()
        } else {
            let title_text = if server_count == total_servers {
                app.lang
                    .search_result_all
                    .replacen("{}", &server_count.to_string(), 1)
            } else {
                app.lang
                    .search_result_partial
                    .replacen("{}", &server_count.to_string(), 1)
                    .replacen("{}", &total_servers.to_string(), 1)
                    .replacen("{}", &app.search_query, 1)
            };
            return draw_search_with_results(
                f,
                area,
                &app.search_query,
                &title_text,
                server_count,
                app.theme,
            );
        };
        (text, app.lang.search_title_idle.to_string())
    };

    let border_color = if app.is_searching {
        app.theme.sapphire
    } else {
        app.theme.border
    };

    let text_color = if app.is_searching {
        app.theme.fg
    } else {
        app.theme.subtext0
    };

    let search = Paragraph::new(search_text)
        .style(Style::default().fg(text_color))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color))
                .title(title),
        );
    f.render_widget(search, area);
}

fn draw_search_with_results(
    f: &mut Frame,
    area: Rect,
    query: &str,
    title: &str,
    count: usize,
    theme: &Theme,
) {
    let border_color = if count > 0 { theme.green } else { theme.red };

    let search = Paragraph::new(query)
        .style(Style::default().fg(theme.fg))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color))
                .title(title),
        );
    f.render_widget(search, area);
}

fn draw_tree(f: &mut Frame, app: &mut App, area: Rect) {
    let visible_items = app.get_visible_items();
    let mut list_items = Vec::new();

    for item in visible_items.iter() {
        let content = match item {
            ConfigItem::Namespace(label) => {
                let id = format!("NS:{}", label);
                let icon = if app.expanded_items.contains(&id) || !app.search_query.is_empty() {
                    "📦"
                } else {
                    "📫"
                };
                Line::from(vec![Span::styled(
                    format!("{} {}", icon, label),
                    Style::default()
                        .fg(app.theme.namespace_header)
                        .add_modifier(Modifier::BOLD),
                )])
            }
            ConfigItem::Group(name, ns) => {
                let id = if ns.is_empty() {
                    format!("Group:{}", name)
                } else {
                    format!("NS:{}:Group:{}", ns, name)
                };
                let icon = if app.expanded_items.contains(&id) || !app.search_query.is_empty() {
                    "📂"
                } else {
                    "📁"
                };
                let indent = if ns.is_empty() { "" } else { "  " };
                Line::from(vec![
                    Span::raw(indent),
                    Span::styled(
                        format!("{} {}", icon, name),
                        Style::default()
                            .fg(app.theme.group_header)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
            }
            ConfigItem::Environment(g, name, ns) => {
                let id = if ns.is_empty() {
                    format!("Env:{}:{}", g, name)
                } else {
                    format!("NS:{}:Env:{}:{}", ns, g, name)
                };
                let icon = if app.expanded_items.contains(&id) || !app.search_query.is_empty() {
                    "🌩️"
                } else {
                    "☁️"
                };
                let indent = if ns.is_empty() { "  " } else { "    " };
                Line::from(vec![
                    Span::raw(indent),
                    Span::styled(
                        format!("{} {}", icon, name),
                        Style::default().fg(app.theme.env_header),
                    ),
                ])
            }
            ConfigItem::Server(server) => {
                let indent = match (
                    server.namespace.is_empty(),
                    server.group_name.is_empty(),
                    server.env_name.is_empty(),
                ) {
                    (true, true, _) => "",             // racine
                    (true, false, true) => "  ",       // groupe racine
                    (true, false, false) => "    ",    // groupe + env racine
                    (false, true, _) => "  ",          // namespace, pas de groupe
                    (false, false, true) => "    ",    // namespace + groupe
                    (false, false, false) => "      ", // namespace + groupe + env
                };
                let server_key = crate::app::App::server_key(server);
                let is_fav = app.favorites.contains(&server_key);
                let icon = if is_fav { "⭐" } else { "🖥️" };
                let name_style = if is_fav {
                    Style::default()
                        .fg(app.theme.yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(app.theme.server_item)
                };
                Line::from(vec![
                    Span::raw(indent),
                    Span::styled(format!("{} {}", icon, server.name), name_style),
                ])
            }
        };

        list_items.push(ListItem::new(content));
    }

    let list = List::new(list_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(if app.favorites_only {
                    app.lang.favorites_title
                } else {
                    app.lang.panel_servers
                })
                .border_style(Style::default().fg(app.theme.border)),
        )
        .highlight_style(
            Style::default()
                .bg(app.theme.selection_bg)
                .fg(app.theme.selection_fg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▎ ");

    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn draw_details(f: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(app.lang.panel_details)
        .border_style(Style::default().fg(app.theme.border));

    let visible_items = app.get_visible_items();
    let text = if let Some(item) = visible_items.get(app.selected_index) {
        match item {
            ConfigItem::Server(server) => {
                // Port : jaune si différent de 22
                let port_style = if server.port != 22 {
                    Style::default()
                        .fg(app.theme.yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(app.theme.subtext0)
                };

                let mut lines = vec![
                    Line::from(vec![
                        Span::styled(
                            app.lang.label_name,
                            Style::default()
                                .add_modifier(Modifier::BOLD)
                                .fg(app.theme.fg),
                        ),
                        Span::raw(&server.name),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            app.lang.label_host,
                            Style::default()
                                .add_modifier(Modifier::BOLD)
                                .fg(app.theme.fg),
                        ),
                        Span::raw(&server.host),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            app.lang.label_port,
                            Style::default()
                                .add_modifier(Modifier::BOLD)
                                .fg(app.theme.fg),
                        ),
                        Span::styled(server.port.to_string(), port_style),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            app.lang.label_user,
                            Style::default()
                                .add_modifier(Modifier::BOLD)
                                .fg(app.theme.fg),
                        ),
                        Span::raw(&server.user),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            app.lang.label_mode,
                            Style::default()
                                .add_modifier(Modifier::BOLD)
                                .fg(app.theme.fg),
                        ),
                        Span::styled(
                            server.default_mode.to_string(),
                            Style::default().fg(app.theme.sapphire),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            app.lang.label_key,
                            Style::default()
                                .add_modifier(Modifier::BOLD)
                                .fg(app.theme.fg),
                        ),
                        Span::raw(&server.ssh_key),
                    ]),
                ];

                // Jump host(s) (mode Rebond) — "user@host:port" ou "u1@h1,u2@h2"
                if let Some(jump) = &server.jump_host {
                    lines.push(Line::from(vec![
                        Span::styled(
                            app.lang.label_jump,
                            Style::default()
                                .add_modifier(Modifier::BOLD)
                                .fg(app.theme.fg),
                        ),
                        Span::styled(jump.clone(), Style::default().fg(app.theme.sky)),
                    ]));
                }

                // Wallix host
                if let Some(bhost) = &server.bastion_host {
                    let bastion_display = match &server.bastion_user {
                        Some(u) => format!("{}@{}", u, bhost),
                        None => bhost.clone(),
                    };
                    lines.push(Line::from(vec![
                        Span::styled(
                            app.lang.label_wallix,
                            Style::default()
                                .add_modifier(Modifier::BOLD)
                                .fg(app.theme.fg),
                        ),
                        Span::raw(" "),
                        Span::styled(bastion_display, Style::default().fg(app.theme.sky)),
                    ]));
                }

                if !server.ssh_options.is_empty() {
                    lines.push(Line::from(vec![Span::styled(
                        app.lang.label_options,
                        Style::default()
                            .add_modifier(Modifier::BOLD)
                            .fg(app.theme.fg),
                    )]));
                    for option in &server.ssh_options {
                        lines.push(Line::from(vec![
                            Span::raw("  \u{2022} "),
                            Span::styled(option, Style::default().fg(app.theme.subtext0)),
                        ]));
                    }
                }

                // ── Dernière connexion ───────────────────────────────────────────
                let last_seen_str = if let Some(ts) = app.last_seen_for(server) {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    let elapsed = now.saturating_sub(ts);
                    if elapsed < 60 {
                        app.lang.last_seen_just_now.to_string()
                    } else {
                        let minutes = elapsed / 60;
                        let hours = minutes / 60;
                        let days = hours / 24;
                        let ago_str = if days >= 1 {
                            format!("{} j", days)
                        } else if hours >= 1 {
                            format!("{} h", hours)
                        } else {
                            format!("{} min", minutes)
                        };
                        app.lang.last_seen_ago.replacen("{}", &ago_str, 1)
                    }
                } else {
                    app.lang.last_seen_never.to_string()
                };
                lines.push(Line::from(vec![
                    Span::styled(
                        app.lang.last_seen_label,
                        Style::default()
                            .add_modifier(Modifier::BOLD)
                            .fg(app.theme.fg),
                    ),
                    Span::styled(last_seen_str, Style::default().fg(app.theme.subtext0)),
                ]));

                // ── Badge tunnels ─────────────────────────────────────────────
                {
                    let n_cfg = app.effective_tunnels(server).len();
                    let n_run = app.active_tunnel_count(server);
                    if n_cfg > 0 {
                        let (badge_fg, badge_text) = if n_run > 0 {
                            (
                                app.theme.green,
                                crate::i18n::fmt(
                                    app.lang.tunnel_badge_active,
                                    &[
                                        &n_run.to_string(),
                                        if n_run > 1 { "s" } else { "" },
                                        &n_cfg.to_string(),
                                        if n_cfg > 1 { "s" } else { "" },
                                    ],
                                ),
                            )
                        } else {
                            (
                                app.theme.subtext0,
                                crate::i18n::fmt(
                                    app.lang.tunnel_badge_none,
                                    &[&n_cfg.to_string(), if n_cfg > 1 { "s" } else { "" }],
                                ),
                            )
                        };
                        lines.push(Line::from(vec![
                            Span::styled(
                                app.lang.tunnel_badge_label,
                                Style::default()
                                    .add_modifier(Modifier::BOLD)
                                    .fg(app.theme.fg),
                            ),
                            Span::styled(badge_text, Style::default().fg(badge_fg)),
                        ]));
                    }
                }

                // ── Progression SCP (si transfert en cours) ─────────────────────
                if let ScpState::Running {
                    direction,
                    label,
                    progress,
                    started_at,
                    file_size,
                } = &app.scp_state
                {
                    const BAR_W: usize = 20;
                    let filled = (*progress as usize * BAR_W / 100).min(BAR_W);
                    let bar_color = if *progress < 60 {
                        app.theme.green
                    } else if *progress < 85 {
                        app.theme.yellow
                    } else {
                        app.theme.sapphire
                    };
                    lines.push(Line::from(vec![Span::styled(
                        crate::i18n::fmt(app.lang.scp_in_progress, &[direction.label()]),
                        Style::default()
                            .fg(app.theme.sapphire)
                            .add_modifier(Modifier::BOLD),
                    )]));
                    // Tronque le chemin par le début pour garder le nom de
                    // fichier visible sur les paths longs (ex : …/long/path/file.txt).
                    let max_w = area.width.saturating_sub(4) as usize;
                    let display_label = if label.len() > max_w && max_w > 1 {
                        format!(
                            "\u{2026}{}",
                            &label[label.len().saturating_sub(max_w.saturating_sub(1))..]
                        )
                    } else {
                        label.clone()
                    };
                    lines.push(Line::from(vec![
                        Span::styled("  ", Style::default()),
                        Span::styled(display_label, Style::default().fg(app.theme.fg)),
                    ]));

                    // ── Calcul vitesse + ETA ──────────────────────────────────────
                    let elapsed_secs = started_at.elapsed().as_secs_f64();
                    let transferred = if *file_size > 0 {
                        (*progress as u64 * file_size) / 100
                    } else {
                        0
                    };
                    let speed_str = if elapsed_secs >= 1.0 && transferred > 0 {
                        let bps = transferred as f64 / elapsed_secs;
                        if bps >= 1_000_000.0 {
                            format!("{:.1} MB/s", bps / 1_000_000.0)
                        } else if bps >= 1_000.0 {
                            format!("{:.0} KB/s", bps / 1_000.0)
                        } else {
                            format!("{:.0} B/s", bps)
                        }
                    } else {
                        "-".to_string()
                    };
                    let eta_str = if elapsed_secs >= 1.0
                        && *progress > 0
                        && *progress < 100
                        && transferred > 0
                    {
                        let remaining = file_size.saturating_sub(transferred);
                        let bps = transferred as f64 / elapsed_secs;
                        let eta_secs = (remaining as f64 / bps) as u64;
                        if eta_secs >= 3600 {
                            format!("ETA {}h{:02}m", eta_secs / 3600, (eta_secs % 3600) / 60)
                        } else if eta_secs >= 60 {
                            format!("ETA {}m{:02}s", eta_secs / 60, eta_secs % 60)
                        } else {
                            format!("ETA {}s", eta_secs)
                        }
                    } else {
                        String::new()
                    };

                    lines.push(Line::from(vec![
                        Span::styled("  [", Style::default().fg(app.theme.subtext0)),
                        Span::styled("█".repeat(filled), Style::default().fg(bar_color)),
                        Span::styled(
                            "░".repeat(BAR_W - filled),
                            Style::default().fg(app.theme.subtext0),
                        ),
                        Span::styled(
                            format!("]{:>4}%", progress),
                            Style::default().fg(app.theme.fg),
                        ),
                        Span::styled(
                            format!("  {}", speed_str),
                            Style::default().fg(app.theme.sky),
                        ),
                        Span::styled(
                            if eta_str.is_empty() {
                                String::new()
                            } else {
                                format!("  {}", eta_str)
                            },
                            Style::default().fg(app.theme.subtext0),
                        ),
                    ]));
                }

                // ── Bloc commande ad-hoc (si actif) ou diagnostic SSH ─────────────
                lines.push(Line::from(""));
                match &app.cmd_state {
                    CmdState::Running(cmd) => {
                        let ms = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.subsec_millis())
                            .unwrap_or(0);
                        let frames = [
                            '\u{280b}', '\u{2819}', '\u{2839}', '\u{2838}', '\u{283c}', '\u{2834}',
                            '\u{2826}', '\u{2827}', '\u{2807}', '\u{280f}',
                        ];
                        let spinner = frames[(ms / 100) as usize % frames.len()];
                        lines.push(Line::from(vec![
                            Span::styled(
                                format!("  {} ", spinner),
                                Style::default().fg(app.theme.sapphire),
                            ),
                            Span::styled(
                                app.lang.cmd_running.replacen("{}", cmd, 1),
                                Style::default().fg(app.theme.subtext0),
                            ),
                        ]));
                    }
                    CmdState::Done {
                        cmd,
                        output,
                        exit_ok,
                    } => {
                        let status_icon = if *exit_ok { "✔" } else { "✗" };
                        let status_color = if *exit_ok {
                            app.theme.green
                        } else {
                            app.theme.red
                        };
                        lines.push(Line::from(vec![
                            Span::styled(
                                format!("{} ", status_icon),
                                Style::default().fg(status_color),
                            ),
                            Span::styled(
                                format!("$ {}", cmd),
                                Style::default().fg(app.theme.sapphire),
                            ),
                        ]));
                        for line in output.lines().take(20) {
                            lines.push(Line::from(vec![Span::styled(
                                format!("  {}", line),
                                Style::default().fg(app.theme.fg),
                            )]));
                        }
                    }
                    CmdState::Error(e) => {
                        lines.push(Line::from(vec![
                            Span::styled("✗ ", Style::default().fg(app.theme.red)),
                            Span::raw(e.as_str()),
                        ]));
                    }
                    _ => {
                        // Pas de commande : on affiche le probe SSH habituel
                        match &app.probe_state {
                            ProbeState::Idle => {
                                lines.push(Line::from(vec![Span::styled(
                                    app.lang.probe_hint,
                                    Style::default().fg(app.theme.subtext0),
                                )]));
                            }
                            ProbeState::Running => {
                                let ms = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .map(|d| d.subsec_millis())
                                    .unwrap_or(0);
                                let frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
                                let spinner = frames[(ms / 100) as usize % frames.len()];
                                lines.push(Line::from(vec![
                                    Span::styled(
                                        format!("  {} ", spinner),
                                        Style::default().fg(app.theme.sapphire),
                                    ),
                                    Span::styled(
                                        app.lang.probe_running,
                                        Style::default().fg(app.theme.subtext0),
                                    ),
                                ]));
                            }
                            ProbeState::Done(r) => {
                                let theme = app.theme;
                                lines.push(Line::from(vec![Span::styled(
                                    app.lang.probe_section,
                                    Style::default().fg(theme.border),
                                )]));
                                if r.profile == ProbeProfile::Wallix {
                                    for note in &r.notes {
                                        let style = if note.contains("error")
                                            || note.contains("<missing>")
                                        {
                                            Style::default().fg(theme.red)
                                        } else if note.contains("skipped") {
                                            Style::default().fg(theme.yellow)
                                        } else if note.contains("ok") {
                                            Style::default().fg(theme.green)
                                        } else {
                                            Style::default().fg(theme.fg)
                                        };
                                        lines.push(Line::from(vec![Span::styled(
                                            format!("  {}", note),
                                            style,
                                        )]));
                                    }
                                } else {
                                    lines.push(Line::from(vec![
                                        Span::styled(
                                            app.lang.probe_kernel,
                                            Style::default()
                                                .add_modifier(Modifier::BOLD)
                                                .fg(theme.fg),
                                        ),
                                        Span::raw(r.kernel.clone()),
                                    ]));
                                    lines.push(Line::from(vec![
                                        Span::styled(
                                            app.lang.probe_os,
                                            Style::default()
                                                .add_modifier(Modifier::BOLD)
                                                .fg(theme.fg),
                                        ),
                                        Span::raw(r.os_name.clone()),
                                    ]));
                                    lines.push(Line::from(vec![
                                        Span::styled(
                                            app.lang.probe_cpu,
                                            Style::default()
                                                .add_modifier(Modifier::BOLD)
                                                .fg(theme.fg),
                                        ),
                                        Span::raw(r.cpu_model.clone()),
                                    ]));
                                    lines.push(Line::from(vec![
                                        Span::styled(
                                            app.lang.probe_cpu_cores,
                                            Style::default()
                                                .add_modifier(Modifier::BOLD)
                                                .fg(theme.fg),
                                        ),
                                        Span::raw(r.cpu_cores.to_string()),
                                    ]));
                                    lines.push(Line::from(vec![
                                        Span::styled(
                                            app.lang.probe_load,
                                            Style::default()
                                                .add_modifier(Modifier::BOLD)
                                                .fg(theme.fg),
                                        ),
                                        Span::raw(r.load.clone()),
                                    ]));
                                    lines.push(probe_bar(
                                        app.lang.probe_ram,
                                        r.ram_pct,
                                        r.ram_total_gb,
                                        theme,
                                    ));
                                    lines.push(probe_bar(
                                        app.lang.probe_disk,
                                        r.disk_pct,
                                        r.disk_total_gb,
                                        theme,
                                    ));
                                    for fs_entry in &r.extra_fs {
                                        match &fs_entry.usage {
                                            Some(usage) => {
                                                let label = app.lang.probe_disk_extra.replacen(
                                                    "{}",
                                                    &fs_entry.mountpoint,
                                                    1,
                                                );
                                                lines.push(probe_bar(
                                                    &label,
                                                    usage.pct,
                                                    usage.total_gb,
                                                    theme,
                                                ));
                                            }
                                            None => {
                                                lines.push(Line::from(vec![Span::styled(
                                                    app.lang.probe_fs_absent.replacen(
                                                        "{}",
                                                        &fs_entry.mountpoint,
                                                        1,
                                                    ),
                                                    Style::default()
                                                        .fg(theme.yellow)
                                                        .add_modifier(Modifier::BOLD),
                                                )]));
                                            }
                                        }
                                    }
                                }
                            }
                            ProbeState::Error(msg) => {
                                lines.push(Line::from(vec![Span::styled(
                                    app.lang.probe_section,
                                    Style::default().fg(app.theme.border),
                                )]));
                                lines.push(Line::from(vec![
                                    Span::styled("\u{2717}  ", Style::default().fg(app.theme.red)),
                                    Span::raw(msg.clone()),
                                ]));
                            }
                        }
                    } // fin du bloc _ => match &probe_state
                }

                lines
            }
            ConfigItem::Namespace(label) => {
                vec![Line::from(vec![Span::styled(
                    format!("📦 Namespace : {}", label),
                    Style::default()
                        .fg(app.theme.namespace_header)
                        .add_modifier(Modifier::BOLD),
                )])]
            }
            ConfigItem::Group(name, _ns) => vec![Line::from(format!("Group: {}", name))],
            ConfigItem::Environment(g, e, _ns) => {
                vec![Line::from(format!("Environment: {} / {}", g, e))]
            }
        }
    } else {
        vec![Line::from(app.lang.details_placeholder)]
    };

    let paragraph = Paragraph::new(text)
        .block(block)
        .style(Style::default().fg(app.theme.fg))
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

/// Construit une ligne de barre de progression textuelle pour le bloc System.
/// `label` : libellé (ex. `"RAM"`, `"Disk /"`), `pct` : 0–100, `total_gb` : capacité.
/// Couleur de la barre : vert < 60 %, jaune 60–85 %, rouge > 85 %.
fn probe_bar(label: &str, pct: u8, total_gb: f32, theme: &Theme) -> Line<'static> {
    const BAR_WIDTH: usize = 12;
    let filled = (pct as usize * BAR_WIDTH / 100).min(BAR_WIDTH);
    let bar_color = if pct < 60 {
        theme.green
    } else if pct < 85 {
        theme.yellow
    } else {
        theme.red
    };
    Line::from(vec![
        Span::styled(
            format!("{:<9}", label),
            Style::default().add_modifier(Modifier::BOLD).fg(theme.fg),
        ),
        Span::styled("█".repeat(filled), Style::default().fg(bar_color)),
        Span::styled(
            "░".repeat(BAR_WIDTH - filled),
            Style::default().fg(theme.subtext0),
        ),
        Span::styled(
            format!("  {:>3}%  {:.1} GB", pct, total_gb),
            Style::default().fg(theme.fg),
        ),
    ])
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    // Mode saisie de commande ad-hoc
    if let CmdState::Prompting(buf) = &app.cmd_state {
        let prompt = format!("{} {}\u{2588}", app.lang.cmd_prompt, buf);
        let paragraph =
            Paragraph::new(prompt).style(Style::default().bg(app.theme.bg).fg(app.theme.yellow));
        f.render_widget(paragraph, area);
        return;
    }

    // Affiche le message temporaire (clipboard, erreur…) si présent
    if let Some((msg, _)) = &app.status_message {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(area);
        f.render_widget(
            Paragraph::new(msg.as_str()).style(
                Style::default()
                    .bg(app.theme.selection_bg)
                    .fg(app.theme.green),
            ),
            chunks[0],
        );
        // Ligne 2 vide mais colorée
        f.render_widget(
            Paragraph::new("").style(Style::default().bg(app.theme.selection_bg)),
            chunks[1],
        );
        return;
    }

    let theme = app.theme;
    let bg = theme.selection_bg;

    // Construit une paire [touche] description sous forme de Spans.
    let kh = |key: &str, desc: &str| -> Vec<Span<'static>> {
        vec![
            Span::styled(
                format!("[{}]", key),
                Style::default()
                    .fg(theme.sky)
                    .bg(bg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {}  ", desc),
                Style::default().fg(theme.subtext0).bg(bg),
            ),
        ]
    };

    let (line1_spans, line2_spans): (Vec<Span>, Vec<Span>) = if app.is_searching {
        (
            [
                kh("↑↓", "naviguer"),
                kh("Esc", "valider / annuler"),
                kh("Ctrl+U", "effacer"),
            ]
            .into_iter()
            .flatten()
            .collect(),
            vec![Span::styled("", Style::default().bg(bg))],
        )
    } else if !app.search_query.is_empty() {
        (
            [
                kh("↑↓", "naviguer"),
                kh("Enter", "connexion"),
                kh("Esc", "effacer filtre"),
                kh("/", "nouvelle recherche"),
                kh("q", "quitter"),
            ]
            .into_iter()
            .flatten()
            .collect(),
            vec![Span::styled("", Style::default().bg(bg))],
        )
    } else {
        // Ligne 1 — actions principales
        let line1 = [
            kh("Enter", "connexion"),
            kh("Space", "expand"),
            kh("↑↓ jk", "naviguer"),
            kh("/", "recherche"),
            kh("Tab 1-3", "mode"),
            kh("T", "tunnels"),
            kh("q", "quitter"),
        ]
        .into_iter()
        .flatten()
        .collect();
        // Ligne 2 — actions secondaires
        let line2 = [
            kh("d", "probe"),
            kh("x", "commande"),
            kh("s", "SCP"),
            kh("y", "copier SSH"),
            kh("f", "favori"),
            kh("F", "★ vue favoris"),
            kh("r", "recharger"),
            kh("H", "tri récent"),
            kh("C", "replier"),
            kh("v", "verbose"),
        ]
        .into_iter()
        .flatten()
        .collect();
        (line1, line2)
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    f.render_widget(
        Paragraph::new(Line::from(line1_spans)).style(Style::default().bg(bg)),
        chunks[0],
    );
    f.render_widget(
        Paragraph::new(Line::from(line2_spans)).style(Style::default().bg(bg)),
        chunks[1],
    );
}
