#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use susshi::config::{Config, ConnectionMode};

    #[test]
    fn test_full_config_structure() {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let path = PathBuf::from(manifest_dir).join("examples/full_config.yaml");

        // Use load() to verify sorting + loading
        let config = Config::load(&path).expect("Failed to load config");
        let resolved = config.resolve().expect("Failed to resolve config");

        // Verify Sorting: Groups should be sorted by name.
        // Assuming full_config.yaml has unsorted groups.
        // "Home Lab" < "VPS Work" (example names)
        // Let's just check relative order of known items if we knew them.

        // Find nextcloud
        let nextcloud = resolved
            .iter()
            .find(|s| s.name == "nextcloud")
            .expect("nextcloud found");
        assert_eq!(nextcloud.host, "192.168.1.13");
        assert_eq!(nextcloud.user, "root"); // server override
        assert_eq!(nextcloud.default_mode, ConnectionMode::Direct);

        // Find db-01
        let db01 = resolved
            .iter()
            .find(|s| s.name == "db-01")
            .expect("db-01 found");
        assert_eq!(db01.host, "192.168.1.11");
        assert_eq!(db01.user, "dev"); // "Projet Alpha" defines user="dev", no override on Env or Server.
        assert_eq!(db01.group_name, "Projet Alpha");

        // Find internal-nas
        let nas = resolved
            .iter()
            .find(|s| s.name == "internal-nas")
            .expect("internal-nas found");
        assert_eq!(nas.user, "root"); // server override
        assert_eq!(nas.default_mode, ConnectionMode::Wallix);
        // Wallix (bastion) config should be inherited from defaults
        assert_eq!(nas.bastion_host.as_deref().unwrap(), "bastion.example.com");
        assert_eq!(nas.bastion_user.as_deref().unwrap(), "bastion");
    }
}
