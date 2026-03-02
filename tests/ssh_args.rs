//! Tests d'intégration pour `ssh::client::build_ssh_args`.
//!
//! Ces tests exercent la fonction `build_ssh_args` depuis l'API publique afin
//! de garantir que les scénarios courants produisent les arguments SSH attendus.
//! Aucun serveur SSH réel n'est requis — tout est purement en mémoire.

use susshi::config::{ConnectionMode, ResolvedServer};
use susshi::ssh::client::build_ssh_args;

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
        tags: vec![],
    }
}

// ─── Mode Direct ─────────────────────────────────────────────────────────────

/// Un serveur minimal produit `-F /dev/null` + `user@host` sans options superflues.
#[test]
fn direct_minimal() {
    let s = base_server();
    let args = build_ssh_args(&s, ConnectionMode::Direct, false).unwrap();

    assert!(args.contains(&"-F".to_string()), "-F attendu");
    assert!(args.contains(&"/dev/null".to_string()), "/dev/null attendu");
    assert!(
        args.contains(&"ops@192.168.1.10".to_string()),
        "destination attendue"
    );
    // Pas d'options superflues pour un serveur port-22 sans clé
    assert!(
        !args.contains(&"-p".to_string()),
        "-p inattendu pour port 22"
    );
    assert!(!args.contains(&"-i".to_string()), "-i inattendu sans clé");
    assert!(!args.contains(&"-v".to_string()), "-v inattendu");
}

/// La clé SSH est passée avec `-i` et le tilde est expandé.
#[test]
fn direct_with_key() {
    let mut s = base_server();
    s.ssh_key = "~/.ssh/id_ed25519".into();
    let args = build_ssh_args(&s, ConnectionMode::Direct, false).unwrap();

    let i_pos = args.iter().position(|a| a == "-i").expect("-i attendu");
    // Le tilde doit avoir été expandé (shellexpand::tilde)
    assert!(
        !args[i_pos + 1].starts_with('~'),
        "le tilde doit être expandé"
    );
    assert!(
        args[i_pos + 1].ends_with("/.ssh/id_ed25519"),
        "chemin de clé incorrect"
    );
}

/// Un port non-standard est ajouté avec `-p`.
#[test]
fn direct_with_port() {
    let mut s = base_server();
    s.port = 2222;
    let args = build_ssh_args(&s, ConnectionMode::Direct, false).unwrap();

    let p_pos = args.iter().position(|a| a == "-p").expect("-p attendu");
    assert_eq!(args[p_pos + 1], "2222", "valeur du port incorrecte");
    // La destination ne doit pas contenir le port
    assert_eq!(
        args.last().unwrap(),
        "ops@192.168.1.10",
        "destination incorrecte"
    );
}

/// Un port embarqué dans la chaîne `host:port` est extrait et transmis via `-p`.
#[test]
fn direct_with_port_in_host_string() {
    let mut s = base_server();
    s.host = "192.168.1.10:2222".into();
    let args = build_ssh_args(&s, ConnectionMode::Direct, false).unwrap();

    assert!(args.contains(&"-p".to_string()), "-p attendu");
    assert!(args.contains(&"2222".to_string()), "valeur 2222 attendue");
    // La destination doit utiliser le host sans le port
    assert_eq!(args.last().unwrap(), "ops@192.168.1.10");
}

/// Les `ssh_options` scalaires sont préfixées par `-o`, les flags (commençant par `-`) passent tels quels.
#[test]
fn direct_with_options() {
    let mut s = base_server();
    s.ssh_options = vec!["ServerAliveInterval=30".into(), "-T".into()];
    let args = build_ssh_args(&s, ConnectionMode::Direct, false).unwrap();

    let o_pos = args.iter().position(|a| a == "-o").expect("-o attendu");
    assert_eq!(
        args[o_pos + 1],
        "ServerAliveInterval=30",
        "option scalaire incorrecte"
    );
    assert!(
        args.contains(&"-T".to_string()),
        "flag -T doit passer tel quel"
    );
}

