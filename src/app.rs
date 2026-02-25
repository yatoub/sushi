use std::collections::HashSet;
use ratatui::widgets::ListState;
use crate::config::{Config, ConfigError, ResolvedServer, ConfigEntry, ConnectionMode};

#[derive(Debug, Clone)]
pub enum ConfigItem {
    Group(String), 
    Environment(String, String), 
    Server(ResolvedServer),
}

pub struct App {
    pub config: Config,
    pub resolved_servers: Vec<ResolvedServer>,
    
    pub selected_index: usize,
    pub list_state: ListState,
    pub expanded_items: HashSet<String>, 
    
    pub search_query: String,
    pub is_searching: bool,
    
    pub connection_mode: ConnectionMode,
    pub verbose_mode: bool,
}

impl App {
    pub fn new(config: Config) -> Result<Self, ConfigError> {
        let resolved = config.resolve()?;
        
        let mut app = Self {
            config,
            resolved_servers: resolved,
            selected_index: 0,
            list_state: ListState::default(),
            expanded_items: HashSet::new(),
            search_query: String::new(),
            is_searching: false,
            connection_mode: ConnectionMode::Direct,
            verbose_mode: false,
        };
        
        app.list_state.select(Some(0));

        // Start collapsed by default
        app.expanded_items.clear();
        
        app.update_mode_from_selection();
        Ok(app)
    }

    pub fn toggle_expansion(&mut self) {
        let items = self.get_visible_items();
        if let Some(item) = items.get(self.selected_index) {
            match item {
                ConfigItem::Group(name) => {
                    let id = format!("Group:{}", name);
                    if self.expanded_items.contains(&id) {
                        self.expanded_items.remove(&id);
                    } else {
                        self.expanded_items.insert(id);
                    }
                }
                ConfigItem::Environment(g, e) => {
                    let id = format!("Env:{}:{}", g, e);
                    if self.expanded_items.contains(&id) {
                         self.expanded_items.remove(&id);
                    } else {
                         self.expanded_items.insert(id);
                    }
                }
                _ => {}
            }
        }
    }

    pub fn get_visible_items(&self) -> Vec<ConfigItem> {
        let mut items = Vec::new();
        
        for entry in &self.config.groups {
            match entry {
                ConfigEntry::Server(s_conf) => {
                    // Top-level server
                    if !self.search_query.is_empty() && !self.matches_search(&s_conf.name, &s_conf.host) {
                        continue;
                    }
                    // Find resolved server with empty group name
                    if let Some(resolved) = self.resolved_servers.iter().find(|rs| 
                        rs.name == s_conf.name && 
                        rs.group_name.is_empty() && 
                        rs.env_name.is_empty()
                    ) {
                        items.push(ConfigItem::Server(resolved.clone()));
                    }
                },
                ConfigEntry::Group(group) => {
                    items.push(ConfigItem::Group(group.name.clone()));
                    
                    let group_id = format!("Group:{}", group.name);
                    let group_expanded = self.expanded_items.contains(&group_id) || !self.search_query.is_empty();
                    
                    if group_expanded {
                        if let Some(envs) = &group.environments {
                            for env in envs {
                                items.push(ConfigItem::Environment(group.name.clone(), env.name.clone()));
                                
                                let env_id = format!("Env:{}:{}", group.name, env.name);
                                let env_expanded = self.expanded_items.contains(&env_id) || !self.search_query.is_empty();

                                if env_expanded {
                                    for server in &env.servers {
                                        if !self.search_query.is_empty() && !self.matches_search(&server.name, &server.host) {
                                            continue;
                                        }
                                        
                                        if let Some(resolved) = self.resolved_servers.iter().find(|rs| 
                                            rs.name == server.name && 
                                            rs.env_name == env.name && 
                                            rs.group_name == group.name
                                        ) {
                                            items.push(ConfigItem::Server(resolved.clone()));
                                        }
                                    }
                                }
                            }
                        }
                        
                        if let Some(servers) = &group.servers {
                            for server in servers {
                                if !self.search_query.is_empty() && !self.matches_search(&server.name, &server.host) {
                                    continue;
                                }

                                if let Some(resolved) = self.resolved_servers.iter().find(|rs| 
                                    rs.name == server.name && 
                                    rs.env_name.is_empty() && 
                                    rs.group_name == group.name
                                ) {
                                     items.push(ConfigItem::Server(resolved.clone()));
                                }
                            }
                        }
                    }
                }
            }
        }
        items
    }

