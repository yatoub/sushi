//! Wallix menu parser and selection logic.
//!
//! This module provides utilities to parse Wallix's interactive menu output
//! and automatically select an entry based on target (user@account@host:protocol)
//! and group matching.

use crate::config::ResolvedServer;
use anyhow::{Result, anyhow};

/// Represents a single entry from a Wallix menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WallixMenuEntry {
    /// Menu entry ID (as String to preserve leading zeros).
    pub id: String,
    /// Target in format "user@account@host:protocol".
    pub target: String,
    /// Authorization group name.
    pub group: String,
}

/// Build the expected Wallix target string from a resolved server.
///
/// Format: `user@account@host:protocol`
pub fn build_expected_target(server: &ResolvedServer) -> String {
    format!(
        "{}@{}@{}:{}",
        server.user, server.wallix_account, server.host, server.wallix_protocol
    )
}

fn normalize_target_segment(value: &str) -> String {
    let mut normalized = String::new();
    let mut previous_was_dash = false;

    for character in value.trim().chars() {
        if character.is_ascii_alphanumeric() {
            normalized.push(character.to_ascii_uppercase());
            previous_was_dash = false;
        } else if !previous_was_dash {
            normalized.push('-');
            previous_was_dash = true;
        }
    }

    normalized.trim_matches('-').to_string()
}

fn infer_wallix_role(server: &ResolvedServer) -> Option<String> {
    let first_host_label = server
        .host
        .split('.')
        .next()
        .unwrap_or(server.host.as_str());
    let host_last_token = first_host_label
        .rsplit('-')
        .next()
        .unwrap_or(first_host_label);
    let source = if !server.name.trim().is_empty() {
        server.name.as_str()
    } else {
        host_last_token
    };

    let role_key: String = source
        .chars()
        .take_while(|character| !character.is_ascii_digit())
        .collect::<String>()
        .to_ascii_lowercase();

    let role = match role_key.as_str() {
        "bdd" | "bd" | "db" => "BD",
        "apps" | "app" | "appli" => "APPLI",
        "adm" | "admin" => "ADMIN",
        "web" => "WEB",
        "kafka" => "KAFKA",
        "els" => "ELS",
        "mig" => "MIG",
        "idp" => "IDP",
        "frt" | "frtrac" => "FRTRAC",
        other if !other.is_empty() => return Some(normalize_target_segment(other)),
        _ => return None,
    };

    Some(role.to_string())
}

pub fn build_expected_targets(server: &ResolvedServer) -> Vec<String> {
    let mut candidates = vec![build_expected_target(server)];

    let first_host_label = server
        .host
        .split('.')
        .next()
        .unwrap_or(server.host.as_str());
    let short_host = normalize_target_segment(first_host_label);
    if !short_host.is_empty() {
        candidates.push(format!(
            "{}@{}@{}:{}",
            server.user, server.wallix_account, short_host, server.wallix_protocol
        ));
    }

    let env = if !server.env_name.trim().is_empty() {
        normalize_target_segment(&server.env_name)
    } else {
        normalize_target_segment(first_host_label.split('-').next().unwrap_or_default())
    };

    let project_from_domain = server.host.split('.').nth(1).map(normalize_target_segment);
    let project_from_group = if server.group_name.trim().is_empty() {
        String::new()
    } else {
        normalize_target_segment(&server.group_name)
    };

    let role = infer_wallix_role(server).unwrap_or_default();

    for project in [project_from_domain.unwrap_or_default(), project_from_group] {
        if !env.is_empty() && !project.is_empty() && !role.is_empty() {
            candidates.push(format!(
                "{}@{}@{}-{}-{}:{}",
                server.user, server.wallix_account, env, project, role, server.wallix_protocol
            ));
        }
    }

    candidates.dedup();
    candidates
}

