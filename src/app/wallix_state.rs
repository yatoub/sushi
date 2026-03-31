use super::*;

fn wallix_matching_error(message: &str) -> bool {
    message.contains("No menu entry found with target")
        || message.contains("No menu entry found for matching targets")
        || message.contains("No menu entry found for target")
        || message.contains("Multiple menu entries")
        || message.contains("wallix.group is not configured")
}

fn group_suffix_matches(entry_group: &str, configured_group: &str) -> bool {
    entry_group == configured_group || entry_group.ends_with(&format!("_{configured_group}"))
}

fn score_entry(
    server: &ResolvedServer,
    expected_targets: &[String],
    entry: &WallixMenuEntry,
) -> u8 {
    let mut score = 0;

    if expected_targets
        .iter()
        .any(|target| target == &entry.target)
    {
        score += 10;
    }

    if let Some(group) = server
        .wallix_group
        .as_deref()
        .map(str::trim)
        .filter(|g| !g.is_empty())
    {
        if entry.group == group {
            score += 4;
        } else if group_suffix_matches(&entry.group, group) {
            score += 2;
        }
    }

    score
}

pub(super) fn targeted_wallix_entries(
    server: &ResolvedServer,
    entries: &[WallixMenuEntry],
) -> Vec<WallixMenuEntry> {
    let expected_targets = build_expected_targets(server);

    let mut targeted: Vec<WallixMenuEntry> = entries
        .iter()
        .filter(|entry| {
            expected_targets
                .iter()
                .any(|target| target == &entry.target)
        })
        .cloned()
        .collect();

    if !targeted.is_empty() {
        targeted
            .sort_by_key(|entry| std::cmp::Reverse(score_entry(server, &expected_targets, entry)));
        return targeted;
    }

    let mut all_entries = entries.to_vec();
    all_entries
        .sort_by_key(|entry| std::cmp::Reverse(score_entry(server, &expected_targets, entry)));
    all_entries
}

impl App {
    pub fn should_open_wallix_selector(&self, server: &ResolvedServer) -> bool {
        let _ = server;
        if self.connection_mode != ConnectionMode::Wallix {
            return false;
        }

        #[cfg(unix)]
        {
            true
        }

        #[cfg(not(unix))]
        {
            false
        }
    }

    pub fn open_wallix_selector(&mut self, server: ResolvedServer, verbose: bool) {
        self.wallix_pending_connection = None;
        let (tx, rx) = mpsc::channel();
        self.wallix_selector = Some(WallixSelectorState::Loading {
            server: Box::new(server.clone()),
        });
        self.wallix_selector_rx = Some(rx);

        std::thread::spawn(move || {
            let result = crate::ssh::client::fetch_wallix_menu_entries(&server, verbose)
                .map_err(|e| e.to_string());
            let _ = tx.send((server, result));
        });
    }

    pub fn poll_wallix_selector(&mut self) {
        let done = if let Some(rx) = &self.wallix_selector_rx {
            rx.try_recv().ok()
        } else {
            None
        };

        if let Some((server, result)) = done {
            match result {
                Ok(entries) => {
                    let server_key = Self::server_key(&server);

                    if let Some(cached_id) = self.wallix_selection_cache.get(&server_key)
                        && entries.iter().any(|entry| &entry.id == cached_id)
                    {
                        self.wallix_pending_connection = Some((server, cached_id.clone()));
                        self.wallix_selector = None;
                        self.wallix_selector_rx = None;
                        return;
                    }

                    if server.wallix_auto_select {
                        match select_id_for_server(&entries, &server) {
                            Ok(selected_id) => {
                                self.wallix_selection_cache
                                    .insert(server_key, selected_id.clone());
                                self.wallix_pending_connection = Some((server, selected_id));
                                self.wallix_selector = None;
                            }
                            Err(err) => {
                                let message = err.to_string();
                                if wallix_matching_error(&message) {
                                    self.wallix_selector = Some(WallixSelectorState::List {
                                        server: Box::new(server.clone()),
                                        entries: targeted_wallix_entries(&server, &entries),
                                        selected: 0,
                                    });
                                } else {
                                    self.wallix_selector = Some(WallixSelectorState::Error {
                                        server: Box::new(server),
                                        message,
                                    });
                                }
                            }
                        }
                    } else {
                        self.wallix_selector = Some(WallixSelectorState::List {
                            server: Box::new(server.clone()),
                            entries: targeted_wallix_entries(&server, &entries),
                            selected: 0,
                        });
                    }
                }
                Err(message) => {
                    self.wallix_selector = Some(WallixSelectorState::Error {
                        server: Box::new(server),
                        message,
                    });
                }
            }
            self.wallix_selector_rx = None;
        }
    }

    pub fn close_wallix_selector(&mut self) {
        self.wallix_selector = None;
        self.wallix_selector_rx = None;
    }

    pub fn take_pending_wallix_connection(&mut self) -> Option<(ResolvedServer, String)> {
        self.wallix_pending_connection.take()
    }

    pub fn remember_wallix_selection(&mut self, server: &ResolvedServer, selected_id: &str) {
        self.wallix_selection_cache
            .insert(Self::server_key(server), selected_id.to_string());
    }

    pub fn wallix_selector_next(&mut self) {
        if let Some(WallixSelectorState::List {
            entries, selected, ..
        }) = &mut self.wallix_selector
            && !entries.is_empty()
        {
            *selected = (*selected + 1).min(entries.len().saturating_sub(1));
        }
    }

    pub fn wallix_selector_previous(&mut self) {
        if let Some(WallixSelectorState::List { selected, .. }) = &mut self.wallix_selector {
            *selected = selected.saturating_sub(1);
        }
    }

    pub fn wallix_selector_selected_id(&self) -> Option<(ResolvedServer, String)> {
        match &self.wallix_selector {
            Some(WallixSelectorState::List {
                server,
                entries,
                selected,
            }) => entries
                .get(*selected)
                .map(|entry| ((**server).clone(), entry.id.clone())),
            _ => None,
        }
    }
}
