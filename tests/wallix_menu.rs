use susshi::config::{ConnectionMode, ResolvedServer};
use susshi::wallix::{build_expected_target, parse_wallix_menu, select_id_for_server};

fn wallix_server(group: Option<&str>) -> ResolvedServer {
    ResolvedServer {
        namespace: String::new(),
        group_name: "ONDE-BD".to_string(),
        env_name: String::new(),
        name: "pp-ond".to_string(),
        host: "PP-ONDE-BD".to_string(),
        user: "pcollin".to_string(),
        port: 22,
        ssh_key: String::new(),
        ssh_options: vec![],
        default_mode: ConnectionMode::Wallix,
        jump_host: None,
        bastion_host: Some("ssh.in.phm.education.gouv.fr".to_string()),
        bastion_user: Some("pcollin".to_string()),
        bastion_template: "{target_user}@%n:SSH:{bastion_user}".to_string(),
        wallix_group: group.map(str::to_string),
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
    }
}

#[test]
fn builds_target_from_resolved_config() {
    let server = wallix_server(Some("PP-ONDE_ces3s-admins"));
    assert_eq!(build_expected_target(&server), "pcollin@default@PP-ONDE-BD:SSH");
}

#[test]
fn selects_expected_id_from_realistic_menu_fixture() {
    let output = r#"
Warning: Permanently added 'ssh.in.phm.education.gouv.fr' (ECDSA) to the list of known hosts.

| ID | Cible (page 1/1)               | Autorisation
|----|--------------------------------|-----------------------
|  0 | pcollin@default@PP-ONDE-BD:SSH | PP-ONDE_ces3s-admins
|  1 | pcollin@default@PP-ONDE-BD:SSH | PP-ONDE_crtech-admins
Tapez h pour l'aide, ctrl-D pour quitter
 >
"#;

    let entries = parse_wallix_menu(output).unwrap();
    let server = wallix_server(Some("PP-ONDE_crtech-admins"));

    assert_eq!(select_id_for_server(&entries, &server).unwrap(), "1");
}

#[test]
fn errors_when_group_is_missing_from_server_config() {
    let output = r#"
| ID | Cible (page 1/1)               | Autorisation
|----|--------------------------------|-----------------------
|  0 | pcollin@default@PP-ONDE-BD:SSH | PP-ONDE_ces3s-admins
"#;

    let entries = parse_wallix_menu(output).unwrap();
    let server = wallix_server(None);
    let error = select_id_for_server(&entries, &server).unwrap_err();

    assert!(error.to_string().contains("wallix.group is not configured"));
}
