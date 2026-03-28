use super::tests_helpers::make_namespace_config;
use super::*;

fn wallix_test_server(group: Option<&str>) -> ResolvedServer {
    ResolvedServer {
        namespace: String::new(),
        group_name: "ALPHA-BD".to_string(),
        env_name: String::new(),
        name: "app-alpha".to_string(),
        host: "APP-ALPHA-BD".to_string(),
        user: "demo_user".to_string(),
        port: 22,
        ssh_key: String::new(),
        ssh_options: vec![],
        default_mode: ConnectionMode::Wallix,
        jump_host: None,
        bastion_host: Some("bastion.example.test".to_string()),
        bastion_user: Some("demo_user".to_string()),
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
fn wallix_selector_required_when_auto_select_disabled() {
    let mut app = App::new(
        make_namespace_config(),
        vec![],
        std::path::PathBuf::new(),
        vec![],
    )
    .unwrap();
    app.connection_mode = ConnectionMode::Wallix;

    let mut server = app.resolved_servers[0].clone();
    server.wallix_auto_select = false;
    server.wallix_fail_if_menu_match_error = true;

    assert!(app.should_open_wallix_selector(&server));
}

#[test]
fn wallix_selector_required_when_auto_select_enabled() {
    let mut app = App::new(
        make_namespace_config(),
        vec![],
        std::path::PathBuf::new(),
        vec![],
    )
    .unwrap();
    app.connection_mode = ConnectionMode::Wallix;

    let mut server = app.resolved_servers[0].clone();
    server.wallix_auto_select = true;
    server.wallix_fail_if_menu_match_error = false;

    assert!(app.should_open_wallix_selector(&server));
}

#[test]
fn wallix_poll_auto_resolves_to_pending_connection() {
    let mut app = App::new(
        make_namespace_config(),
        vec![],
        std::path::PathBuf::new(),
        vec![],
    )
    .unwrap();

    let server = wallix_test_server(Some("dev-admins"));
    let entries = vec![WallixMenuEntry {
        id: "42".to_string(),
        target: "demo_user@default@APP-ALPHA-BD:SSH".to_string(),
        group: "APP-ALPHA_dev-admins".to_string(),
    }];

    let (tx, rx) = std::sync::mpsc::channel();
    app.wallix_selector_rx = Some(rx);
    tx.send((server.clone(), Ok(entries))).unwrap();

    app.poll_wallix_selector();

    assert!(app.wallix_selector.is_none());
    let pending = app.take_pending_wallix_connection();
    assert!(pending.is_some());
    let (_, selected_id) = pending.unwrap();
    assert_eq!(selected_id, "42");
    assert_eq!(
        app.wallix_selection_cache.get(&App::server_key(&server)),
        Some(&"42".to_string())
    );
}

#[test]
fn wallix_poll_ambiguous_resolution_opens_targeted_selector() {
    let mut app = App::new(
        make_namespace_config(),
        vec![],
        std::path::PathBuf::new(),
        vec![],
    )
    .unwrap();

    let server = wallix_test_server(Some("dev-admins"));
    let entries = vec![
        WallixMenuEntry {
            id: "11".to_string(),
            target: "demo_user@default@APP-ALPHA-BD:SSH".to_string(),
            group: "APP-ALPHA_dev-admins".to_string(),
        },
        WallixMenuEntry {
            id: "12".to_string(),
            target: "demo_user@default@APP-ALPHA-BD:SSH".to_string(),
            group: "APP-ALPHA_dev-admins".to_string(),
        },
        WallixMenuEntry {
            id: "99".to_string(),
            target: "demo_user@default@OTHER:SSH".to_string(),
            group: "OTHER_dev-admins".to_string(),
        },
    ];

    let (tx, rx) = std::sync::mpsc::channel();
    app.wallix_selector_rx = Some(rx);
    tx.send((server, Ok(entries))).unwrap();

    app.poll_wallix_selector();

    assert!(app.take_pending_wallix_connection().is_none());
    match &app.wallix_selector {
        Some(WallixSelectorState::List { entries, .. }) => {
            assert_eq!(entries.len(), 2);
            assert!(
                entries
                    .iter()
                    .all(|entry| entry.target == "demo_user@default@APP-ALPHA-BD:SSH")
            );
        }
        _ => panic!("expected Wallix selector list"),
    }
}

#[test]
fn wallix_poll_missing_group_opens_targeted_selector() {
    let mut app = App::new(
        make_namespace_config(),
        vec![],
        std::path::PathBuf::new(),
        vec![],
    )
    .unwrap();

    let server = wallix_test_server(None);
    let entries = vec![
        WallixMenuEntry {
            id: "21".to_string(),
            target: "demo_user@default@APP-ALPHA-BD:SSH".to_string(),
            group: "APP-ALPHA_ops-admins".to_string(),
        },
        WallixMenuEntry {
            id: "22".to_string(),
            target: "demo_user@default@APP-ALPHA-BD:SSH".to_string(),
            group: "APP-ALPHA_dev-admins".to_string(),
        },
    ];

    let (tx, rx) = std::sync::mpsc::channel();
    app.wallix_selector_rx = Some(rx);
    tx.send((server, Ok(entries))).unwrap();

    app.poll_wallix_selector();

    assert!(app.take_pending_wallix_connection().is_none());
    assert!(matches!(
        app.wallix_selector,
        Some(WallixSelectorState::List { .. })
    ));
}

#[test]
fn wallix_poll_uses_cached_selection_when_available() {
    let mut app = App::new(
        make_namespace_config(),
        vec![],
        std::path::PathBuf::new(),
        vec![],
    )
    .unwrap();

    let server = wallix_test_server(Some("dev-admins"));
    app.wallix_selection_cache
        .insert(App::server_key(&server), "77".to_string());

    let entries = vec![
        WallixMenuEntry {
            id: "77".to_string(),
            target: "demo_user@default@APP-ALPHA-BD:SSH".to_string(),
            group: "APP-ALPHA_ops-admins".to_string(),
        },
        WallixMenuEntry {
            id: "78".to_string(),
            target: "demo_user@default@APP-ALPHA-BD:SSH".to_string(),
            group: "APP-ALPHA_dev-admins".to_string(),
        },
    ];

    let (tx, rx) = std::sync::mpsc::channel();
    app.wallix_selector_rx = Some(rx);
    tx.send((server, Ok(entries))).unwrap();

    app.poll_wallix_selector();

    assert!(app.wallix_selector.is_none());
    let pending = app.take_pending_wallix_connection();
    assert!(pending.is_some());
    let (_, selected_id) = pending.unwrap();
    assert_eq!(selected_id, "77");
}

#[test]
fn targeted_wallix_entries_keeps_matching_target_when_available() {
    let server = wallix_test_server(Some("dev-admins"));

    let entries = vec![
        WallixMenuEntry {
            id: "1".to_string(),
            target: "demo_user@default@OTHER:SSH".to_string(),
            group: "OTHER_dev-admins".to_string(),
        },
        WallixMenuEntry {
            id: "2".to_string(),
            target: "demo_user@default@APP-ALPHA-BD:SSH".to_string(),
            group: "APP-ALPHA_dev-admins".to_string(),
        },
    ];

    let filtered = wallix_state::targeted_wallix_entries(&server, &entries);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, "2");
}
