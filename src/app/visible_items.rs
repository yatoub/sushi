use super::*;

impl App {
    pub fn get_visible_items(&mut self) -> Vec<ConfigItem> {
        if self.items_dirty {
            self.cached_items = self.build_visible_items();
            self.items_dirty = false;
        }
        self.cached_items.clone()
    }

    fn build_visible_items(&self) -> Vec<ConfigItem> {
        // Mode "tri par recent" : liste plate de tous les serveurs tries par last_seen
        if self.sort_by_recent {
            return self.build_recent_items();
        }

        let mut items = Vec::new();
        let searching = !self.search_query.is_empty();

        for entry in &self.config.groups {
            match entry {
                ConfigEntry::Namespace(ns) => {
                    // En mode favorites_only, on n'affiche le namespace que s'il a des favoris
                    if self.favorites_only
                        && !self.namespace_has_visible_servers(&ns.entries, &ns.label)
                    {
                        continue;
                    }
                    items.push(ConfigItem::Namespace(ns.label.clone()));
                    let ns_id = format!("NS:{}", ns.label);
                    let ns_expanded = self.expanded_items.contains(&ns_id) || searching;
                    if ns_expanded {
                        self.push_entries(&ns.entries, &ns.label, &mut items);
                    }
                }
                ConfigEntry::Server(s_conf) => {
                    if let Some(resolved) = self.resolved_servers.iter().find(|rs| {
                        rs.name == s_conf.name
                            && rs.group_name.is_empty()
                            && rs.env_name.is_empty()
                            && rs.namespace.is_empty()
                    }) {
                        if searching
                            && !self.matches_search(&resolved.name, &resolved.host, &resolved.tags)
                        {
                            continue;
                        }
                        if self.favorites_only
                            && !self.favorites.contains(&Self::server_key(resolved))
                        {
                            continue;
                        }
                        items.push(ConfigItem::Server(Box::new(resolved.clone())));
                    }
                }
                ConfigEntry::Group(group) => {
                    if self.favorites_only && !self.group_has_visible_servers(group, "") {
                        continue;
                    }
                    items.push(ConfigItem::Group(group.name.clone(), String::new()));
                    let group_id = format!("Group:{}", group.name);
                    let group_expanded = self.expanded_items.contains(&group_id) || searching;

                    if group_expanded {
                        self.push_group_children(group, "", &mut items);
                    }
                }
            }
        }
        items
    }

    /// Construit une liste plate triee par derniere connexion (mode H actif).
    fn build_recent_items(&self) -> Vec<ConfigItem> {
        let mut servers: Vec<ConfigItem> = self
            .resolved_servers
            .iter()
            .filter(|rs| {
                if self.favorites_only && !self.favorites.contains(&Self::server_key(rs)) {
                    return false;
                }
                if !self.search_query.is_empty()
                    && !self.matches_search(&rs.name, &rs.host, &rs.tags)
                {
                    return false;
                }
                true
            })
            .cloned()
            .map(|s| ConfigItem::Server(Box::new(s)))
            .collect();

        servers.sort_by(|a, b| {
            let ts_a = if let ConfigItem::Server(s) = a {
                self.last_seen
                    .get(&Self::server_key(s))
                    .copied()
                    .unwrap_or(0)
            } else {
                0
            };
            let ts_b = if let ConfigItem::Server(s) = b {
                self.last_seen
                    .get(&Self::server_key(s))
                    .copied()
                    .unwrap_or(0)
            } else {
                0
            };
            ts_b.cmp(&ts_a) // decroissant : le plus recent en premier
        });
        servers
    }

    /// Indique si un namespace contient des serveurs favoris visibles.
    fn namespace_has_visible_servers(&self, entries: &[ConfigEntry], ns: &str) -> bool {
        for entry in entries {
            match entry {
                ConfigEntry::Group(g) => {
                    if self.group_has_visible_servers(g, ns) {
                        return true;
                    }
                }
                ConfigEntry::Server(s) => {
                    if let Some(resolved) = self
                        .resolved_servers
                        .iter()
                        .find(|rs| rs.name == s.name && rs.namespace == ns)
                        && self.favorites.contains(&Self::server_key(resolved))
                    {
                        return true;
                    }
                }
                ConfigEntry::Namespace(_) => {}
            }
        }
        false
    }

    /// Indique si un groupe contient des serveurs favoris visibles.
    fn group_has_visible_servers(&self, group: &crate::config::Group, ns: &str) -> bool {
        if let Some(envs) = &group.environments {
            for env in envs {
                for s in &env.servers {
                    if let Some(resolved) = self.resolved_servers.iter().find(|rs| {
                        rs.name == s.name
                            && rs.group_name == group.name
                            && rs.env_name == env.name
                            && rs.namespace == ns
                    }) && self.favorites.contains(&Self::server_key(resolved))
                    {
                        return true;
                    }
                }
            }
        }
        if let Some(servers) = &group.servers {
            for s in servers {
                if let Some(resolved) = self.resolved_servers.iter().find(|rs| {
                    rs.name == s.name
                        && rs.group_name == group.name
                        && rs.env_name.is_empty()
                        && rs.namespace == ns
                }) && self.favorites.contains(&Self::server_key(resolved))
                {
                    return true;
                }
            }
        }
        false
    }