/// Parse Wallix menu output into structured entries.
///
/// The Wallix menu typically looks like:
/// ```text
///    ID │ Cible                              │ Autorisation
/// ───────┼────────────────────────────────────┼──────────────────────
///  1234  │ demo_user@default@APP-ALPHA-BD:SSH    │ APP-ALPHA_ops-admins
///  5678  │ demo_user@default@APP-ALPHA-BD:SSH    │ APP-ALPHA_dev-admins
/// ```
///
/// This function extracts the ID, target (Cible), and group (Autorisation) columns.
pub fn parse_wallix_menu(output: &str) -> Result<Vec<WallixMenuEntry>> {
    let mut entries = Vec::new();

    for line in output.lines() {
        let trimmed = line.trim();

        // Ignore headers, separators and empty lines.
        if trimmed.is_empty()
            || trimmed.contains("ID")
            || trimmed.contains("Cible")
            || trimmed.contains("Autorisation")
            || trimmed
                .chars()
                .all(|c| matches!(c, '─' | '┼' | '│' | '-' | '+'))
        {
            continue;
        }

        let separator = if trimmed.contains('│') {
            '│'
        } else if trimmed.contains('|') {
            '|'
        } else {
            continue;
        };

        let mut columns = trimmed
            .split(separator)
            .map(str::trim)
            .filter(|column| !column.is_empty());
        let Some(id) = columns.next() else {
            continue;
        };
        let Some(target) = columns.next() else {
            continue;
        };
        let Some(group) = columns.next() else {
            continue;
        };

        if !id.is_empty()
            && id.chars().all(|character| character.is_ascii_digit())
            && !target.is_empty()
            && !group.is_empty()
        {
            entries.push(WallixMenuEntry {
                id: id.to_string(),
                target: target.to_string(),
                group: group.to_string(),
            });
        }
    }

    if entries.is_empty() {
        return Err(anyhow!("No valid menu entries found in Wallix output"));
    }

    Ok(entries)
}

