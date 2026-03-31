use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Tabs, Wrap},
};

use crate::app::{App, CmdState, ConfigItem, ScpState};
use crate::probe::{ProbeProfile, ProbeState};
use crate::ssh::sftp::ScpDirection;
use crate::ui::theme::Theme;

pub(crate) fn draw_connection_mode_area(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
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

pub(crate) fn draw_search_bar(f: &mut Frame, app: &mut App, area: Rect) {
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

pub(crate) fn draw_tree(f: &mut Frame, app: &mut App, area: Rect) {
    let visible_items = app.get_visible_items();
    let mut list_items = Vec::new();

    for item in &visible_items {
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
                    (true, true, _) => "",
                    (true, false, true) => "  ",
                    (true, false, false) => "    ",
                    (false, true, _) => "  ",
                    (false, false, true) => "    ",
                    (false, false, false) => "      ",
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

pub(crate) fn draw_details(f: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(app.lang.panel_details)
        .border_style(Style::default().fg(app.theme.border));

    let visible_items = app.get_visible_items();
    let text = if let Some(item) = visible_items.get(app.selected_index) {
        match item {
            ConfigItem::Server(server) => {
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

                if let ScpState::Running {
                    direction,
                    label,
                    progress,
                    started_at,
                    file_size,
                } = &app.scp_state
                {
                    let dir_label = if *direction == ScpDirection::Upload {
                        app.lang.scp_direction_upload_label
                    } else {
                        app.lang.scp_direction_download_label
                    };
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
                        crate::i18n::fmt(app.lang.scp_in_progress, &[dir_label]),
                        Style::default()
                            .fg(app.theme.sapphire)
                            .add_modifier(Modifier::BOLD),
                    )]));
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
                            format!(
                                "{} {}h{:02}m",
                                app.lang.scp_eta_label,
                                eta_secs / 3600,
                                (eta_secs % 3600) / 60
                            )
                        } else if eta_secs >= 60 {
                            format!(
                                "{} {}m{:02}s",
                                app.lang.scp_eta_label,
                                eta_secs / 60,
                                eta_secs % 60
                            )
                        } else {
                            format!("{} {}s", app.lang.scp_eta_label, eta_secs)
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
                    _ => match &app.probe_state {
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
                                    let style =
                                        if note.contains("error") || note.contains("<missing>") {
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
                    },
                }

                lines
            }
            ConfigItem::Namespace(label) => {
                vec![Line::from(vec![Span::styled(
                    crate::i18n::fmt(app.lang.details_namespace, &[label]),
                    Style::default()
                        .fg(app.theme.namespace_header)
                        .add_modifier(Modifier::BOLD),
                )])]
            }
            ConfigItem::Group(name, _ns) => {
                vec![Line::from(crate::i18n::fmt(
                    app.lang.details_group,
                    &[name],
                ))]
            }
            ConfigItem::Environment(g, e, _ns) => {
                vec![Line::from(crate::i18n::fmt(
                    app.lang.details_environment,
                    &[g, e],
                ))]
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

pub(crate) fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    if let CmdState::Prompting(buf) = &app.cmd_state {
        let prompt = format!("{} {}\u{2588}", app.lang.cmd_prompt, buf);
        let paragraph =
            Paragraph::new(prompt).style(Style::default().bg(app.theme.bg).fg(app.theme.yellow));
        f.render_widget(paragraph, area);
        return;
    }

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
        f.render_widget(
            Paragraph::new("").style(Style::default().bg(app.theme.selection_bg)),
            chunks[1],
        );
        return;
    }

    let theme = app.theme;
    let bg = theme.selection_bg;

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
                kh("↑↓", app.lang.hint_navigate),
                kh("Esc", app.lang.hint_validate_cancel),
                kh("Ctrl+U", app.lang.hint_clear),
            ]
            .into_iter()
            .flatten()
            .collect(),
            vec![Span::styled("", Style::default().bg(bg))],
        )
    } else if !app.search_query.is_empty() {
        (
            [
                kh("↑↓", app.lang.hint_navigate),
                kh("Enter", app.lang.hint_connect),
                kh("Esc", app.lang.hint_clear_filter),
                kh("/", app.lang.hint_new_search),
                kh("q", app.lang.hint_quit),
            ]
            .into_iter()
            .flatten()
            .collect(),
            vec![Span::styled("", Style::default().bg(bg))],
        )
    } else {
        let line1 = [
            kh("Enter", app.lang.hint_connect),
            kh("Space", app.lang.hint_expand),
            kh("↑↓ jk", app.lang.hint_navigate),
            kh("/", app.lang.hint_search),
            kh("Tab 1-3", app.lang.hint_mode),
            kh("T", app.lang.hint_tunnels),
            kh("q", app.lang.hint_quit),
        ]
        .into_iter()
        .flatten()
        .collect();
        let line2 = [
            kh("d", app.lang.hint_probe),
            kh("x", app.lang.hint_command),
            kh("s", app.lang.hint_scp),
            kh("y", app.lang.hint_copy_ssh),
            kh("f", app.lang.hint_favorite),
            kh("F", app.lang.hint_favorites_view),
            kh("r", app.lang.hint_reload),
            kh("H", app.lang.hint_recent_sort),
            kh("C", app.lang.hint_collapse),
            kh("v", app.lang.hint_verbose),
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
