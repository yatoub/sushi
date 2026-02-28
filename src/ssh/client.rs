use crate::config::{ConnectionMode, ResolvedServer};
use anyhow::Result;
use std::os::unix::process::CommandExt;
use std::process::Command;

/// Construit la liste complète des arguments SSH sans lancer de processus.
/// Séparé de `connect()` pour être testable unitairement.
///
/// **Invariant** : la destination (`user@host` ou `bastion_host`) est toujours
/// le **dernier** argument de la liste retournée. `probe()` s'appuie sur cet
/// invariant pour insérer ses options juste avant elle via `args.pop()`.
pub fn build_ssh_args(
    server: &ResolvedServer,
    mode: ConnectionMode,
    verbose: bool,
) -> Result<Vec<String>> {
    let mut args: Vec<String> = Vec::new();

    if !server.use_system_ssh_config {
        args.push("-F".into());
        args.push("/dev/null".into());
    }

    if verbose {
        args.push("-v".into());
    }

    // Clé et options SSH — placées AVANT la destination pour que celle-ci
    // reste en dernière position (invariant utilisé par probe()).
    if !server.ssh_key.is_empty() {
        let expanded = shellexpand::tilde(&server.ssh_key);
        args.push("-i".into());
        args.push(expanded.into_owned());
    }

    for opt in &server.ssh_options {
        if opt.starts_with('-') {
            args.push(opt.clone());
        } else {
            args.push("-o".into());
            args.push(opt.clone());
        }
    }

    // Destination — toujours en dernier.
    match mode {
        ConnectionMode::Direct => {
            collect_target_args(&mut args, &server.user, &server.host, server.port);
        }
        ConnectionMode::Jump => {
            let jump_str = server.jump_host.as_deref().unwrap_or("");
            if jump_str.is_empty() {
                return Err(anyhow::anyhow!("Jump host not configured for this server"));
            }
            args.push("-J".into());
            args.push(jump_str.to_string());
            collect_target_args(&mut args, &server.user, &server.host, server.port);
        }
        ConnectionMode::Bastion => {
            let bastion_host_str = server.bastion_host.as_deref().unwrap_or("");
            if bastion_host_str.is_empty() {
                return Err(anyhow::anyhow!(
                    "Bastion host not configured for this server"
                ));
            }
            let bastion_user = server.bastion_user.as_deref().unwrap_or("root");
            let (t_host, _t_port) = parse_host_port(&server.host);
            let user_string = server
                .bastion_template
                .replace("{target_user}", &server.user)
                .replace("{target_host}", t_host)
                .replace("{bastion_user}", bastion_user)
                .replace("%n", t_host);
            args.push("-l".into());
            args.push(user_string);
            let (b_host, b_port) = parse_host_port(bastion_host_str);
            if let Some(p) = b_port {
                args.push("-p".into());
                args.push(p.to_string());
            }
            args.push(b_host.to_string());
        }
    }

    Ok(args)
}

/// Lance la connexion SSH en remplaçant le processus courant (`exec`).
pub fn connect(server: &ResolvedServer, mode: ConnectionMode, verbose: bool) -> Result<()> {
    let args = build_ssh_args(server, mode, verbose)?;
    let mut command = Command::new("ssh");
    command.args(&args);
    let err = command.exec();
    Err(anyhow::Error::new(err).context("Failed to exec ssh command"))
}

// ─── helpers privés ──────────────────────────────────────────────────────────

fn collect_target_args(args: &mut Vec<String>, user: &str, host_str: &str, server_port: u16) {
    let (host, embedded_port) = parse_host_port(host_str);
    // Priorité : port embarqué dans host_str (ex. "host:2222") puis server.port.
    let port = embedded_port
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(server_port);
    if port != 22 {
        args.push("-p".into());
        args.push(port.to_string());
    }
    args.push(format!("{}@{}", user, host));
}

