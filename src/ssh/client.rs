use std::process::Command;
use std::os::unix::process::CommandExt;
use crate::config::{ResolvedServer, ConnectionMode};
use anyhow::Result;

/// Construit la liste complète des arguments SSH sans lancer de processus.
/// Séparé de `connect()` pour être testable unitairement.
pub fn build_ssh_args(server: &ResolvedServer, mode: ConnectionMode, verbose: bool) -> Result<Vec<String>> {
    let mut args: Vec<String> = Vec::new();

    if !server.use_system_ssh_config {
        args.push("-F".into());
        args.push("/dev/null".into());
    }

    if verbose {
        args.push("-v".into());
    }

    match mode {
        ConnectionMode::Direct => {
            collect_target_args(&mut args, &server.user, &server.host);
        }
        ConnectionMode::Jump => {
            let jump_host_str = server.jump_host.as_deref().unwrap_or("");
            if jump_host_str.is_empty() {
                return Err(anyhow::anyhow!("Jump host not configured for this server"));
            }
            let jump_user = server.jump_user.as_deref().unwrap_or(&server.user);
            let jump_arg = format_host_arg(jump_user, jump_host_str);
            args.push("-J".into());
            args.push(jump_arg);
            collect_target_args(&mut args, &server.user, &server.host);
        }
        ConnectionMode::Bastion => {
            let bastion_host_str = server.bastion_host.as_deref().unwrap_or("");
            if bastion_host_str.is_empty() {
                return Err(anyhow::anyhow!("Bastion host not configured for this server"));
            }
            let bastion_user = server.bastion_user.as_deref().unwrap_or("root");
            let (t_host, _t_port) = parse_host_port(&server.host);
            let user_string = server.bastion_template
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

fn collect_target_args(args: &mut Vec<String>, user: &str, host_str: &str) {
    let (host, port) = parse_host_port(host_str);
    if let Some(p) = port {
        args.push("-p".into());
        args.push(p.to_string());
    }
    args.push(format!("{}@{}", user, host));
}

fn format_host_arg(user: &str, host_str: &str) -> String {
    let (host, port) = parse_host_port(host_str);
    if let Some(p) = port {
        format!("{}@{}:{}", user, host, p)
    } else {
        format!("{}@{}", user, host)
    }
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
            jump_user: None,
            bastion_host: None,
            bastion_user: None,
            bastion_template: "{target_user}@%n:SSH:{bastion_user}".into(),
            use_system_ssh_config: false,
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
        s.jump_host = Some("jump.example.com".into());
        s.jump_user = Some("juser".into());
        let args = build_ssh_args(&s, ConnectionMode::Jump, false).unwrap();
        let j_pos = args.iter().position(|a| a == "-J").expect("-J present");
        assert_eq!(args[j_pos + 1], "juser@jump.example.com");
        assert!(args.contains(&"admin@10.0.0.1".to_string()));
    }

    #[test]
    fn jump_with_port() {
        let mut s = base_server();
        s.jump_host = Some("jump.example.com:2222".into());
        s.jump_user = Some("juser".into());
        let args = build_ssh_args(&s, ConnectionMode::Jump, false).unwrap();
        let j_pos = args.iter().position(|a| a == "-J").expect("-J present");
        assert_eq!(args[j_pos + 1], "juser@jump.example.com:2222");
    }

    #[test]
    fn jump_fallback_user() {
        // jump_user absent → server user is used
        let mut s = base_server();
        s.jump_host = Some("jump.example.com".into());
        s.jump_user = None;
        let args = build_ssh_args(&s, ConnectionMode::Jump, false).unwrap();
        let j_pos = args.iter().position(|a| a == "-J").expect("-J present");
        assert_eq!(args[j_pos + 1], "admin@jump.example.com");
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


