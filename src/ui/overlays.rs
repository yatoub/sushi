use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

use crate::app::{
    App, ConfigItem, ScpFormField, ScpState, TunnelFormField, TunnelOverlayState,
    WallixSelectorState,
};
use crate::i18n::Strings;
use crate::ssh::sftp::ScpDirection;
use crate::ssh::tunnel::TunnelStatus;
use crate::ui::theme::Theme;

/// Rectangle centre de taille fixe dans `area`.
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

/// Panneau d'erreur centre, affiche par-dessus l'interface normale.
pub(crate) fn draw_error_overlay(
    f: &mut Frame,
    msg: String,
    area: Rect,
    theme: &Theme,
    lang: &Strings,
) {
    let lines: Vec<&str> = msg.lines().collect();
    let inner_h = (lines.len() as u16).max(1);
    let popup_h = inner_h + 5;
    let popup_w = (msg.lines().map(|l| l.len()).max().unwrap_or(20) as u16 + 6)
        .clamp(40, area.width.saturating_sub(4));

    let popup_area = centered_rect(popup_w, popup_h, area);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(lang.error_title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.red))
        .style(Style::default().bg(theme.bg));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

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

pub(crate) fn draw_wallix_selector_overlay(f: &mut Frame, app: &mut App, area: Rect) {
    let Some(state) = &app.wallix_selector else {
        return;
    };

    let popup_area = centered_rect(86, 18, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(app.lang.wallix_selector_title)
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
                Paragraph::new(crate::i18n::fmt(
                    app.lang.wallix_selector_loading,
                    &[&server.name],
                ))
                .style(Style::default().fg(app.theme.fg)),
                chunks[0],
            );
            f.render_widget(
                Paragraph::new(app.lang.wallix_selector_loading_hint)
                    .style(Style::default().fg(app.theme.subtext0))
                    .wrap(Wrap { trim: true }),
                chunks[1],
            );
            f.render_widget(
                Paragraph::new(app.lang.wallix_selector_cancel_hint)
                    .style(Style::default().fg(app.theme.subtext0)),
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
                Paragraph::new(crate::i18n::fmt(
                    app.lang.wallix_selector_error,
                    &[&server.name],
                ))
                .style(
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
                Paragraph::new(app.lang.wallix_selector_close_hint)
                    .style(Style::default().fg(app.theme.subtext0)),
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
                Paragraph::new(crate::i18n::fmt(
                    app.lang.wallix_selector_choose,
                    &[&server.name, &server.host],
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
                Paragraph::new(app.lang.wallix_selector_list_hint)
                    .style(Style::default().fg(app.theme.subtext0)),
                chunks[2],
            );
        }
    }
}

/// Overlay flottant centre listant les tunnels SSH d'un serveur.
pub(crate) fn draw_tunnel_overlay(f: &mut Frame, app: &mut App, area: Rect) {
    match &app.tunnel_overlay {
        Some(TunnelOverlayState::Form(_)) => {
            draw_tunnel_form(f, app, area);
            return;
        }
        Some(TunnelOverlayState::List { .. }) => {}
        None => return,
    }

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

    let content_h = (n_tunnels as u16 + 1).max(1);
    let popup_h = (content_h + 5).min(area.height.saturating_sub(4));
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

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(2),
        ])
        .split(inner);

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

/// Formulaire d'edition / creation d'un tunnel SSH.
fn draw_tunnel_form(f: &mut Frame, app: &mut App, area: Rect) {
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

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

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

    if !form.error.is_empty() {
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  ✗ ", Style::default().fg(app.theme.red)),
                Span::styled(form.error.as_str(), Style::default().fg(app.theme.red)),
            ])),
            chunks[4],
        );
    }

    f.render_widget(
        Paragraph::new(app.lang.tunnel_form_hint).style(Style::default().fg(app.theme.subtext0)),
        chunks[6],
    );
}

/// Dispatch entre la selection de direction et le formulaire SCP.
pub(crate) fn draw_scp_overlay(f: &mut Frame, app: &mut App, area: Rect) {
    match &app.scp_state {
        ScpState::SelectingDirection => draw_scp_direction_select(f, app, area),
        ScpState::FillingForm { .. } => draw_scp_form(f, app, area),
        ScpState::Done { .. } | ScpState::Error(_) => draw_scp_result(f, app, area),
        _ => {}
    }
}

/// Petit overlay de selection de direction (Upload / Download).
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
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
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
            Span::styled(
                format!("{}  ", app.lang.scp_direction_upload_label),
                s_label,
            ),
            Span::styled(app.lang.scp_direction_upload, s_sub),
        ])),
        chunks[0],
    );
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  ↓  ", s_active),
            Span::styled(
                format!("{}  ", app.lang.scp_direction_download_label),
                s_label,
            ),
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

    let dir_label = if direction == ScpDirection::Upload {
        app.lang.scp_direction_upload_label
    } else {
        app.lang.scp_direction_download_label
    };
    let title = crate::i18n::fmt(app.lang.scp_form_title, &[dir_label, &server_name]);

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
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
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

    let inner_w = popup_area.width.saturating_sub(2) as usize;

    for (i, (label, value, field)) in fields.iter().enumerate() {
        let focused = *field == focus;
        let (label_fg, value_bg, cursor) = if focused {
            (app.theme.sapphire, app.theme.selection_bg, "█")
        } else {
            (app.theme.subtext0, app.theme.bg, "")
        };

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

/// Overlay de resultat SCP (Done / Error) — ferme avec Esc / Enter / q.
fn draw_scp_result(f: &mut Frame, app: &mut App, area: Rect) {
    let (icon, color, msg) = match &app.scp_state {
        ScpState::Done { direction, exit_ok } => {
            let dir_label = if *direction == ScpDirection::Upload {
                app.lang.scp_direction_upload_label
            } else {
                app.lang.scp_direction_download_label
            };
            let icon = if *exit_ok { "✔" } else { "✗" };
            let color = if *exit_ok {
                app.theme.green
            } else {
                app.theme.red
            };
            let msg = if *exit_ok {
                crate::i18n::fmt(app.lang.scp_result_success, &[dir_label])
            } else {
                crate::i18n::fmt(app.lang.scp_result_errors, &[dir_label])
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
