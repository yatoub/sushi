//! Wallix menu parser and selection logic.
//!
//! This module provides utilities to parse Wallix's interactive menu output
//! and automatically select an entry based on target (user@account@host:protocol)
//! and group matching.

use crate::config::ResolvedServer;
use anyhow::{anyhow, Result};

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

/// Parse Wallix menu output into structured entries.
///
/// The Wallix menu typically looks like:
/// ```text
///    ID │ Cible                              │ Autorisation
/// ───────┼────────────────────────────────────┼──────────────────────
///  1234  │ pcollin@default@PP-ONDE-BD:SSH    │ PP-ONDE_ces3s-admins
///  5678  │ pcollin@default@PP-ONDE-BD:SSH    │ PP-ONDE_crtech-admins
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
            || trimmed.chars().all(|c| matches!(c, '─' | '┼' | '│' | '-' | '+'))
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
    // Filter by target first
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

    // Filter by group
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
            n, target, group
        )),
    }
}

/// Select a Wallix menu entry directly from a resolved server configuration.
pub fn select_id_for_server(entries: &[WallixMenuEntry], server: &ResolvedServer) -> Result<String> {
    let group = server.wallix_group.as_deref().ok_or_else(|| {
        anyhow!(
            "wallix.group is not configured for server '{}'",
            server.name
        )
    })?;

    let target = build_expected_target(server);
    select_id_by_target_and_group(entries, &target, group)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_menu() {
        let output = r#"
   ID │ Cible                              │ Autorisation
───────┼────────────────────────────────────┼──────────────────────
 1234  │ pcollin@default@PP-ONDE-BD:SSH    │ PP-ONDE_ces3s-admins
 5678  │ pcollin@default@PP-ONDE-BD:SSH    │ PP-ONDE_crtech-admins
"#;
        let entries = parse_wallix_menu(output).expect("Should parse menu");
        assert_eq!(entries.len(), 2);

        assert_eq!(entries[0].id, "1234");
        assert_eq!(entries[0].target, "pcollin@default@PP-ONDE-BD:SSH");
        assert_eq!(entries[0].group, "PP-ONDE_ces3s-admins");

        assert_eq!(entries[1].id, "5678");
        assert_eq!(entries[1].group, "PP-ONDE_crtech-admins");
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
        assert!(result.unwrap_err().to_string().contains("No valid menu entries"));
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
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No menu entry found with target"));
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
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No menu entry found for target"));
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
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Multiple menu entries"));
    }

    #[test]
    fn test_build_expected_target_from_resolved_server() {
        let server = ResolvedServer {
            namespace: String::new(),
            group_name: String::new(),
            env_name: String::new(),
            name: "pp-ond-ces3s".to_string(),
            host: "PP-ONDE-BD".to_string(),
            user: "pcollin".to_string(),
            port: 22,
            ssh_key: String::new(),
            ssh_options: vec![],
            default_mode: crate::config::ConnectionMode::Wallix,
            jump_host: None,
            bastion_host: Some("ssh.in.phm.education.gouv.fr".to_string()),
            bastion_user: Some("pcollin".to_string()),
            bastion_template: "{target_user}@%n:SSH:{bastion_user}".to_string(),
            wallix_group: Some("PP-ONDE_ces3s-admins".to_string()),
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

        assert_eq!(build_expected_target(&server), "pcollin@default@PP-ONDE-BD:SSH");
    }

    #[test]
    fn test_select_id_for_server_uses_resolved_server_fields() {
        let entries = vec![WallixMenuEntry {
            id: "0".to_string(),
            target: "pcollin@default@PP-ONDE-BD:SSH".to_string(),
            group: "PP-ONDE_ces3s-admins".to_string(),
        }];

        let server = ResolvedServer {
            namespace: String::new(),
            group_name: String::new(),
            env_name: String::new(),
            name: "pp-ond-ces3s".to_string(),
            host: "PP-ONDE-BD".to_string(),
            user: "pcollin".to_string(),
            port: 22,
            ssh_key: String::new(),
            ssh_options: vec![],
            default_mode: crate::config::ConnectionMode::Wallix,
            jump_host: None,
            bastion_host: Some("ssh.in.phm.education.gouv.fr".to_string()),
            bastion_user: Some("pcollin".to_string()),
            bastion_template: "{target_user}@%n:SSH:{bastion_user}".to_string(),
            wallix_group: Some("PP-ONDE_ces3s-admins".to_string()),
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
            target: "pcollin@default@PP-ONDE-BD:SSH".to_string(),
            group: "PP-ONDE_ces3s-admins".to_string(),
        }];

        let server = ResolvedServer {
            namespace: String::new(),
            group_name: String::new(),
            env_name: String::new(),
            name: "pp-ond-missing-group".to_string(),
            host: "PP-ONDE-BD".to_string(),
            user: "pcollin".to_string(),
            port: 22,
            ssh_key: String::new(),
            ssh_options: vec![],
            default_mode: crate::config::ConnectionMode::Wallix,
            jump_host: None,
            bastion_host: Some("ssh.in.phm.education.gouv.fr".to_string()),
            bastion_user: Some("pcollin".to_string()),
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
    fn test_parse_realistic_wallix_output() {
        let output = r#"
╔════════════════════════════════════════════════════════════════════════════════╗
║                    Wallix Bastion - Interactive Menu                           ║
╚════════════════════════════════════════════════════════════════════════════════╝

   ID │ Cible                                │ Autorisation
───────┼──────────────────────────────────────┼────────────────────────
 0001  │ pcollin@default@PP-ONDE-BD:SSH      │ PP-ONDE_ces3s-admins
 0002  │ pcollin@default@PP-ONDE-BD:SSH      │ PP-ONDE_crtech-admins
 0003  │ pcollin@default@OTHER-SERVER:SSH    │ PP-ONDE_ces3s-admins
"#;
        let entries = parse_wallix_menu(output).expect("Should parse realistic output");
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].id, "0001");
        assert_eq!(entries[2].group, "PP-ONDE_ces3s-admins");
    }
}
