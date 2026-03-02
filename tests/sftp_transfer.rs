/// Test d'intégration SFTP — nécessite un serveur SSH réel sur 192.168.1.13.
/// Lancer avec : cargo test --test sftp_transfer -- --nocapture --ignored
///
/// Le serveur cible doit être joignable et l'agent SSH ou ~/.ssh/id_ed25519 doivent
/// permettre une connexion root@192.168.1.13.
use std::time::Duration;
use susshi::config::{ConnectionMode, ResolvedServer};
use susshi::ssh::scp::{ScpDirection, ScpEvent};
use susshi::ssh::sftp::spawn_sftp;

fn test_server() -> ResolvedServer {
    ResolvedServer {
        namespace: String::new(),
        group_name: String::new(),
        env_name: String::new(),
        name: "test-sftp".into(),
        host: "192.168.1.13".into(),
        user: "root".into(),
        port: 22,
        ssh_key: String::new(), // agent SSH ou clés par défaut
        ssh_options: vec![],
        default_mode: ConnectionMode::Direct,
        jump_host: None,
        bastion_host: None,
        bastion_user: None,
        bastion_template: String::new(),
        use_system_ssh_config: false,
        probe_filesystems: vec![],
        tunnels: vec![],
    }
}

/// Upload /tmp/random-data.bin → root@192.168.1.13:/root/random-data.bin
/// Vérifie que tous les événements ScpEvent::Progress arrivent dans l'ordre
/// et que ScpEvent::Done(true) est bien reçu.
#[test]
#[ignore]
fn upload_random_data_to_192_168_1_13() {
    let local = "/tmp/random-data.bin";
    assert!(
        std::path::Path::new(local).exists(),
        "Fichier de test manquant : {local}\nCréer avec : dd if=/dev/urandom of={local} bs=1M count=2"
    );

    let server = test_server();
    let rx = spawn_sftp(
        &server,
        ConnectionMode::Direct,
        ScpDirection::Upload,
        local,
        "/root/random-data.bin",
    )
    .expect("spawn_sftp ne doit pas échouer immédiatement");

    let mut last_pct: i64 = -1;

    loop {
        match rx.recv_timeout(Duration::from_secs(30)) {
            Ok(ScpEvent::Progress(pct)) => {
                println!("Progress: {pct}%");
                assert!(
                    pct as i64 >= last_pct,
                    "progression régressive : {pct} < {last_pct}"
                );
                last_pct = pct as i64;
            }
            Ok(ScpEvent::Done(success)) => {
                println!("Done: success={success}");
                assert!(success, "transfert signalé échoué (Done(false))");
                break;
            }
            Ok(ScpEvent::Error(e)) => {
                panic!("ScpEvent::Error reçu : {e}");
            }
            Err(e) => {
                panic!("Timeout ou channel fermé : {e}");
            }
        }
    }

    assert_eq!(last_pct, 100, "progression n'a pas atteint 100%");
    println!("Upload OK — progression finale : {last_pct}%");
}
