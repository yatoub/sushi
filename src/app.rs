use std::collections::HashSet;
use crate::config::{Config, ResolvedServer};

// Enum to represent different types of items in the list
#[derive(Debug, Clone)]
pub enum ConfigItem {
    Group(String), // Name
    Environment(String, String), // GroupName, EnvName
    Server(ResolvedServer),
}

pub struct App {
    pub config: Config,
    pub resolved_servers: Vec<ResolvedServer>,
    pub should_quit: bool,
    
    // Navigation state
    pub selected_index: usize,
    pub expanded_items: HashSet<String>, // Stores IDs like "Group:Name" or "Env:Group:Name"
    
    // Search
    pub search_query: String,
    pub is_searching: bool,
}

impl App {
    pub fn new(config: Config) -> Self {
        let resolved = config.resolve().unwrap_or_default(); 
        
        let mut app = Self {
            config,
            resolved_servers: resolved,
            should_quit: false,
            selected_index: 0,
            expanded_items: HashSet::new(),
            search_query: String::new(),
            is_searching: false,
        };
        
        // Expand all by default for now
        for group in &app.config.groups {
            app.expanded_items.insert(format!("Group:{}", group.name));
            for env in &group.environments {
                 app.expanded_items.insert(format!("Env:{}:{}", group.name, env.name));
            }
        }
        
        app
    }

    pub fn get_visible_items(&self) -> Vec<ConfigItem> {
        let mut items = Vec::new();
        
        for group in &self.config.groups {
            items.push(ConfigItem::Group(group.name.clone()));
            
            // If group expanded or searching (assume search expands context)
            let group_id = format!("Group:{}", group.name);
            let group_expanded = self.expanded_items.contains(&group_id);
            
            // Simple logic: If filtering, just check if any child server matches
            // If yes, show group and env context?
            // "Fuzzy-find search bar doit filtrer les serveurs en temps réel."
            // When filtering "web", show only "web" servers under their hierarchy? Or flatten?
            // Let's implement full hierarchy filtering: show parents if children match.
            // But for now, stick to basic expansion logic.
            
            if group_expanded || !self.search_query.is_empty() {
                for env in &group.environments {
                    items.push(ConfigItem::Environment(group.name.clone(), env.name.clone()));
                    
                    let env_id = format!("Env:{}:{}", group.name, env.name);
                    let env_expanded = self.expanded_items.contains(&env_id);

                    if env_expanded || !self.search_query.is_empty() {
                        for server in &env.servers {
                            if !self.search_query.is_empty() && !server.name.to_lowercase().contains(&self.search_query.to_lowercase()) {
                                continue;
                            }
                            
                            // Find the resolved server to store in the item
                            // We need to match by group, env, and name to be precise if multiple exist
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
        }
        items
    }

    pub fn next(&mut self) {
        let count = self.get_visible_items().len();
        if count > 0 {
            self.selected_index = (self.selected_index + 1) % count;
        }
    }

    pub fn previous(&mut self) {
        let count = self.get_visible_items().len();
        if count > 0 {
            if self.selected_index > 0 {
                self.selected_index -= 1;
            } else {
                self.selected_index = count - 1;
            }
        }
    }

    pub fn on_tick(&mut self) {
        // Handle tick events if needed
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}