    fn matches_search(&self, name: &str, host: &str) -> bool {
        let query = self.search_query.to_lowercase();
        name.to_lowercase().contains(&query) || host.to_lowercase().contains(&query)
    }

    pub fn next(&mut self) {
        let count = self.get_visible_items().len();
        if count > 0 {
            self.selected_index = (self.selected_index + 1) % count;
            self.list_state.select(Some(self.selected_index));
            self.update_mode_from_selection();
        }
    }

    pub fn previous(&mut self) {
        let count = self.get_visible_items().len();
        if count > 0 {
            if self.selected_index == 0 {
                self.selected_index = count - 1;
            } else {
                self.selected_index -= 1;
            }
            self.list_state.select(Some(self.selected_index));
            self.update_mode_from_selection();
        }
    }

    pub fn select(&mut self, index: usize) {
        let count = self.get_visible_items().len();
        if count > 0 && index < count {
            self.selected_index = index;
            self.list_state.select(Some(self.selected_index));
            self.update_mode_from_selection();
        }
    }

    fn update_mode_from_selection(&mut self) {
        let items = self.get_visible_items();
        if let Some(item) = items.get(self.selected_index) {
            if let ConfigItem::Server(server) = item {
                self.connection_mode = server.default_mode;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Group, Server, Environment, ConfigEntry};

    fn create_test_config() -> Config {
        Config {
            defaults: None,
            groups: vec![
                ConfigEntry::Group(Group {
                    name: "G1".to_string(),
                    user: None, ssh_key: None, mode: None, ssh_port: None, ssh_options: None, bastion: None, rebond: None,
                    environments: Some(vec![
                        Environment {
                            name: "E1".to_string(),
                            user: None, ssh_key: None, mode: None, ssh_port: None, ssh_options: None, bastion: None, rebond: None,
                            servers: vec![
                                Server {
                                    name: "S1".to_string(),
                                    host: "10.0.0.1".to_string(),
                                    user: None, ssh_key: None, ssh_port: None, ssh_options: None, mode: None, bastion: None, rebond: None
                                }
                            ]
                        }
                    ]),
                    servers: Some(vec![
                        Server {
                            name: "S2".to_string(),
                            host: "10.0.0.2".to_string(),
                            user: None, ssh_key: None, ssh_port: None, ssh_options: None, mode: None, bastion: None, rebond: None
                        }
                    ]),
                })
            ]
        }
    }

    #[test]
    fn test_initial_visibility() {
        let config = create_test_config();
        let app = App::new(config).unwrap();
        let items = app.get_visible_items();
        
        // Initially only the group header is visible (collapsed state)
        assert_eq!(items.len(), 1);
        match &items[0] {
            ConfigItem::Group(name) => assert_eq!(name, "G1"),
            _ => panic!("Expected Group G1"),
        }
    }

    #[test]
    fn test_expansion() {
        let config = create_test_config();
        let mut app = App::new(config).unwrap();
        
        // Expand Expand group G1
        app.toggle_expansion(); 
        // Note: selected_index is 0, pointing to G1.
        
        let items = app.get_visible_items();
        
        // Should verify items: G1, E1, S2 (Environment header + direct server)
        // E1 is collapsed by default.
        
        // Order inside Group iteration:
        // 1. Environments (E1)
        // 2. Servers (S2)
        
        // So: G1, E1, S2
        assert_eq!(items.len(), 3);
        
        match &items[1] {
            ConfigItem::Environment(g, e) => {
                assert_eq!(g, "G1");
                assert_eq!(e, "E1");
            },
            _ => panic!("Expected Environment E1"),
        }
        
        match &items[2] {
            ConfigItem::Server(s) => assert_eq!(s.name, "S2"),
            _ => panic!("Expected Server S2"),
        }
    }

    #[test]
    fn test_search_filtering() {
        let config = create_test_config();
        let mut app = App::new(config).unwrap();
        
        app.search_query = "S1".to_string();
        let items = app.get_visible_items();
        
        // With search "S1":
        // G1 (header always shown)
        // E1 (header always shown inside expanded group)
        // S1 (matches "S1")
        // S2 (filtered out)
        
        assert!(items.len() >= 3);
        
        // Verify S1 is present
        let has_s1 = items.iter().any(|i| match i {
            ConfigItem::Server(s) => s.name == "S1",
            _ => false,
        });
        assert!(has_s1, "Should contain S1");
        
        // Verify S2 is filtered out
        let has_s2 = items.iter().any(|i| match i {
             ConfigItem::Server(s) => s.name == "S2",
             _ => false,
        });
        assert!(!has_s2, "Should NOT contain S2");
    }
}