/// Select a menu entry ID based on target and group matching.
///
/// This function implements the core selection algorithm:
/// 1. Filter entries by exact target match (user@account@host:protocol).
/// 2. Filter by exact group match (`wallix.group`, or legacy `wallix_group`).
/// 3. Return error if no match or multiple matches found.
/// 4. Return the ID if exactly one match.
pub fn select_id_by_target_and_group(
    entries: &[WallixMenuEntry],
    target: &str,
    group: &str,
) -> Result<String> {
    let target_matches: Vec<_> = entries.iter().filter(|e| e.target == target).collect();

    if target_matches.is_empty() {
        return Err(anyhow!(
            "No menu entry found with target '{}'. Available targets: {}",
            target,
            entries
                .iter()
                .map(|e| e.target.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    let group_matches: Vec<_> = target_matches.iter().filter(|e| e.group == group).collect();

    match group_matches.len() {
        0 => Err(anyhow!(
            "No menu entry found for target '{}' with group '{}'. Available groups for this target: {}",
            target,
            group,
            target_matches
                .iter()
                .map(|e| e.group.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
        1 => Ok(group_matches[0].id.clone()),
        n => Err(anyhow!(
            "Multiple menu entries ({}) found for target '{}' and group '{}'. Cannot auto-select.",
            n,
            target,
            group
        )),
    }
}

fn normalize_authorization_segment(value: &str) -> String {
    let mut normalized = String::new();
    let mut previous_was_dash = false;

    for character in value.trim().chars() {
        if character.is_ascii_alphanumeric() {
            normalized.push(character.to_ascii_uppercase());
            previous_was_dash = false;
        } else if !previous_was_dash {
            normalized.push('-');
            previous_was_dash = true;
        }
    }

    normalized.trim_matches('-').to_string()
}

pub fn build_expected_groups(server: &ResolvedServer) -> Result<Vec<String>> {
    let configured_group = server.wallix_group.as_deref().ok_or_else(|| {
        anyhow!(
            "wallix.group is not configured for server '{}'",
            server.name
        )
    })?;

    let mut candidates = vec![configured_group.to_string()];

    if !configured_group.contains('_') {
        let mut prefix_parts = Vec::new();
        if !server.env_name.trim().is_empty() {
            let env = normalize_authorization_segment(&server.env_name);
            if !env.is_empty() {
                prefix_parts.push(env);
            }
        }
        if !server.group_name.trim().is_empty() {
            let group = normalize_authorization_segment(&server.group_name);
            if !group.is_empty() {
                prefix_parts.push(group);
            }
        }

        if !prefix_parts.is_empty() {
            candidates.push(format!("{}_{}", prefix_parts.join("-"), configured_group));
        }
    }

    candidates.dedup();
    Ok(candidates)
}

fn group_suffix_matches(entry_group: &str, configured_group: &str) -> bool {
    entry_group == configured_group || entry_group.ends_with(&format!("_{configured_group}"))
}

/// Select a Wallix menu entry directly from a resolved server configuration.
pub fn select_id_for_server(
    entries: &[WallixMenuEntry],
    server: &ResolvedServer,
) -> Result<String> {
    let targets = build_expected_targets(server);
    let groups = build_expected_groups(server)?;
    let configured_group = server.wallix_group.as_deref().ok_or_else(|| {
        anyhow!(
            "wallix.group is not configured for server '{}'",
            server.name
        )
    })?;

    let mut had_target_match = false;
    let mut available_groups_for_matching_targets: Vec<String> = Vec::new();

    for target in &targets {
        let target_entries: Vec<&WallixMenuEntry> =
            entries.iter().filter(|e| &e.target == target).collect();
        if target_entries.is_empty() {
            continue;
        }

        had_target_match = true;

        for entry in &target_entries {
            if !available_groups_for_matching_targets.contains(&entry.group) {
                available_groups_for_matching_targets.push(entry.group.clone());
            }
        }

        for group in &groups {
            let exact: Vec<_> = target_entries
                .iter()
                .filter(|entry| entry.group == *group)
                .collect();
            match exact.len() {
                1 => return Ok(exact[0].id.clone()),
                n if n > 1 => {
                    return Err(anyhow!(
                        "Multiple menu entries ({}) found for target '{}' and group '{}'. Cannot auto-select.",
                        n,
                        target,
                        group
                    ));
                }
                _ => {}
            }
        }

        let suffix_matches: Vec<_> = target_entries
            .iter()
            .filter(|entry| group_suffix_matches(&entry.group, configured_group))
            .collect();
        match suffix_matches.len() {
            1 => return Ok(suffix_matches[0].id.clone()),
            n if n > 1 => {
                return Err(anyhow!(
                    "Multiple menu entries ({}) found for target '{}' matching group suffix '{}'. Cannot auto-select.",
                    n,
                    target,
                    configured_group
                ));
            }
            _ => {}
        }
    }

    if had_target_match {
        return Err(anyhow!(
            "No menu entry found for matching targets with group '{}'. Available groups for these targets: {}",
            configured_group,
            available_groups_for_matching_targets.join(", ")
        ));
    }

    Err(anyhow!(
        "No menu entry found with target '{}'. Available targets: {}",
        targets
            .last()
            .cloned()
            .unwrap_or_else(|| build_expected_target(server)),
        entries
            .iter()
            .map(|e| e.target.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_suffix_matches_short_group() {
        assert!(group_suffix_matches("APP-ANSCORE_dev-admins", "dev-admins"));
    }

    #[test]
    fn test_group_suffix_matches_exact_group() {
        assert!(group_suffix_matches("dev-admins", "dev-admins"));
    }

    #[test]
    fn test_group_suffix_does_not_match_unrelated_group() {
        assert!(!group_suffix_matches(
            "APP-ANSCORE_ops-admins",
            "dev-admins"
        ));
    }

    #[test]
    fn test_parse_simple_menu() {
        let output = r#"
   ID │ Cible                              │ Autorisation
───────┼────────────────────────────────────┼──────────────────────
 1234  │ demo_user@default@APP-ALPHA-BD:SSH    │ APP-ALPHA_ops-admins
 5678  │ demo_user@default@APP-ALPHA-BD:SSH    │ APP-ALPHA_dev-admins
"#;
        let entries = parse_wallix_menu(output).expect("Should parse menu");
        assert_eq!(entries.len(), 2);

        assert_eq!(entries[0].id, "1234");
        assert_eq!(entries[0].target, "demo_user@default@APP-ALPHA-BD:SSH");
        assert_eq!(entries[0].group, "APP-ALPHA_ops-admins");

        assert_eq!(entries[1].id, "5678");
        assert_eq!(entries[1].group, "APP-ALPHA_dev-admins");
    }

    #[test]
    fn test_parse_menu_with_varied_whitespace() {
        let output = "  123   │   user@default@host:SSH   │   group-name  ";
        let entries = parse_wallix_menu(output).expect("Should parse despite whitespace");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "123");
        assert_eq!(entries[0].target, "user@default@host:SSH");
        assert_eq!(entries[0].group, "group-name");
    }

    #[test]
    fn test_parse_menu_with_leading_zeros() {
        let output = "0001  │ user@default@host:SSH  │ my-group";
        let entries = parse_wallix_menu(output).expect("Should preserve leading zeros");
        assert_eq!(entries[0].id, "0001");
    }

    #[test]
    fn test_parse_empty_output_returns_error() {
        let output = r#"
   ID │ Cible │ Autorisation
───────┼───────┼──────────────
"#;
        let result = parse_wallix_menu(output);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No valid menu entries")
        );
    }

    #[test]
    fn test_select_unique_target_and_group_match() {
        let entries = vec![
            WallixMenuEntry {
                id: "1234".to_string(),
                target: "user@default@host:SSH".to_string(),
                group: "admins".to_string(),
            },
            WallixMenuEntry {
                id: "5678".to_string(),
                target: "user@default@other:SSH".to_string(),
                group: "admins".to_string(),
            },
        ];

        let id = select_id_by_target_and_group(&entries, "user@default@host:SSH", "admins")
            .expect("Should find unique match");
        assert_eq!(id, "1234");
    }

    #[test]
    fn test_select_no_target_match() {
        let entries = vec![WallixMenuEntry {
            id: "1234".to_string(),
            target: "user@default@host:SSH".to_string(),
            group: "admins".to_string(),
        }];

        let result = select_id_by_target_and_group(&entries, "other@default@host:SSH", "admins");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No menu entry found with target")
        );
    }

    #[test]
    fn test_select_target_match_but_no_group_match() {
        let entries = vec![
            WallixMenuEntry {
                id: "1234".to_string(),
                target: "user@default@host:SSH".to_string(),
                group: "admins".to_string(),
            },
            WallixMenuEntry {
                id: "5678".to_string(),
                target: "user@default@host:SSH".to_string(),
                group: "operators".to_string(),
            },
        ];

        let result = select_id_by_target_and_group(&entries, "user@default@host:SSH", "users");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No menu entry found for target")
        );
    }

    #[test]
    fn test_select_multiple_matches_returns_error() {
        let entries = vec![
            WallixMenuEntry {
                id: "1234".to_string(),
                target: "user@default@host:SSH".to_string(),
                group: "admins".to_string(),
            },
            WallixMenuEntry {
                id: "5678".to_string(),
                target: "user@default@host:SSH".to_string(),
                group: "admins".to_string(),
            },
        ];

        let result = select_id_by_target_and_group(&entries, "user@default@host:SSH", "admins");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Multiple menu entries")
        );
    }

    #[test]
    fn test_build_expected_target_from_resolved_server() {
        let server = ResolvedServer {
            namespace: String::new(),
            group_name: String::new(),
            env_name: String::new(),
            name: "alpha-ops".to_string(),
            host: "APP-ALPHA-BD".to_string(),
            user: "demo_user".to_string(),
            port: 22,
            ssh_key: String::new(),
            ssh_options: vec![],
            default_mode: crate::config::ConnectionMode::Wallix,
            jump_host: None,
            bastion_host: Some("bastion.example.test".to_string()),
            bastion_user: Some("demo_user".to_string()),
            bastion_template: "{target_user}@%n:SSH:{bastion_user}".to_string(),
            wallix_group: Some("APP-ALPHA_ops-admins".to_string()),
            wallix_account: "default".to_string(),
            wallix_protocol: "SSH".to_string(),
            wallix_auto_select: true,
            wallix_fail_if_menu_match_error: true,
            wallix_selection_timeout_secs: 8,
            use_system_ssh_config: false,
            probe_filesystems: vec![],
            tunnels: vec![],
            tags: vec![],
            control_master: false,
            control_path: String::new(),
            control_persist: "10m".to_string(),
            pre_connect_hook: None,
            post_disconnect_hook: None,
            hook_timeout_secs: 5,
        };

        assert_eq!(
            build_expected_target(&server),
            "demo_user@default@APP-ALPHA-BD:SSH"
        );
    }

    #[test]
    fn test_select_id_for_server_uses_resolved_server_fields() {
        let entries = vec![WallixMenuEntry {
            id: "0".to_string(),
            target: "demo_user@default@APP-ALPHA-BD:SSH".to_string(),
            group: "APP-ALPHA_ops-admins".to_string(),
        }];

        let server = ResolvedServer {
            namespace: String::new(),
            group_name: String::new(),
            env_name: String::new(),
            name: "alpha-ops".to_string(),
            host: "APP-ALPHA-BD".to_string(),
            user: "demo_user".to_string(),
            port: 22,
            ssh_key: String::new(),
            ssh_options: vec![],
            default_mode: crate::config::ConnectionMode::Wallix,
            jump_host: None,
            bastion_host: Some("bastion.example.test".to_string()),
            bastion_user: Some("demo_user".to_string()),
            bastion_template: "{target_user}@%n:SSH:{bastion_user}".to_string(),
            wallix_group: Some("APP-ALPHA_ops-admins".to_string()),
            wallix_account: "default".to_string(),
            wallix_protocol: "SSH".to_string(),
            wallix_auto_select: true,
            wallix_fail_if_menu_match_error: true,
            wallix_selection_timeout_secs: 8,
            use_system_ssh_config: false,
            probe_filesystems: vec![],
            tunnels: vec![],
            tags: vec![],
            control_master: false,
            control_path: String::new(),
            control_persist: "10m".to_string(),
            pre_connect_hook: None,
            post_disconnect_hook: None,
            hook_timeout_secs: 5,
        };

        assert_eq!(select_id_for_server(&entries, &server).unwrap(), "0");
    }

    #[test]
    fn test_select_id_for_server_requires_group() {
        let entries = vec![WallixMenuEntry {
            id: "0".to_string(),
            target: "demo_user@default@APP-ALPHA-BD:SSH".to_string(),
            group: "APP-ALPHA_ops-admins".to_string(),
        }];

        let server = ResolvedServer {
            namespace: String::new(),
            group_name: String::new(),
            env_name: String::new(),
            name: "app-alpha-missing-group".to_string(),
            host: "APP-ALPHA-BD".to_string(),
            user: "demo_user".to_string(),
            port: 22,
            ssh_key: String::new(),
            ssh_options: vec![],
            default_mode: crate::config::ConnectionMode::Wallix,
            jump_host: None,
            bastion_host: Some("bastion.example.test".to_string()),
            bastion_user: Some("demo_user".to_string()),
            bastion_template: "{target_user}@%n:SSH:{bastion_user}".to_string(),
            wallix_group: None,
            wallix_account: "default".to_string(),
            wallix_protocol: "SSH".to_string(),
            wallix_auto_select: true,
            wallix_fail_if_menu_match_error: true,
            wallix_selection_timeout_secs: 8,
            use_system_ssh_config: false,
            probe_filesystems: vec![],
            tunnels: vec![],
            tags: vec![],
            control_master: false,
            control_path: String::new(),
            control_persist: "10m".to_string(),
            pre_connect_hook: None,
            post_disconnect_hook: None,
            hook_timeout_secs: 5,
        };

        let error = select_id_for_server(&entries, &server).unwrap_err();
        assert!(error.to_string().contains("wallix.group is not configured"));
    }

    #[test]
    fn test_build_expected_groups_adds_yaml_structure_prefix() {
        let server = ResolvedServer {
            namespace: String::new(),
            group_name: "ALPHA".to_string(),
            env_name: "PP".to_string(),
            name: "alpha-dev".to_string(),
            host: "APP-ALPHA-BD".to_string(),
            user: "demo_user".to_string(),
            port: 22,
            ssh_key: String::new(),
            ssh_options: vec![],
            default_mode: crate::config::ConnectionMode::Wallix,
            jump_host: None,
            bastion_host: Some("bastion.example.test".to_string()),
            bastion_user: Some("demo_user".to_string()),
            bastion_template: "{target_user}@%n:SSH:{bastion_user}".to_string(),
            wallix_group: Some("dev-admins".to_string()),
            wallix_account: "default".to_string(),
            wallix_protocol: "SSH".to_string(),
            wallix_auto_select: true,
            wallix_fail_if_menu_match_error: true,
            wallix_selection_timeout_secs: 8,
            use_system_ssh_config: false,
            probe_filesystems: vec![],
            tunnels: vec![],
            tags: vec![],
            control_master: false,
            control_path: String::new(),
            control_persist: "10m".to_string(),
            pre_connect_hook: None,
            post_disconnect_hook: None,
            hook_timeout_secs: 5,
        };

        let groups = build_expected_groups(&server).unwrap();
        assert_eq!(
            groups,
            vec!["dev-admins".to_string(), "PP-ALPHA_dev-admins".to_string()]
        );
    }

    #[test]
    fn test_build_expected_targets_adds_wallix_alias_from_fqdn_and_yaml() {
        let server = ResolvedServer {
            namespace: String::new(),
            group_name: "ALPHA".to_string(),
            env_name: "PP".to_string(),
            name: "bdd01".to_string(),
            host: "app-db01.alpha.example.test".to_string(),
            user: "demo_user".to_string(),
            port: 22,
            ssh_key: String::new(),
            ssh_options: vec![],
            default_mode: crate::config::ConnectionMode::Wallix,
            jump_host: None,
            bastion_host: Some("bastion.example.test".to_string()),
            bastion_user: Some("demo_user".to_string()),
            bastion_template: "{target_user}@%n:SSH:{bastion_user}".to_string(),
            wallix_group: Some("dev-admins".to_string()),
            wallix_account: "default".to_string(),
            wallix_protocol: "SSH".to_string(),
            wallix_auto_select: true,
            wallix_fail_if_menu_match_error: true,
            wallix_selection_timeout_secs: 8,
            use_system_ssh_config: false,
            probe_filesystems: vec![],
            tunnels: vec![],
            tags: vec![],
            control_master: false,
            control_path: String::new(),
            control_persist: "10m".to_string(),
            pre_connect_hook: None,
            post_disconnect_hook: None,
            hook_timeout_secs: 5,
        };

        let targets = build_expected_targets(&server);
        assert!(targets.contains(&"demo_user@default@app-db01.alpha.example.test:SSH".to_string()));
        assert!(targets.contains(&"demo_user@default@APP-DB01:SSH".to_string()));
        assert!(targets.contains(&"demo_user@default@PP-ALPHA-BD:SSH".to_string()));
    }

    #[test]
    fn test_select_id_for_server_accepts_short_group_and_yaml_structure() {
        let entries = vec![WallixMenuEntry {
            id: "1".to_string(),
            target: "demo_user@default@PP-ALPHA-BD:SSH".to_string(),
            group: "APP-ALPHA_dev-admins".to_string(),
        }];

        let server = ResolvedServer {
            namespace: String::new(),
            group_name: "ALPHA".to_string(),
            env_name: "PP".to_string(),
            name: "bdd01".to_string(),
            host: "app-db01.alpha.example.test".to_string(),
            user: "demo_user".to_string(),
            port: 22,
            ssh_key: String::new(),
            ssh_options: vec![],
            default_mode: crate::config::ConnectionMode::Wallix,
            jump_host: None,
            bastion_host: Some("bastion.example.test".to_string()),
            bastion_user: Some("demo_user".to_string()),
            bastion_template: "{target_user}@%n:SSH:{bastion_user}".to_string(),
            wallix_group: Some("dev-admins".to_string()),
            wallix_account: "default".to_string(),
            wallix_protocol: "SSH".to_string(),
            wallix_auto_select: true,
            wallix_fail_if_menu_match_error: true,
            wallix_selection_timeout_secs: 8,
            use_system_ssh_config: false,
            probe_filesystems: vec![],
            tunnels: vec![],
            tags: vec![],
            control_master: false,
            control_path: String::new(),
            control_persist: "10m".to_string(),
            pre_connect_hook: None,
            post_disconnect_hook: None,
            hook_timeout_secs: 5,
        };

        assert_eq!(select_id_for_server(&entries, &server).unwrap(), "1");
    }

    #[test]
    fn test_select_id_for_server_accepts_prefixed_group_suffix() {
        let entries = vec![WallixMenuEntry {
            id: "640".to_string(),
            target: "demo_user@default@app-anscore02.example.test:SSH".to_string(),
            group: "APP-ANSCORE_dev-admins".to_string(),
        }];

        let server = ResolvedServer {
            namespace: String::new(),
            group_name: "Service Beta".to_string(),
            env_name: String::new(),
            name: "Service Beta".to_string(),
            host: "app-anscore02.example.test".to_string(),
            user: "demo_user".to_string(),
            port: 22,
            ssh_key: String::new(),
            ssh_options: vec![],
            default_mode: crate::config::ConnectionMode::Wallix,
            jump_host: None,
            bastion_host: Some("bastion.example.test".to_string()),
            bastion_user: Some("demo_user".to_string()),
            bastion_template: "{target_user}@%n:SSH:{bastion_user}".to_string(),
            wallix_group: Some("dev-admins".to_string()),
            wallix_account: "default".to_string(),
            wallix_protocol: "SSH".to_string(),
            wallix_auto_select: true,
            wallix_fail_if_menu_match_error: true,
            wallix_selection_timeout_secs: 8,
            use_system_ssh_config: false,
            probe_filesystems: vec![],
            tunnels: vec![],
            tags: vec![],
            control_master: false,
            control_path: String::new(),
            control_persist: "10m".to_string(),
            pre_connect_hook: None,
            post_disconnect_hook: None,
            hook_timeout_secs: 5,
        };

        assert_eq!(select_id_for_server(&entries, &server).unwrap(), "640");
    }

    #[test]
    fn test_parse_realistic_wallix_output() {
        let output = r#"
╔════════════════════════════════════════════════════════════════════════════════╗
║                    Wallix Bastion - Interactive Menu                           ║
╚════════════════════════════════════════════════════════════════════════════════╝

   ID │ Cible                                │ Autorisation
───────┼──────────────────────────────────────┼────────────────────────
 0001  │ demo_user@default@APP-ALPHA-BD:SSH      │ APP-ALPHA_ops-admins
 0002  │ demo_user@default@APP-ALPHA-BD:SSH      │ APP-ALPHA_dev-admins
 0003  │ demo_user@default@OTHER-SERVER:SSH    │ APP-ALPHA_ops-admins
"#;
        let entries = parse_wallix_menu(output).expect("Should parse realistic output");
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].id, "0001");
        assert_eq!(entries[2].group, "APP-ALPHA_ops-admins");
    }
}
