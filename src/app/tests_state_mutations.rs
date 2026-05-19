use super::tests_helpers::make_namespace_config;
use super::*;
use crate::config::{ConfigEntry, Group, Server};

fn make_simple_config() -> Config {
    Config {
        defaults: None,
        includes: vec![],
        groups: vec![ConfigEntry::Group(Group {
            name: "Prod".to_string(),
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
                name: "web-01".to_string(),
                host: "203.0.113.10".to_string(),
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
        vars: Default::default(),
    }
}

// ── expansion_state ──────────────────────────────────────────────────────────

#[test]
fn collapse_all_clears_expanded_and_resets_selection() {
    let config = make_simple_config();
    let mut app = App::new(config, vec![], std::path::PathBuf::from("/fake"), vec![]).unwrap();

    // Expand "Prod" group
    app.expanded_items.insert("Group:Prod".to_string());
    app.selected_index = 2;

    app.collapse_all();

    assert!(app.expanded_items.is_empty());
    assert_eq!(app.selected_index, 0);
}

#[test]
fn collapse_all_marks_items_dirty() {
    let config = make_simple_config();
    let mut app = App::new(config, vec![], std::path::PathBuf::from("/fake"), vec![]).unwrap();

    app.items_dirty = false;
    app.collapse_all();

    assert!(app.items_dirty);
}

#[test]
fn invalidate_cache_sets_dirty() {
    let config = make_simple_config();
    let mut app = App::new(config, vec![], std::path::PathBuf::from("/fake"), vec![]).unwrap();

    app.items_dirty = false;
    app.invalidate_cache();

    assert!(app.items_dirty);
}

// ── favorites ────────────────────────────────────────────────────────────────

#[test]
fn toggle_favorites_view_flips_flag_and_resets_index() {
    let config = make_namespace_config();
    let mut app = App::new(config, vec![], std::path::PathBuf::from("/fake"), vec![]).unwrap();

    assert!(!app.favorites_only);
    app.toggle_favorites_view();
    assert!(app.favorites_only);

    app.selected_index = 3;
    app.toggle_favorites_view();
    assert!(!app.favorites_only);
    assert_eq!(app.selected_index, 0);
}

#[test]
fn toggle_favorites_view_marks_items_dirty() {
    let config = make_simple_config();
    let mut app = App::new(config, vec![], std::path::PathBuf::from("/fake"), vec![]).unwrap();

    app.items_dirty = false;
    app.toggle_favorites_view();
    assert!(app.items_dirty);
}

#[test]
fn record_connection_stores_timestamp() {
    let config = make_simple_config();
    let mut app = App::new(config, vec![], std::path::PathBuf::from("/fake"), vec![]).unwrap();

    let server = app.resolved_servers.first().cloned().unwrap();
    assert!(app.last_seen_for(&server).is_none());

    app.record_connection(&server);

    assert!(app.last_seen_for(&server).is_some());
}

#[test]
fn record_connection_timestamp_is_recent() {
    let config = make_simple_config();
    let mut app = App::new(config, vec![], std::path::PathBuf::from("/fake"), vec![]).unwrap();

    let server = app.resolved_servers.first().cloned().unwrap();
    app.record_connection(&server);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let ts = app.last_seen_for(&server).unwrap();

    assert!(ts <= now);
    assert!(now - ts < 5);
}
