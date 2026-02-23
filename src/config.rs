use serde::Deserialize;
use thiserror::Error;
use std::fs;
use std::path::Path;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("Missing configuration for server '{0}': {1}")]
    MissingField(String, String), // server_name, field_name
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub defaults: Option<Defaults>,
    pub groups: Vec<Group>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Defaults {
    pub user: Option<String>,
    pub ssh_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Group {
    pub name: String,
    pub user: Option<String>,
    pub ssh_key: Option<String>,
    pub environments: Vec<Environment>,
}

#[derive(Debug, Deserialize)]
pub struct Environment {
    pub name: String,
    pub user: Option<String>,
    pub ssh_key: Option<String>,
    pub servers: Vec<ServerRaw>,
}

#[derive(Debug, Deserialize)]
pub struct ServerRaw {
    pub name: String,
    pub host: String,
    pub user: Option<String>,
    pub ssh_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedServer {
    pub group_name: String,
    pub env_name: String,
    pub name: String,
    pub host: String,
    pub user: String,
    pub ssh_key: String,
}

impl Config {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path)?;
        let config = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    pub fn resolve(&self) -> Result<Vec<ResolvedServer>, ConfigError> {
        let mut resolved = Vec::new();
        
        let default_user = self.defaults.as_ref().and_then(|d| d.user.clone());
        let default_key = self.defaults.as_ref().and_then(|d| d.ssh_key.clone());

        for group in &self.groups {
            let group_user = group.user.clone().or(default_user.clone());
            let group_key = group.ssh_key.clone().or(default_key.clone());

            for env in &group.environments {
                let env_user = env.user.clone().or(group_user.clone());
                let env_key = env.ssh_key.clone().or(group_key.clone());

                for server in &env.servers {
                    let final_user = server.user.clone().or(env_user.clone());
                    let final_key = server.ssh_key.clone().or(env_key.clone());

                    match (final_user, final_key) {
                        (Some(u), Some(k)) => {
                            resolved.push(ResolvedServer {
                                group_name: group.name.clone(),
                                env_name: env.name.clone(),
                                name: server.name.clone(),
                                host: server.host.clone(),
                                user: u,
                                ssh_key: k,
                            });
                        }
                        (None, _) => return Err(ConfigError::MissingField(server.name.clone(), "user".to_string())),
                        (_, None) => return Err(ConfigError::MissingField(server.name.clone(), "ssh_key".to_string())),
                    }
                }
            }
        }

        Ok(resolved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_server_inherits_user_from_group() {
        let yaml = r#"
            defaults:
              user: "global_admin"
              ssh_key: "~/.ssh/id_rsa"
            groups:
              - name: "Alpha"
                user: "group_user"
                environments:
                  - name: "Prod"
                    servers:
                      - name: "srv-1"
                        host: "10.0.0.1"
        "#;
        
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        let servers = config.resolve().unwrap();
        
        assert_eq!(servers.len(), 1);
        let s = &servers[0];
        
        assert_eq!(s.name, "srv-1");
        assert_eq!(s.user, "group_user"); // From Group
        assert_eq!(s.ssh_key, "~/.ssh/id_rsa"); // From Defaults
    }

    #[test]
    fn test_missing_config_returns_error() {
        let yaml = r#"
            groups:
              - name: "GroupBad"
                environments:
                  - name: "Prod"
                    servers:
                      - name: "srv-bad"
                        host: "1.2.3.4"
        "#;
        // user and ssh_key are missing everywhere
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        let result = config.resolve();
        
        match result {
            Err(ConfigError::MissingField(server, field)) => {
                assert_eq!(server, "srv-bad");
                assert!(field == "user" || field == "ssh_key");
            }
            _ => panic!("Expected MissingField error, got check logic"),
        }
    }

    #[test]
    fn test_load_from_file() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        writeln!(file, r#"
            defaults:
              user: "file_user"
              ssh_key: "file_key"
            groups: []
        "#).unwrap();

        let config = Config::load_from_file(file.path()).unwrap();
        assert_eq!(config.defaults.unwrap().user.unwrap(), "file_user");
    }
}
