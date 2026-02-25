use std::process::Command;
use std::os::unix::process::CommandExt; // Import exec
use crate::config::{ResolvedServer, ConnectionMode};
use anyhow::{Result};

pub fn connect(server: &ResolvedServer, mode: ConnectionMode, verbose: bool) -> Result<()> {
    
    let mut command = Command::new("ssh");
    
    // Explicitly ignore user config as requested
    command.arg("-F").arg("/dev/null");
    
    // Add verbose flag if enabled
    if verbose {
        command.arg("-v");
    }

    // Add extra args if needed, e.g. -t for forcing TTY allocation
    // command.arg("-t"); 

    match mode {
        ConnectionMode::Direct => {
            add_target_args(&mut command, &server.user, &server.host);
        },
        ConnectionMode::Jump => {
            let jump_host_str = server.jump_host.as_deref().unwrap_or("");
            if jump_host_str.is_empty() {
                return Err(anyhow::anyhow!("Jump host not configured for this server"));
            }
            let jump_user = server.jump_user.as_deref().unwrap_or(&server.user);
            let jump_arg = format_host_arg(jump_user, jump_host_str);
            command.arg("-J").arg(jump_arg);
            add_target_args(&mut command, &server.user, &server.host);
        },
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
            command.arg("-l").arg(user_string);
            let (b_host, b_port) = parse_host_port(bastion_host_str);
            if let Some(p) = b_port {
                command.arg("-p").arg(p);
            }
            command.arg(b_host);
        },
    }

    if !server.ssh_key.is_empty() {
        let expanded = shellexpand::tilde(&server.ssh_key);
        command.arg("-i").arg(expanded.as_ref());
    }
    
    // Add custom SSH options
    for opt in &server.ssh_options {
        // Simple heuristic: if it starts with hyphen, treat as flag, else option
        if opt.starts_with('-') {
            command.arg(opt);
        } else {
            command.arg("-o").arg(opt);
        }
    }

    // Replace current process with SSH
    // If successful, this function never returns.
    let err = command.exec();

    // If we are here, exec failed
    Err(anyhow::Error::new(err).context("Failed to exec ssh command"))
}

fn add_target_args(cmd: &mut Command, user: &str, host_str: &str) {
    let (host, port) = parse_host_port(host_str);
    if let Some(p) = port {
        cmd.arg("-p").arg(p);
    }
    cmd.arg(format!("{}@{}", user, host));
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

