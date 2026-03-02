#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use susshi::config::{Config, ConnectionMode};

    fn load() -> Vec<susshi::config::ResolvedServer> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let path = PathBuf::from(manifest_dir).join("examples/full_config.yaml");
        let config = Config::load(&path).expect("Failed to load config");
        config.resolve().expect("Failed to resolve config")
    }

    #[test]
    fn test_full_config_structure() {
        let resolved = load();

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

    // ── Tunnels ───────────────────────────────────────────────────────────────

    #[test]
    fn db01_has_two_tunnels() {
        let resolved = load();
        let db01 = resolved.iter().find(|s| s.name == "db-01").unwrap();
        assert_eq!(
            db01.tunnels.len(),
            2,
            "db-01 doit avoir 2 tunnels configurés"
        );
    }

    #[test]
    fn db01_tunnel_postgresql() {
        let resolved = load();
        let db01 = resolved.iter().find(|s| s.name == "db-01").unwrap();
        let pg = db01
            .tunnels
            .iter()
            .find(|t| t.label == "PostgreSQL")
            .expect("tunnel PostgreSQL manquant");
        assert_eq!(pg.local_port, 5432);
        assert_eq!(pg.remote_host, "127.0.0.1");
        assert_eq!(pg.remote_port, 5432);
    }

    #[test]
    fn db01_tunnel_redis() {
        let resolved = load();
        let db01 = resolved.iter().find(|s| s.name == "db-01").unwrap();
        let redis = db01
            .tunnels
            .iter()
            .find(|t| t.label == "Redis")
            .expect("tunnel Redis manquant");
        assert_eq!(redis.local_port, 6379);
        assert_eq!(redis.remote_host, "127.0.0.1");
        assert_eq!(redis.remote_port, 6379);
    }

    #[test]
    fn server_without_tunnels_has_empty_list() {
        let resolved = load();
        // nextcloud n'a pas de tunnels définis → liste vide
        let nextcloud = resolved.iter().find(|s| s.name == "nextcloud").unwrap();
        assert!(
            nextcloud.tunnels.is_empty(),
            "nextcloud ne doit pas hériter de tunnels"
        );
    }

    #[test]
    fn wallix_server_has_no_tunnels_defined() {
        let resolved = load();
        // internal-nas est en mode Wallix : pas de tunnels dans la config
        let nas = resolved.iter().find(|s| s.name == "internal-nas").unwrap();
        assert!(
            nas.tunnels.is_empty(),
            "internal-nas (Wallix) ne doit pas avoir de tunnels"
        );
    }
}