    /// Itère les entrees d'un namespace et les pousse dans `items`.
    fn push_entries(&self, entries: &[ConfigEntry], ns: &str, items: &mut Vec<ConfigItem>) {
        let searching = !self.search_query.is_empty();

        for entry in entries {
            match entry {
                ConfigEntry::Group(group) => {
                    items.push(ConfigItem::Group(group.name.clone(), ns.to_string()));
                    let group_id = format!("NS:{}:Group:{}", ns, group.name);
                    let group_expanded = self.expanded_items.contains(&group_id) || searching;
                    if group_expanded {
                        self.push_group_children_ns(group, ns, items);
                    }
                }
                ConfigEntry::Server(s_conf) => {
                    if let Some(resolved) = self.resolved_servers.iter().find(|rs| {
                        rs.name == s_conf.name
                            && rs.group_name.is_empty()
                            && rs.env_name.is_empty()
                            && rs.namespace == ns
                    }) {
                        if searching
                            && !self.matches_search(&resolved.name, &resolved.host, &resolved.tags)
                        {
                            continue;
                        }
                        items.push(ConfigItem::Server(Box::new(resolved.clone())));
                    }
                }
                ConfigEntry::Namespace(_) => {} // imbrique - ignore
            }
        }
    }

    /// Enfants d'un groupe racine (pas de namespace).
    fn push_group_children(
        &self,
        group: &crate::config::Group,
        _ns: &str,
        items: &mut Vec<ConfigItem>,
    ) {
        let searching = !self.search_query.is_empty();

        if let Some(envs) = &group.environments {
            for env in envs {
                // En mode favoris, ignorer l'env s'il n'a aucun favori visible
                if self.favorites_only {
                    let has_fav = env.servers.iter().any(|s| {
                        self.resolved_servers.iter().any(|rs| {
                            rs.name == s.name
                                && rs.env_name == env.name
                                && rs.group_name == group.name
                                && rs.namespace.is_empty()
                                && self.favorites.contains(&Self::server_key(rs))
                        })
                    });
                    if !has_fav {
                        continue;
                    }
                }
                items.push(ConfigItem::Environment(
                    group.name.clone(),
                    env.name.clone(),
                    String::new(),
                ));
                let env_id = format!("Env:{}:{}", group.name, env.name);
                let env_expanded = self.expanded_items.contains(&env_id) || searching;
                if env_expanded {
                    for server in &env.servers {
                        if let Some(resolved) = self.resolved_servers.iter().find(|rs| {
                            rs.name == server.name
                                && rs.env_name == env.name
                                && rs.group_name == group.name
                                && rs.namespace.is_empty()
                        }) {
                            if searching
                                && !self.matches_search(
                                    &resolved.name,
                                    &resolved.host,
                                    &resolved.tags,
                                )
                            {
                                continue;
                            }
                            if self.favorites_only
                                && !self.favorites.contains(&Self::server_key(resolved))
                            {
                                continue;
                            }
                            items.push(ConfigItem::Server(Box::new(resolved.clone())));
                        }
                    }
                }
            }
        }

        if let Some(servers) = &group.servers {
            for server in servers {
                if let Some(resolved) = self.resolved_servers.iter().find(|rs| {
                    rs.name == server.name
                        && rs.env_name.is_empty()
                        && rs.group_name == group.name
                        && rs.namespace.is_empty()
                }) {
                    if searching
                        && !self.matches_search(&resolved.name, &resolved.host, &resolved.tags)
                    {
                        continue;
                    }
                    if self.favorites_only && !self.favorites.contains(&Self::server_key(resolved))
                    {
                        continue;
                    }
                    items.push(ConfigItem::Server(Box::new(resolved.clone())));
                }
            }
        }
    }

    /// Enfants d'un groupe sous namespace.
    fn push_group_children_ns(
        &self,
        group: &crate::config::Group,
        ns: &str,
        items: &mut Vec<ConfigItem>,
    ) {
        let searching = !self.search_query.is_empty();

        if let Some(envs) = &group.environments {
            for env in envs {
                if self.favorites_only {
                    let has_fav = env.servers.iter().any(|s| {
                        self.resolved_servers.iter().any(|rs| {
                            rs.name == s.name
                                && rs.env_name == env.name
                                && rs.group_name == group.name
                                && rs.namespace == ns
                                && self.favorites.contains(&Self::server_key(rs))
                        })
                    });
                    if !has_fav {
                        continue;
                    }
                }
                items.push(ConfigItem::Environment(
                    group.name.clone(),
                    env.name.clone(),
                    ns.to_string(),
                ));
                let env_id = format!("NS:{}:Env:{}:{}", ns, group.name, env.name);
                let env_expanded = self.expanded_items.contains(&env_id) || searching;
                if env_expanded {
                    for server in &env.servers {
                        if let Some(resolved) = self.resolved_servers.iter().find(|rs| {
                            rs.name == server.name
                                && rs.env_name == env.name
                                && rs.group_name == group.name
                                && rs.namespace == ns
                        }) {
                            if searching
                                && !self.matches_search(
                                    &resolved.name,
                                    &resolved.host,
                                    &resolved.tags,
                                )
                            {
                                continue;
                            }
                            if self.favorites_only
                                && !self.favorites.contains(&Self::server_key(resolved))
                            {
                                continue;
                            }
                            items.push(ConfigItem::Server(Box::new(resolved.clone())));
                        }
                    }
                }
            }
        }

        if let Some(servers) = &group.servers {
            for server in servers {
                if let Some(resolved) = self.resolved_servers.iter().find(|rs| {
                    rs.name == server.name
                        && rs.env_name.is_empty()
                        && rs.group_name == group.name
                        && rs.namespace == ns
                }) {
                    if searching
                        && !self.matches_search(&resolved.name, &resolved.host, &resolved.tags)
                    {
                        continue;
                    }
                    if self.favorites_only && !self.favorites.contains(&Self::server_key(resolved))
                    {
                        continue;
                    }
                    items.push(ConfigItem::Server(Box::new(resolved.clone())));
                }
            }
        }
    }

    fn matches_search(&self, name: &str, host: &str, tags: &[String]) -> bool {
        search::matches_search_query(&self.search_query, name, host, tags)
    }
}
