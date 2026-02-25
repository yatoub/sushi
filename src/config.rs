use serde::Deserialize;
use thiserror::Error;
use std::path::Path;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("Missing configuration for server '{0}': {1}")]
    MissingField(String, String),
}

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    pub defaults: Option<Defaults>,
    pub groups: Vec<ConfigEntry>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum ConfigEntry {
    Server(Server),
    Group(Group),
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Defaults {
    pub user: Option<String>,
    pub ssh_key: Option<String>,
    pub mode: Option<String>,
    pub ssh_port: Option<u16>,
    pub ssh_options: Option<Vec<String>>,
    pub bastion: Option<BastionConfig>,
    pub rebond: Option<JumpConfig>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct BastionConfig {
    pub host: Option<String>,
    pub user: Option<String>,
    pub template: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct JumpConfig {
    pub host: Option<String>,
    pub user: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Group {
    pub name: String,
    pub user: Option<String>,
    pub ssh_key: Option<String>,
    pub mode: Option<String>,
    pub ssh_port: Option<u16>,
    pub ssh_options: Option<Vec<String>>,
    pub bastion: Option<BastionConfig>,
    pub rebond: Option<JumpConfig>,
    pub environments: Option<Vec<Environment>>,
    pub servers: Option<Vec<Server>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Environment {
    pub name: String,
    pub user: Option<String>,
    pub ssh_key: Option<String>,
    pub mode: Option<String>,
    pub ssh_port: Option<u16>,
    pub ssh_options: Option<Vec<String>>,
    pub bastion: Option<BastionConfig>,
    pub rebond: Option<JumpConfig>,
    pub servers: Vec<Server>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Server {
    pub name: String,
    pub host: String, // Host is mandatory on leaf
    pub user: Option<String>,
    pub ssh_key: Option<String>,
    pub ssh_port: Option<u16>,
    pub ssh_options: Option<Vec<String>>,
    pub mode: Option<String>, // "direct", "jump", "bastion"
    pub bastion: Option<BastionConfig>,
    pub rebond: Option<JumpConfig>,
}

#[derive(Debug, Clone)]
pub struct ResolvedServer {
    pub group_name: String,
    pub env_name: String, 
    pub name: String,
    pub host: String,
    pub user: String,
    pub port: u16,
    pub ssh_key: String,
    pub ssh_options: Vec<String>,
    pub default_mode: String, 
    pub jump_host: Option<String>,
    pub jump_user: Option<String>,
    pub bastion_host: Option<String>,
    pub bastion_user: Option<String>,
    pub bastion_template: String,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let mut config: Config = serde_yaml::from_str(&content)?;
        config.sort();
        Ok(config)
    }

    pub fn sort(&mut self) {
        // Sort top-level entries (Groups or Servers)
        self.groups.sort_by(|a, b| {
            let name_a = match a {
                ConfigEntry::Group(g) => &g.name,
                ConfigEntry::Server(s) => &s.name,
            };
            let name_b = match b {
                ConfigEntry::Group(g) => &g.name,
                ConfigEntry::Server(s) => &s.name,
            };
            name_a.cmp(name_b)
        });

        // Sort children
        for entry in &mut self.groups {
            if let ConfigEntry::Group(group) = entry {
                // Sort environments
                if let Some(envs) = &mut group.environments {
                    envs.sort_by(|a, b| a.name.cmp(&b.name));
                    // Sort servers inside environments
                    for env in envs {
                        env.servers.sort_by(|a, b| a.name.cmp(&b.name));
                    }
                }
                // Sort direct servers in group
                if let Some(servers) = &mut group.servers {
                    servers.sort_by(|a, b| a.name.cmp(&b.name));
                }
            }
        }
    }

    pub fn resolve(&self) -> Result<Vec<ResolvedServer>, ConfigError> {
        let mut resolved = Vec::new();
        
        let d = self.defaults.clone().unwrap_or_default();
        
        for entry in &self.groups {
            match entry {
                ConfigEntry::Group(group) => {
                    // Merge defaults -> Group
                    let g_user = group.user.as_deref().or(d.user.as_deref());
                    let g_key = group.ssh_key.as_deref().or(d.ssh_key.as_deref());
                    let g_mode = group.mode.as_deref().or(d.mode.as_deref());
                    let g_port = group.ssh_port.or(d.ssh_port);
                    let g_opts = if let Some(opts) = &group.ssh_options {
                         Some(opts.clone())
                    } else {
                         d.ssh_options.clone()
                    };

                    let g_bastion = merge_bastion(&d.bastion, &group.bastion);
                    let g_jump = merge_jump(&d.rebond, &group.rebond);

                    if let Some(envs) = &group.environments {
                        for env in envs {
                            // Merge Group -> Env
                            let e_user = env.user.as_deref().or(g_user);
                            let e_key = env.ssh_key.as_deref().or(g_key);
                            let e_mode = env.mode.as_deref().or(g_mode);
                            let e_port = env.ssh_port.or(g_port);
                            let e_opts = if let Some(opts) = &env.ssh_options {
                                 Some(opts.clone())
                            } else {
                                 g_opts.clone()
                            };
                            
                            let e_bastion = merge_bastion(&g_bastion, &env.bastion);
                            let e_jump = merge_jump(&g_jump, &env.rebond);

                            for server in &env.servers {
                                 let r = resolve_server(
                                     server, 
                                     &group.name, 
                                     &env.name,
                                     e_user, e_key, e_mode, e_port, e_opts.as_ref(), // Pass ref
                                     &e_bastion, &e_jump
                                 )?;
                                 resolved.push(r);
                            }
                        }
                    }
                    
                    if let Some(servers) = &group.servers {
                        for server in servers {
                             let r = resolve_server(
                                 server, 
                                 &group.name, 
                                 "",
                                 g_user, g_key, g_mode, g_port, g_opts.as_ref(),
                                 &g_bastion, &g_jump
                             )?;
                             resolved.push(r);
                        }
                    }
                },
                ConfigEntry::Server(server) => {
                     // Top-level server
                     // Use empty string for group/env to signify top-level
                     let r = resolve_server(
                         server, 
                         "", 
                         "",
                         d.user.as_deref(), d.ssh_key.as_deref(), d.mode.as_deref(), d.ssh_port, d.ssh_options.as_ref(),
                         &d.bastion, &d.rebond
                     )?;
                     resolved.push(r);
                }
            }
        }
        
        Ok(resolved)
    }
}

fn merge_bastion(parent: &Option<BastionConfig>, child: &Option<BastionConfig>) -> Option<BastionConfig> {
    match (parent, child) {
        (None, None) => None,
        (Some(p), None) => Some(p.clone()),
        (None, Some(c)) => Some(c.clone()),
        (Some(p), Some(c)) => {
            Some(BastionConfig {
                host: c.host.clone().or(p.host.clone()),
                user: c.user.clone().or(p.user.clone()),
                template: c.template.clone().or(p.template.clone()),
            })
        }
    }
}

fn merge_jump(parent: &Option<JumpConfig>, child: &Option<JumpConfig>) -> Option<JumpConfig> {
    match (parent, child) {
        (None, None) => None,
        (Some(p), None) => Some(p.clone()),
        (None, Some(c)) => Some(c.clone()),
        (Some(p), Some(c)) => {
             Some(JumpConfig {
                host: c.host.clone().or(p.host.clone()),
                user: c.user.clone().or(p.user.clone()),
            })
        }
    }
}

fn resolve_server(
    s: &Server,
    group: &str,
    env: &str,
    def_user: Option<&str>,
    def_key: Option<&str>,
    def_mode: Option<&str>,
    def_port: Option<u16>,
    def_opts: Option<&Vec<String>>,
    def_bastion: &Option<BastionConfig>,
    def_jump: &Option<JumpConfig>,
) -> Result<ResolvedServer, ConfigError> {
    
    let user = s.user.as_deref().or(def_user).unwrap_or("root").to_string();
    let port = s.ssh_port.or(def_port).unwrap_or(22);
    let key = s.ssh_key.as_deref().or(def_key).unwrap_or("~/.ssh/id_rsa").to_string();
    
    let opts = if let Some(o) = &s.ssh_options {
        o.clone()
    } else {
        def_opts.cloned().unwrap_or_default()
    };
    
    let final_bastion = merge_bastion(def_bastion, &s.bastion);
    let final_jump = merge_jump(def_jump, &s.rebond);

    let mode_str = s.mode.as_deref().or(def_mode).unwrap_or("direct").to_string();
    
    let bastion_template = final_bastion.as_ref()
        .and_then(|b| b.template.clone())
        .unwrap_or_else(|| "{target_user}@%n:SSH:{bastion_user}".to_string());

    Ok(ResolvedServer {
        group_name: group.to_string(),
        env_name: env.to_string(),
        name: s.name.clone(),
        host: s.host.clone(),
        user,
        port,
        ssh_key: key,
        ssh_options: opts,
        default_mode: mode_str,
        
        jump_host: final_jump.as_ref().and_then(|j| j.host.clone()),
        jump_user: final_jump.as_ref().and_then(|j| j.user.clone()),
        bastion_host: final_bastion.as_ref().and_then(|b| b.host.clone()),
        bastion_user: final_bastion.as_ref().and_then(|b| b.user.clone()),
        bastion_template,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_bastion() {
        let parent = Some(BastionConfig {
            host: Some("parent_host".to_string()),
            user: Some("parent_user".to_string()),
            template: Some("parent_tmpl".to_string()),
        });
        let child = BastionConfig {
            host: None,
            user: Some("child_user".to_string()),
            template: None,
        };

        let merged = merge_bastion(&parent, &Some(child)).unwrap();
        // Child user overrides parent
        assert_eq!(merged.user, Some("child_user".to_string()));
        // Parent host is inherited
        assert_eq!(merged.host, Some("parent_host".to_string()));
        // Parent template is inherited
        assert_eq!(merged.template, Some("parent_tmpl".to_string()));
    }

    #[test]
    fn test_sorting_mixed() {
        let mut config = Config {
            defaults: None,
            groups: vec![
                ConfigEntry::Group(Group {
                    name: "Zeus".to_string(),
                    user: None, ssh_key: None, mode: None, ssh_port: None, ssh_options: None, bastion: None, rebond: None, environments: None, servers: None
                }),
                ConfigEntry::Server(Server {
                    name: "Alpha".to_string(),
                    host: "10.0.0.1".to_string(),
                    user: None, ssh_key: None, ssh_port: None, ssh_options: None, mode: None, bastion: None, rebond: None
                }),
                ConfigEntry::Group(Group {
                    name: "Beta".to_string(),
                    user: None, ssh_key: None, mode: None, ssh_port: None, ssh_options: None, bastion: None, rebond: None, environments: None, servers: None
                }),
            ],
        };

        config.sort();

        // Check order: Alpha, Beta, Zeus
        match &config.groups[0] {
            ConfigEntry::Server(s) => assert_eq!(s.name, "Alpha"),
            _ => panic!("Expected Alpha first"),
        }
        match &config.groups[1] {
            ConfigEntry::Group(g) => assert_eq!(g.name, "Beta"),
            _ => panic!("Expected Beta second"),
        }
        match &config.groups[2] {
            ConfigEntry::Group(g) => assert_eq!(g.name, "Zeus"),
            _ => panic!("Expected Zeus third"),
        }
    }

    #[test]
    fn test_resolve_inheritance_chain() {
        let config = Config {
            defaults: Some(Defaults {
                user: Some("default_user".to_string()),
                ssh_port: Some(2222),
                ..Default::default()
            }),
            groups: vec![
                ConfigEntry::Group(Group {
                    name: "G1".to_string(),
                    user: Some("group_user".to_string()), // Override default
                    ssh_key: None,
                    mode: None,
                    ssh_port: None, // Inherits 2222
                    ssh_options: None,
                    bastion: None,
                    rebond: None,
                    environments: Some(vec![
                        Environment {
                            name: "Env1".to_string(),
                            user: None, // Inherits "group_user"
                            ssh_key: None,
                            mode: None,
                            ssh_port: None, // Inherits 2222
                            ssh_options: None,
                            bastion: None,
                            rebond: None,
                            servers: vec![
                                Server {
                                    name: "S1".to_string(),
                                    host: "1.1.1.1".to_string(),
                                    user: None, // Inherits "group_user"
                                    ssh_key: None,
                                    ssh_port: Some(8080), // Override 2222
                                    ssh_options: None,
                                    mode: None,
                                    bastion: None,
                                    rebond: None,
                                }
                            ]
                        }
                    ]),
                    servers: None,
                })
            ]
        };

        let resolved = config.resolve().unwrap();
        let s1 = &resolved[0];

        assert_eq!(s1.name, "S1");
        assert_eq!(s1.user, "group_user"); 
        assert_eq!(s1.port, 8080);
    }
}