fn parse_host_port(s: &str) -> (&str, Option<&str>) {
    if let Some((host, port)) = s.split_once(':') {
        (host, Some(port))
    } else {
        (s, None)
    }
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConnectionMode;

    fn base_server() -> ResolvedServer {
        ResolvedServer {
            group_name: "G".into(),
            env_name: "E".into(),
            name: "srv".into(),
            host: "10.0.0.1".into(),
            user: "admin".into(),
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
        }
    }

    // ── mode Direct ──────────────────────────────────────────────────────────

    #[test]
    fn direct_basic() {
        let s = base_server();
        let args = build_ssh_args(&s, ConnectionMode::Direct, false).unwrap();
        assert!(args.contains(&"-F".to_string()));
        assert!(args.contains(&"/dev/null".to_string()));
        assert!(args.contains(&"admin@10.0.0.1".to_string()));
        assert!(!args.contains(&"-v".to_string()));
    }

    #[test]
    fn direct_verbose() {
        let s = base_server();
        let args = build_ssh_args(&s, ConnectionMode::Direct, true).unwrap();
        assert!(args.contains(&"-v".to_string()));
    }

    #[test]
    fn direct_with_port_in_host() {
        let mut s = base_server();
        s.host = "10.0.0.1:2222".into();
        let args = build_ssh_args(&s, ConnectionMode::Direct, false).unwrap();
        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"2222".to_string()));
        assert!(args.contains(&"admin@10.0.0.1".to_string()));
    }

    #[test]
    fn direct_with_port_field() {
        // Port via server.port (cas CLI --port ou ssh_port dans la config),
        // sans port embarqué dans la chaîne hôte.
        let mut s = base_server();
        s.port = 2222;
        let args = build_ssh_args(&s, ConnectionMode::Direct, false).unwrap();
        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"2222".to_string()));
        assert!(args.contains(&"admin@10.0.0.1".to_string()));
    }

    #[test]
    fn direct_with_ssh_key() {
        let mut s = base_server();
        s.ssh_key = "~/.ssh/id_ed25519".into();
        let args = build_ssh_args(&s, ConnectionMode::Direct, false).unwrap();
        let key_pos = args.iter().position(|a| a == "-i").expect("-i present");
        assert!(!args[key_pos + 1].is_empty());
    }

    #[test]
    fn direct_with_ssh_options() {
        let mut s = base_server();
        s.ssh_options = vec!["StrictHostKeyChecking=no".into(), "-T".into()];
        let args = build_ssh_args(&s, ConnectionMode::Direct, false).unwrap();
        // String option → prefixed with -o
        let o_pos = args.iter().position(|a| a == "-o").expect("-o present");
        assert_eq!(args[o_pos + 1], "StrictHostKeyChecking=no");
        // Flag option → passed as-is
        assert!(args.contains(&"-T".to_string()));
    }

    #[test]
    fn direct_use_system_ssh_config() {
        let mut s = base_server();
        s.use_system_ssh_config = true;
        let args = build_ssh_args(&s, ConnectionMode::Direct, false).unwrap();
        assert!(!args.contains(&"-F".to_string()));
    }

    // ── mode Jump ────────────────────────────────────────────────────────────

    #[test]
    fn jump_basic() {
        let mut s = base_server();
        // jump_host contient déjà "user@host" (pré-formaté par resolve_server)
        s.jump_host = Some("juser@jump.example.com".into());
        let args = build_ssh_args(&s, ConnectionMode::Jump, false).unwrap();
        let j_pos = args.iter().position(|a| a == "-J").expect("-J present");
        assert_eq!(args[j_pos + 1], "juser@jump.example.com");
        assert!(args.contains(&"admin@10.0.0.1".to_string()));
    }

    #[test]
    fn jump_with_port() {
        let mut s = base_server();
        s.jump_host = Some("juser@jump.example.com:2222".into());
        let args = build_ssh_args(&s, ConnectionMode::Jump, false).unwrap();
        let j_pos = args.iter().position(|a| a == "-J").expect("-J present");
        assert_eq!(args[j_pos + 1], "juser@jump.example.com:2222");
    }

    #[test]
    fn jump_fallback_user() {
        // jump_user absent → l'utilisateur du serveur est déjà intégré au moment de la résolution
        let mut s = base_server();
        s.jump_host = Some("admin@jump.example.com".into()); // user=admin = server.user
        let args = build_ssh_args(&s, ConnectionMode::Jump, false).unwrap();
        let j_pos = args.iter().position(|a| a == "-J").expect("-J present");
        assert_eq!(args[j_pos + 1], "admin@jump.example.com");
    }

    #[test]
    fn jump_multi_hop() {
        // Chaîne de deux sauts pré-formatée par resolve_server
        let mut s = base_server();
        s.jump_host = Some("juser@jump1.example.com,juser@jump2.example.com".into());
        let args = build_ssh_args(&s, ConnectionMode::Jump, false).unwrap();
        let j_pos = args.iter().position(|a| a == "-J").expect("-J present");
        assert_eq!(
            args[j_pos + 1],
            "juser@jump1.example.com,juser@jump2.example.com"
        );
        assert!(args.contains(&"admin@10.0.0.1".to_string()));
    }

    #[test]
    fn jump_missing_host_returns_error() {
        let s = base_server(); // jump_host = None
        let err = build_ssh_args(&s, ConnectionMode::Jump, false).unwrap_err();
        assert!(err.to_string().contains("Jump host not configured"));
    }

    // ── mode Bastion ─────────────────────────────────────────────────────────

    #[test]
    fn bastion_basic() {
        let mut s = base_server();
        s.bastion_host = Some("bastion.example.com".into());
        s.bastion_user = Some("buser".into());
        let args = build_ssh_args(&s, ConnectionMode::Bastion, false).unwrap();
        let l_pos = args.iter().position(|a| a == "-l").expect("-l present");
        // template: {target_user}@%n:SSH:{bastion_user}
        assert_eq!(args[l_pos + 1], "admin@10.0.0.1:SSH:buser");
        assert!(args.contains(&"bastion.example.com".to_string()));
    }

    #[test]
    fn bastion_with_port() {
        let mut s = base_server();
        s.bastion_host = Some("bastion.example.com:8022".into());
        s.bastion_user = Some("buser".into());
        let args = build_ssh_args(&s, ConnectionMode::Bastion, false).unwrap();
        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"8022".to_string()));
        assert!(args.contains(&"bastion.example.com".to_string()));
    }

    #[test]
    fn bastion_fallback_user() {
        let mut s = base_server();
        s.bastion_host = Some("bastion.example.com".into());
        s.bastion_user = None; // fallback → "root"
        let args = build_ssh_args(&s, ConnectionMode::Bastion, false).unwrap();
        let l_pos = args.iter().position(|a| a == "-l").expect("-l present");
        assert!(args[l_pos + 1].ends_with(":SSH:root"));
    }

    #[test]
    fn bastion_missing_host_returns_error() {
        let s = base_server(); // bastion_host = None
        let err = build_ssh_args(&s, ConnectionMode::Bastion, false).unwrap_err();
        assert!(err.to_string().contains("Bastion host not configured"));
    }

    #[test]
    fn bastion_custom_template() {
        let mut s = base_server();
        s.bastion_host = Some("bastion.example.com".into());
        s.bastion_user = Some("buser".into());
        s.bastion_template = "{bastion_user}+{target_user}@{target_host}".into();
        let args = build_ssh_args(&s, ConnectionMode::Bastion, false).unwrap();
        let l_pos = args.iter().position(|a| a == "-l").expect("-l present");
        assert_eq!(args[l_pos + 1], "buser+admin@10.0.0.1");
    }
}