/// `use_system_ssh_config: true` supprime le `-F /dev/null`.
#[test]
fn system_ssh_config() {
    let mut s = base_server();
    s.use_system_ssh_config = true;
    let args = build_ssh_args(&s, ConnectionMode::Direct, false).unwrap();

    assert!(
        !args.contains(&"-F".to_string()),
        "-F ne doit pas être présent quand use_system_ssh_config=true"
    );
}

// ─── Mode Jump ───────────────────────────────────────────────────────────────

/// Mode Jump correct : `-J user@host` suivi de la destination cible.
#[test]
fn jump_host() {
    let mut s = base_server();
    s.jump_host = Some("jops@jump.infra.example.com".into());
    let args = build_ssh_args(&s, ConnectionMode::Jump, false).unwrap();

    let j_pos = args.iter().position(|a| a == "-J").expect("-J attendu");
    assert_eq!(args[j_pos + 1], "jops@jump.infra.example.com");
    assert_eq!(args.last().unwrap(), "ops@192.168.1.10");
}

/// Mode Jump sans `jump_host` configuré retourne une erreur explicite.
#[test]
fn jump_no_host() {
    let s = base_server(); // jump_host = None
    let err = build_ssh_args(&s, ConnectionMode::Jump, false).unwrap_err();
    assert!(
        err.to_string().contains("Jump host not configured"),
        "message d'erreur attendu, obtenu : {}",
        err
    );
}

// ─── Mode Wallix ─────────────────────────────────────────────────────────────

/// Mode Wallix : le template est correctement substitué dans `-l`.
#[test]
fn wallix_template() {
    let mut s = base_server();
    s.bastion_host = Some("bastion.corp.example.com".into());
    s.bastion_user = Some("bops".into());
    // template par défaut : {target_user}@%n:SSH:{bastion_user}
    let args = build_ssh_args(&s, ConnectionMode::Wallix, false).unwrap();

    let l_pos = args.iter().position(|a| a == "-l").expect("-l attendu");
    assert_eq!(
        args[l_pos + 1],
        "ops@192.168.1.10:SSH:bops",
        "template Wallix incorrect"
    );
    assert!(
        args.contains(&"bastion.corp.example.com".to_string()),
        "bastion host absent"
    );
}

/// Mode Wallix sans `bastion_host` retourne une erreur explicite.
#[test]
fn wallix_no_host() {
    let s = base_server(); // bastion_host = None
    let err = build_ssh_args(&s, ConnectionMode::Wallix, false).unwrap_err();
    assert!(
        err.to_string().contains("Wallix host not configured"),
        "message d'erreur attendu, obtenu : {}",
        err
    );
}

// ─── Invariant destination ────────────────────────────────────────────────────

/// La destination (`user@host`) est **toujours** le dernier argument de la liste,
/// quels que soient les modes, clés, options et ports configurés.
///
/// Cet invariant est critique : `build_tunnel_args` et `probe` s'en servent pour
/// insérer leurs propres options juste avant la cible en faisant un `args.pop()`.
#[test]
fn destination_is_last() {
    // Direct + clé + options + port non-standard + verbose
    let mut s = base_server();
    s.ssh_key = "~/.ssh/id_ed25519".into();
    s.ssh_options = vec![
        "StrictHostKeyChecking=no".into(),
        "-T".into(),
        "BatchMode=yes".into(),
    ];
    s.port = 22222;
    let args = build_ssh_args(&s, ConnectionMode::Direct, true).unwrap();
    assert_eq!(
        args.last().unwrap(),
        "ops@192.168.1.10",
        "Direct : destination doit être en dernière position"
    );

    // Jump + clé + port dans l'hôte
    let mut s2 = base_server();
    s2.ssh_key = "~/.ssh/prod_ed25519".into();
    s2.host = "192.168.1.10:22".into();
    s2.jump_host = Some("jops@jump.example.com:2222".into());
    let args2 = build_ssh_args(&s2, ConnectionMode::Jump, false).unwrap();
    assert_eq!(
        args2.last().unwrap(),
        "ops@192.168.1.10",
        "Jump : destination doit être en dernière position"
    );
}
