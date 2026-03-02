pub mod theme;
pub mod widgets;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Tabs, Wrap},
};

use crate::app::{App, AppMode, CmdState, ConfigItem};
use crate::i18n::Strings;
use crate::probe::ProbeState;
use crate::ui::theme::Theme;

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
    let titles = vec![app.lang.tab_direct, app.lang.tab_jump, app.lang.tab_bastion];
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

                // Bastion host
                if let Some(bhost) = &server.bastion_host {
                    let bastion_display = match &server.bastion_user {
                        Some(u) => format!("{}@{}", u, bhost),
                        None => bhost.clone(),
                    };
                    lines.push(Line::from(vec![
                        Span::styled(
                            app.lang.label_bastion,
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
                                lines.push(Line::from(vec![
                                    Span::styled(
                                        app.lang.probe_kernel,
                                        Style::default().add_modifier(Modifier::BOLD).fg(theme.fg),
                                    ),
                                    Span::raw(r.kernel.clone()),
                                ]));
                                lines.push(Line::from(vec![
                                    Span::styled(
                                        app.lang.probe_os,
                                        Style::default().add_modifier(Modifier::BOLD).fg(theme.fg),
                                    ),
                                    Span::raw(r.os_name.clone()),
                                ]));
                                lines.push(Line::from(vec![
                                    Span::styled(
                                        app.lang.probe_cpu,
                                        Style::default().add_modifier(Modifier::BOLD).fg(theme.fg),
                                    ),
                                    Span::raw(r.cpu_model.clone()),
                                ]));
                                lines.push(Line::from(vec![
                                    Span::styled(
                                        app.lang.probe_cpu_cores,
                                        Style::default().add_modifier(Modifier::BOLD).fg(theme.fg),
                                    ),
                                    Span::raw(r.cpu_cores.to_string()),
                                ]));
                                lines.push(Line::from(vec![
                                    Span::styled(
                                        app.lang.probe_load,
                                        Style::default().add_modifier(Modifier::BOLD).fg(theme.fg),
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
        let paragraph = Paragraph::new(msg.as_str()).style(
            Style::default()
                .bg(app.theme.selection_bg)
                .fg(app.theme.green),
        );
        f.render_widget(paragraph, area);
        return;
    }

    let text = if app.is_searching {
        app.lang.status_searching
    } else if !app.search_query.is_empty() {
        app.lang.status_search_active
    } else {
        app.lang.status_normal
    };
    let paragraph =
        Paragraph::new(text).style(Style::default().bg(app.theme.selection_bg).fg(app.theme.fg));
    f.render_widget(paragraph, area);
}
