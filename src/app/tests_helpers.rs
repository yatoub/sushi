use super::*;

pub(super) fn make_namespace_config() -> Config {
    use crate::config::{NamespaceEntry, Server};

    Config {
        defaults: None,
        includes: vec![],
        groups: vec![
            ConfigEntry::Group(crate::config::Group {
                name: "RootGroup".to_string(),
                user: None,
                ssh_key: None,
                mode: None,
                ssh_port: None,
                ssh_options: None,
                wallix: None,
                wallix_group: None,
                jump: None,
                probe_filesystems: None,
                environments: None,
                tunnels: None,
                tags: None,
                servers: Some(vec![Server {
                    name: "root_srv".to_string(),
                    host: "203.0.113.1".to_string(),
                    user: None,
                    ssh_key: None,
                    ssh_port: None,
                    ssh_options: None,
                    mode: None,
                    wallix: None,
                    jump: None,
                    probe_filesystems: None,
                    tunnels: None,
                    tags: None,
                    ..Default::default()
                }]),
            }),
            ConfigEntry::Namespace(NamespaceEntry {
                label: "CES".to_string(),
                source_path: "/fake/ces.yml".to_string(),
                defaults: None,
                vars: Default::default(),
                entries: vec![ConfigEntry::Group(crate::config::Group {
                    name: "CES_Group".to_string(),
                    user: None,
                    ssh_key: None,
                    mode: None,
                    ssh_port: None,
                    ssh_options: None,
                    wallix: None,
                    wallix_group: None,
                    jump: None,
                    probe_filesystems: None,
                    environments: None,
                    tunnels: None,
                    tags: None,
                    servers: Some(vec![Server {
                        name: "ces_srv".to_string(),
                        host: "203.0.113.2".to_string(),
                        user: None,
                        ssh_key: None,
                        ssh_port: None,
                        ssh_options: None,
                        mode: None,
                        wallix: None,
                        jump: None,
                        probe_filesystems: None,
                        tunnels: None,
                        tags: None,
                        ..Default::default()
                    }]),
                })],
            }),
        ],
        vars: Default::default(),
    }
}
