use std::collections::HashSet;
use ratatui::widgets::ListState;
use crate::config::{Config, ResolvedServer, ConfigEntry};

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
    
    pub connection_mode: usize, 
}

impl App {
    pub fn new(config: Config) -> Self {
        let resolved = config.resolve().unwrap_or_default(); 
        
        let mut app = Self {
            config,
            resolved_servers: resolved,
            selected_index: 0,
            list_state: ListState::default(),
            expanded_items: HashSet::new(),
            search_query: String::new(),
            is_searching: false,
            connection_mode: 0,
        };
        
        app.list_state.select(Some(0));

        // Start collapsed by default
        app.expanded_items.clear();
        
        app.update_mode_from_selection();
        app
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
                    if !self.search_query.is_empty() && !s_conf.name.to_lowercase().contains(&self.search_query.to_lowercase()) {
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
                                        if !self.search_query.is_empty() && !server.name.to_lowercase().contains(&self.search_query.to_lowercase()) {
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
                                if !self.search_query.is_empty() && !server.name.to_lowercase().contains(&self.search_query.to_lowercase()) {
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
                match server.default_mode.as_str() {
                    "jump" => self.connection_mode = 1,
                    "bastion" => self.connection_mode = 2,
                    _ => self.connection_mode = 0, // "direct" or default
                }
            }
        }
    }
}
