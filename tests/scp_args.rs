//! Tests d'intégration pour `ssh::scp::build_scp_args`.
//!
//! Vérifie que les arguments `scp` sont correctement construits pour les
//! différents modes (Direct, Jump, Wallix) et directions (Upload, Download).
//! Aucun processus `scp` n'est lancé — les tests sont purement en mémoire.

use susshi::config::{ConnectionMode, ResolvedServer};
use susshi::ssh::scp::{ScpDirection, build_scp_args};

// ─── Helper ──────────────────────────────────────────────────────────────────

fn base_server() -> ResolvedServer {
    ResolvedServer {
        namespace: String::new(),
        group_name: "integration".into(),
        env_name: "test".into(),
        name: "srv".into(),
        host: "192.168.1.10".into(),
        user: "ops".into(),
        port: 22,
        ssh_key: String::new(),
        ssh_options: vec![],
        default_mode: ConnectionMode::Direct,
        jump_host: None,
        bastion_host: None,
        bastion_user: None,
        bastion_template: "{target_user}@%n:SSH:{bastion_user}".into(),
        use_system_ssh_config: false,
        probe_filesystems: vec![],
        tunnels: vec![],
    }
}

// ─── Upload ───────────────────────────────────────────────────────────────────

/// Upload direct : le fichier local est la source et la destination distante est en dernier.
#[test]
fn upload_direct() {
    let s = base_server();
    let args = build_scp_args(
        &s,
        ConnectionMode::Direct,
        &ScpDirection::Upload,
        "/tmp/dump.sql",
        "/home/ops/dump.sql",
    )
    .unwrap();

    // En upload : args = [..., local_src, remote_dst]
    assert_eq!(
        args.last().unwrap(),
        "ops@192.168.1.10:/home/ops/dump.sql",
        "destination distante doit être en dernier"
    );
    assert_eq!(
        args[args.len() - 2],
        "/tmp/dump.sql",
        "source locale doit précéder la destination"
    );
}

// ─── Download ─────────────────────────────────────────────────────────────────

/// Download direct : la source distante est en avant-dernier, la destination locale en dernier.
#[test]
fn download_direct() {
    let s = base_server();
    let args = build_scp_args(
        &s,
        ConnectionMode::Direct,
        &ScpDirection::Download,
        "/tmp/dump.sql",
        "/home/ops/dump.sql",
    )
    .unwrap();

    // En download : args = [..., remote_src, local_dst]
    assert_eq!(
        args.last().unwrap(),
        "/tmp/dump.sql",
        "destination locale doit être en dernier"
    );
    assert_eq!(
        args[args.len() - 2],
        "ops@192.168.1.10:/home/ops/dump.sql",
        "source distante doit précéder la destination locale"
    );
}

// ─── Jump ─────────────────────────────────────────────────────────────────────

/// Mode Jump : `-J user@host` transmis à `scp` pour le forward.
#[test]
fn jump_host_forwarded() {
    let mut s = base_server();
    s.jump_host = Some("jops@jump.infra.example.com".into());
    let args = build_scp_args(
        &s,
        ConnectionMode::Jump,
        &ScpDirection::Upload,
        "file.txt",
        "/tmp/file.txt",
    )
    .unwrap();

    let j_pos = args.iter().position(|a| a == "-J").expect("-J attendu");
    assert_eq!(args[j_pos + 1], "jops@jump.infra.example.com");
    // La destination distante reste en dernier
    assert!(
        args.last().unwrap().starts_with("ops@192.168.1.10:"),
        "destination distante incorrecte"
    );
}

/// Mode Jump sans `jump_host` configuré → erreur explicite.
#[test]
fn jump_missing_host_returns_error() {
    let s = base_server(); // jump_host = None
    let err = build_scp_args(
        &s,
        ConnectionMode::Jump,
        &ScpDirection::Upload,
        "f",
        "r",
    )
    .unwrap_err();
    assert!(
        err.to_string().contains("Jump host"),
        "message d'erreur attendu, obtenu : {}",
        err
    );
}

// ─── Wallix ───────────────────────────────────────────────────────────────────

/// Mode Wallix est systématiquement refusé (le bastion Wallix gère les transferts lui-même).
#[test]
fn wallix_disabled() {
    let s = base_server();
    let err = build_scp_args(
        &s,
        ConnectionMode::Wallix,
        &ScpDirection::Upload,
        "file.txt",
        "/tmp/file.txt",
    )
    .unwrap_err();
    assert!(
        err.to_string().contains("Wallix"),
        "erreur Wallix attendue, obtenu : {}",
        err
    );
}

// ─── Port ─────────────────────────────────────────────────────────────────────

/// `scp` utilise `-P` (majuscule) pour le port, contrairement à `ssh` qui utilise `-p`.
#[test]
fn port_flag_is_uppercase_p() {
    let mut s = base_server();
    s.port = 2222;
    let args =
        build_scp_args(&s, ConnectionMode::Direct, &ScpDirection::Upload, "f", "r").unwrap();
    assert!(args.contains(&"-P".to_string()), "-P (majuscule) attendu");
    assert!(!args.contains(&"-p".to_string()), "-p (minuscule) inattendu");
}

/// Port 22 par défaut ne doit pas générer de flag `-P`.
#[test]
fn no_port_flag_for_default_port_22() {
    let s = base_server();
    let args =
        build_scp_args(&s, ConnectionMode::Direct, &ScpDirection::Upload, "f", "r").unwrap();
    assert!(!args.contains(&"-P".to_string()), "-P inattendu pour port 22");
}

// ─── Clé SSH ──────────────────────────────────────────────────────────────────

/// La clé SSH est passée avec `-i`.
#[test]
fn ssh_key_passed_with_i_flag() {
    let mut s = base_server();
    s.ssh_key = "/home/ops/.ssh/prod_ed25519".into();
    let args =
        build_scp_args(&s, ConnectionMode::Direct, &ScpDirection::Upload, "f", "r").unwrap();
    let i_pos = args.iter().position(|a| a == "-i").expect("-i attendu");
    assert_eq!(args[i_pos + 1], "/home/ops/.ssh/prod_ed25519");
}

// ─── Config système SSH ───────────────────────────────────────────────────────

/// `use_system_ssh_config: true` supprime le `-F /dev/null`.
#[test]
fn use_system_ssh_config_omits_f_flag() {
    let mut s = base_server();
    s.use_system_ssh_config = true;
    let args =
        build_scp_args(&s, ConnectionMode::Direct, &ScpDirection::Upload, "f", "r").unwrap();
    assert!(
        !args.contains(&"-F".to_string()),
        "-F ne doit pas être présent quand use_system_ssh_config=true"
    );
}
