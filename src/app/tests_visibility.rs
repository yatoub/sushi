use super::tests_helpers::make_namespace_config;
use super::*;
use crate::config::{ConfigEntry, Environment, Group, Server};

fn create_test_config() -> Config {
    Config {
        defaults: None,
        includes: vec![],
        groups: vec![ConfigEntry::Group(Group {
            name: "G1".to_string(),
            user: None,
            ssh_key: None,
            mode: None,
            ssh_port: None,
            ssh_options: None,
            wallix: None,
            wallix_group: None,
            jump: None,
            probe_filesystems: None,
            environments: Some(vec![Environment {
                name: "E1".to_string(),
                user: None,
                ssh_key: None,
                mode: None,
                ssh_port: None,
                ssh_options: None,
                wallix: None,
                wallix_group: None,
                jump: None,
                probe_filesystems: None,
                tunnels: None,
                tags: None,
                servers: vec![Server {
                    name: "S1".to_string(),
                    host: "198.51.100.1".to_string(),
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
                }],
            }]),
            servers: Some(vec![Server {
                name: "S2".to_string(),
                host: "198.51.100.2".to_string(),
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
            tunnels: None,
            tags: None,
        })],
        vars: Default::default(),
    }
}

#[test]
fn test_initial_visibility() {
    let config = create_test_config();
    let mut app = App::new(config, vec![], std::path::PathBuf::new(), vec![]).unwrap();
    let items = app.get_visible_items();

    assert_eq!(items.len(), 1);
    match &items[0] {
        ConfigItem::Group(name, _ns) => assert_eq!(name, "G1"),
        _ => panic!("Expected Group G1"),
    }
}

#[test]
fn test_expansion() {
    let config = create_test_config();
    let mut app = App::new(config, vec![], std::path::PathBuf::new(), vec![]).unwrap();

    app.toggle_expansion();
    let items = app.get_visible_items();

    assert_eq!(items.len(), 3);

    match &items[1] {
        ConfigItem::Environment(g, e, _ns) => {
            assert_eq!(g, "G1");
            assert_eq!(e, "E1");
        }
        _ => panic!("Expected Environment E1"),
    }

    match &items[2] {
        ConfigItem::Server(s) => assert_eq!(s.name, "S2"),
        _ => panic!("Expected Server S2"),
    }
}

#[test]
fn test_collapse_all() {
    let config = create_test_config();
    let mut app = App::new(config, vec![], std::path::PathBuf::new(), vec![]).unwrap();

    app.toggle_expansion();
    app.selected_index = 1;
    app.items_dirty = true;
    app.toggle_expansion();

    assert!(!app.expanded_items.is_empty());

    app.collapse_all();

    assert!(app.expanded_items.is_empty());
    assert_eq!(app.selected_index, 0);
    let items = app.get_visible_items();
    assert_eq!(items.len(), 1);
}

#[test]
fn test_search_filtering() {
    let config = create_test_config();
    let mut app = App::new(config, vec![], std::path::PathBuf::new(), vec![]).unwrap();

    app.search_query = "S1".to_string();
    app.invalidate_cache();
    let items = app.get_visible_items();

    assert!(items.len() >= 3);

    let has_s1 = items.iter().any(|i| match i {
        ConfigItem::Server(s) => s.name == "S1",
        _ => false,
    });
    assert!(has_s1, "Should contain S1");

    let has_s2 = items.iter().any(|i| match i {
        ConfigItem::Server(s) => s.name == "S2",
        _ => false,
    });
    assert!(!has_s2, "Should NOT contain S2");
}

#[test]
fn test_namespace_visibility_collapsed() {
    let config = make_namespace_config();
    let mut app = App::new(config, vec![], std::path::PathBuf::new(), vec![]).unwrap();
    app.expanded_items.clear();
    app.invalidate_cache();
    let items = app.get_visible_items();

    assert_eq!(items.len(), 2);
    assert!(matches!(
        &items[0],
        ConfigItem::Group(name, ns) if name == "RootGroup" && ns.is_empty()
    ));
    assert!(matches!(&items[1], ConfigItem::Namespace(label) if label == "CES"));
}

#[test]
fn test_namespace_expansion() {
    let config = make_namespace_config();
    let mut app = App::new(config, vec![], std::path::PathBuf::new(), vec![]).unwrap();
    app.expanded_items.clear();
    app.invalidate_cache();

    app.select(1);
    app.toggle_expansion();

    let items = app.get_visible_items();

    assert_eq!(items.len(), 3);
    assert!(matches!(
        &items[2],
        ConfigItem::Group(name, ns) if name == "CES_Group" && ns == "CES"
    ));
}

#[test]
fn test_search_crosses_namespaces() {
    let config = make_namespace_config();
    let mut app = App::new(config, vec![], std::path::PathBuf::new(), vec![]).unwrap();

    app.search_query = "ces_srv".to_string();
    app.invalidate_cache();
    let items = app.get_visible_items();

    let has_ces = items.iter().any(|i| match i {
        ConfigItem::Server(s) => s.name == "ces_srv",
        _ => false,
    });
    assert!(has_ces, "Search should find ces_srv in namespace CES");

    let has_root = items.iter().any(|i| match i {
        ConfigItem::Server(s) => s.name == "root_srv",
        _ => false,
    });
    assert!(!has_root, "root_srv should be filtered out");
}
