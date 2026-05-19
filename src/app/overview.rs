use super::*;
use crate::probe::{ProbeResult, probe};

impl App {
    /// Lance un probe parallèle sur tous les serveurs du groupe/env sélectionné.
    /// Remplit `self.overview` et `self.overview_rx`.
    pub fn open_overview(&mut self) {
        let items = self.get_visible_items();
        let selected = items.get(self.selected_index);

        let (group_name, servers) = match selected {
            Some(ConfigItem::Group(g, _)) => {
                let group = g.clone();
                let svs: Vec<ResolvedServer> = self
                    .resolved_servers
                    .iter()
                    .filter(|s| s.group_name == group)
                    .cloned()
                    .collect();
                (group, svs)
            }
            Some(ConfigItem::Environment(g, e, _)) => {
                let (group, env) = (g.clone(), e.clone());
                let svs: Vec<ResolvedServer> = self
                    .resolved_servers
                    .iter()
                    .filter(|s| s.group_name == group && s.env_name == env)
                    .cloned()
                    .collect();
                (format!("{group} / {env}"), svs)
            }
            _ => return,
        };

        if servers.is_empty() {
            return;
        }

        let entries: Vec<OverviewEntry> = servers
            .iter()
            .map(|s| OverviewEntry {
                server_name: s.name.clone(),
                host: s.host.clone(),
                status: OverviewStatus::Pending,
            })
            .collect();

        let (tx, rx) = mpsc::channel::<(usize, Result<ProbeResult, String>)>();

        for (idx, server) in servers.into_iter().enumerate() {
            let tx = tx.clone();
            let mode = self.connection_mode;
            std::thread::spawn(move || {
                let result = probe(&server, mode).map_err(|e| e.to_string());
                let _ = tx.send((idx, result));
            });
        }

        self.overview = Some(OverviewState {
            group_name,
            entries,
            scroll: 0,
        });
        self.overview_rx = Some(rx);
    }

    /// Ferme l'overlay overview.
    pub fn close_overview(&mut self) {
        self.overview = None;
        self.overview_rx = None;
    }

    /// Pompe les résultats de probe disponibles dans le canal overview.
    pub fn poll_overview(&mut self) {
        let results: Vec<(usize, Result<ProbeResult, String>)> = self
            .overview_rx
            .as_ref()
            .map(|rx| rx.try_iter().collect())
            .unwrap_or_default();

        if let Some(ov) = &mut self.overview {
            for (idx, result) in results {
                if let Some(entry) = ov.entries.get_mut(idx) {
                    entry.status = match result {
                        Ok(r) => OverviewStatus::Ok {
                            load: r.load.clone(),
                            ram_pct: r.ram_pct,
                            disk_pct: r.disk_pct,
                        },
                        Err(e) => OverviewStatus::Error(e),
                    };
                }
            }
        }
    }

    /// Fait défiler l'overview vers le bas.
    pub fn overview_scroll_down(&mut self) {
        if let Some(ov) = &mut self.overview {
            ov.scroll = ov.scroll.saturating_add(1);
        }
    }

    /// Fait défiler l'overview vers le haut.
    pub fn overview_scroll_up(&mut self) {
        if let Some(ov) = &mut self.overview {
            ov.scroll = ov.scroll.saturating_sub(1);
        }
    }
}
