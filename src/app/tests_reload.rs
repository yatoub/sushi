use super::*;

#[test]
fn test_reload_detects_new_host_in_included_file() {
    use std::fs;

    let temp = tempfile::tempdir().unwrap();
    let main_path = temp.path().join("main.yaml");
    let include_path = temp.path().join("included.yaml");

    fs::write(
        &include_path,
        "groups:\n  - name: \"IncGroup\"\n    servers:\n      - name: \"inc-1\"\n        host: \"198.51.100.101\"\n",
    )
    .unwrap();

    fs::write(
        &main_path,
        "groups: []\nincludes:\n  - label: \"Included\"\n    path: \"included.yaml\"\n",
    )
    .unwrap();

    let (config, warnings, validation_warnings) =
        Config::load_merged(&main_path, &mut std::collections::HashSet::new()).unwrap();
    assert!(warnings.is_empty());
    assert!(validation_warnings.is_empty());

    let mut app =
        App::new(config, vec![], main_path.clone(), vec![]).expect("app init should work");
    assert_eq!(app.resolved_servers.len(), 1);

    fs::write(
        &include_path,
        "groups:\n  - name: \"IncGroup\"\n    servers:\n      - name: \"inc-1\"\n        host: \"198.51.100.101\"\n      - name: \"inc-2\"\n        host: \"198.51.100.102\"\n",
    )
    .unwrap();

    app.reload().expect("reload should succeed");

    assert_eq!(app.resolved_servers.len(), 2);
    assert!(app.resolved_servers.iter().any(|s| s.name == "inc-2"));
}
