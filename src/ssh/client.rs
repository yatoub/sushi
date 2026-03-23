use crate::config::{ConnectionMode, ResolvedServer};
use crate::wallix::WallixMenuEntry;
#[cfg(unix)]
use crate::wallix::{parse_wallix_menu, select_id_for_server};
use anyhow::Result;
#[cfg(unix)]
use nix::pty::{ForkptyResult, Winsize, forkpty};
#[cfg(unix)]
use nix::sys::wait::{WaitPidFlag, WaitStatus, waitpid};
#[cfg(unix)]
use std::os::unix::io::AsRawFd;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::process::Command;
#[cfg(unix)]
use std::{
    ffi::CString,
    io::{Read, Write},
};

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

    // ControlMaster SSH multiplexing (non supporté en mode Wallix).
    if server.control_master && mode != ConnectionMode::Wallix && !server.control_path.is_empty() {
        if let Some(parent) = std::path::Path::new(&server.control_path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        args.push("-o".into());
        args.push("ControlMaster=auto".into());
        args.push("-o".into());
        args.push(format!("ControlPath={}", server.control_path));
        args.push("-o".into());
        args.push(format!("ControlPersist={}", server.control_persist));
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
        ConnectionMode::Wallix => {
            let bastion_host_str = server.bastion_host.as_deref().unwrap_or("");
            if bastion_host_str.is_empty() {
                return Err(anyhow::anyhow!(
                    "Wallix host not configured for this server"
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
    #[cfg(unix)]
    if should_use_wallix_menu_automation(server, mode) {
        return connect_wallix_via_pty_with_selection(server, verbose, None);
    }

    let args = build_ssh_args(server, mode, verbose)?;
    let mut command = Command::new("ssh");
    command.args(&args);
    #[cfg(unix)]
    {
        let err = command.exec();
        Err(anyhow::Error::new(err).context("Failed to exec ssh command"))
    }
    #[cfg(not(unix))]
    {
        command
            .status()
            .map(|_| ())
            .map_err(|e| anyhow::Error::new(e).context("Failed to spawn ssh command"))
    }
}

/// Lance la connexion SSH dans un sous-processus bloquant (sans `exec`).
/// Contrairement à [`connect`], retourne après la fin de la session SSH —
/// utilisé quand `keep_open` est actif pour revenir à la TUI ensuite.
pub fn connect_blocking(
    server: &ResolvedServer,
    mode: ConnectionMode,
    verbose: bool,
) -> Result<()> {
    #[cfg(unix)]
    if should_use_wallix_menu_automation(server, mode) {
        return connect_wallix_via_pty_with_selection(server, verbose, None);
    }

    let args = build_ssh_args(server, mode, verbose)?;
    Command::new("ssh")
        .args(&args)
        .status()
        .map(|_| ())
        .map_err(|e| anyhow::Error::new(e).context("Failed to spawn ssh command"))
}

/// Récupère les entrées du menu Wallix affichées par le bastion sans ouvrir de shell distant.
#[cfg(unix)]
pub fn fetch_wallix_menu_entries(
    server: &ResolvedServer,
    verbose: bool,
) -> Result<Vec<WallixMenuEntry>> {
    let args = build_wallix_bastion_args(server, verbose)?;
    let (child, mut master_reader, mut master_writer) = spawn_wallix_pty(&args)?;
    let mut transcript = String::new();

    loop {
        let page = read_until_wallix_prompt(&mut master_reader)?;
        transcript.push_str(&page);

        match parse_wallix_page_position(&page) {
            Some((current, total)) if current < total => {
                master_writer.write_all(b"n\n")?;
                master_writer.flush()?;
            }
            _ => break,
        }
    }

    unsafe {
        libc::kill(child.as_raw(), libc::SIGTERM);
    }
    let _ = waitpid(child, Some(WaitPidFlag::WNOHANG));
    parse_wallix_menu(&transcript)
}

#[cfg(not(unix))]
pub fn fetch_wallix_menu_entries(
    _server: &ResolvedServer,
    _verbose: bool,
) -> Result<Vec<WallixMenuEntry>> {
    anyhow::bail!("Wallix menu fetching is only supported on Unix")
}

/// Lance une session Wallix en forçant un ID déjà choisi côté TUI.
pub fn connect_wallix_with_selection(
    server: &ResolvedServer,
    verbose: bool,
    selected_id: &str,
) -> Result<()> {
    #[cfg(unix)]
    {
        connect_wallix_via_pty_with_selection(server, verbose, Some(selected_id))
    }
    #[cfg(not(unix))]
    {
        let _ = (server, verbose, selected_id);
        anyhow::bail!("Wallix menu automation is only supported on Unix")
    }
}

/// Variante bloquante de [`connect_wallix_with_selection`].
pub fn connect_blocking_wallix_with_selection(
    server: &ResolvedServer,
    verbose: bool,
    selected_id: &str,
) -> Result<()> {
    connect_wallix_with_selection(server, verbose, selected_id)
}

// ─── helpers privés ──────────────────────────────────────────────────────────

#[cfg(unix)]
fn should_use_wallix_menu_automation(server: &ResolvedServer, mode: ConnectionMode) -> bool {
    mode == ConnectionMode::Wallix && server.wallix_auto_select
}

#[cfg(unix)]
fn build_wallix_bastion_args(server: &ResolvedServer, verbose: bool) -> Result<Vec<String>> {
    let mut args: Vec<String> = Vec::new();

    if !server.use_system_ssh_config {
        args.push("-F".into());
        args.push("/dev/null".into());
    }

    if verbose {
        args.push("-v".into());
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

    let bastion_host_str = server.bastion_host.as_deref().unwrap_or("");
    if bastion_host_str.is_empty() {
        return Err(anyhow::anyhow!(
            "Wallix host not configured for this server"
        ));
    }

    let bastion_user = server.bastion_user.as_deref().unwrap_or("root");
    args.push("-l".into());
    args.push(bastion_user.to_string());

    let (b_host, b_port) = parse_host_port(bastion_host_str);
    if let Some(p) = b_port {
        args.push("-p".into());
        args.push(p.to_string());
    }
    args.push(b_host.to_string());

    Ok(args)
}

#[cfg(unix)]
fn current_winsize() -> Option<Winsize> {
    let mut winsize = Winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

    let rc = unsafe { libc::ioctl(libc::STDIN_FILENO, libc::TIOCGWINSZ, &mut winsize) };
    if rc == 0 { Some(winsize) } else { None }
}

#[cfg(unix)]
fn contains_wallix_prompt(buffer: &str) -> bool {
    let trimmed = buffer.trim_end();
    trimmed.ends_with(" >")
        || trimmed.ends_with(">")
        || trimmed.lines().rev().find(|line| !line.trim().is_empty()) == Some(">")
}

#[cfg(unix)]
fn contains_wallix_target_address_prompt(buffer: &str) -> bool {
    let lowered = buffer.to_ascii_lowercase();
    lowered.contains("adresse cible")
        || lowered.contains("target address")
        || lowered.contains("destination address")
}

#[cfg(unix)]
fn parse_wallix_page_position(buffer: &str) -> Option<(u32, u32)> {
    let lowered = buffer.to_ascii_lowercase();
    let marker = "page ";
    let start = lowered.rfind(marker)? + marker.len();
    let tail = &lowered[start..];

    let mut current = String::new();
    let mut total = String::new();
    let mut seen_slash = false;

    for character in tail.chars() {
        if character.is_ascii_digit() {
            if seen_slash {
                total.push(character);
            } else {
                current.push(character);
            }
        } else if character == '/' && !seen_slash {
            seen_slash = true;
        } else if !current.is_empty() {
            break;
        }
    }

    if current.is_empty() || total.is_empty() {
        return None;
    }

    Some((current.parse().ok()?, total.parse().ok()?))
}

#[cfg(unix)]
fn is_wallix_menu_matching_error(err: &anyhow::Error) -> bool {
    let message = err.to_string();
    message.contains("No menu entry found with target")
        || message.contains("No menu entry found for matching targets")
        || message.contains("No menu entry found for target")
}

#[cfg(unix)]
fn spawn_wallix_pty(args: &[String]) -> Result<(nix::unistd::Pid, std::fs::File, std::fs::File)> {
    let mut argv = Vec::with_capacity(args.len() + 2);
    argv.push(CString::new("ssh")?);
    for arg in args {
        argv.push(CString::new(arg.as_str())?);
    }
    let mut argv_ptrs: Vec<*const libc::c_char> = argv.iter().map(|arg| arg.as_ptr()).collect();
    argv_ptrs.push(std::ptr::null());

    let winsize = current_winsize();
    let fork = unsafe { forkpty(winsize.as_ref(), None) }
        .map_err(|err| anyhow::anyhow!("Failed to create PTY for Wallix session: {err}"))?;

    match fork {
        ForkptyResult::Child => unsafe {
            libc::execvp(argv[0].as_ptr(), argv_ptrs.as_ptr());
            libc::_exit(127);
        },
        ForkptyResult::Parent { child, master } => {
            let master_reader = std::fs::File::from(master);
            let master_writer = master_reader.try_clone()?;
            Ok((child, master_reader, master_writer))
        }
    }
}

#[cfg(unix)]
fn read_until_wallix_prompt(master_reader: &mut std::fs::File) -> Result<String> {
    let mut transcript = String::new();
    loop {
        let mut buf = [0_u8; 4096];
        let read = master_reader.read(&mut buf)?;
        if read == 0 {
            break;
        }

        let chunk = String::from_utf8_lossy(&buf[..read]);
        transcript.push_str(&chunk);
        if transcript.len() > 64 * 1024 {
            let drain = transcript.len().saturating_sub(64 * 1024);
            transcript.drain(..drain);
        }

        if contains_wallix_prompt(&transcript) {
            return Ok(transcript);
        }
    }

    Err(anyhow::anyhow!(
        "Wallix session exited before the selection prompt was displayed"
    ))
}

#[cfg(unix)]
fn connect_wallix_via_pty_with_selection(
    server: &ResolvedServer,
    verbose: bool,
    selected_id: Option<&str>,
) -> Result<()> {
    let args = build_wallix_bastion_args(server, verbose)?;
    let (child, mut master_reader, mut master_writer) = spawn_wallix_pty(&args)?;
    let mut stdout = std::io::stdout().lock();
    let mut stdin = std::io::stdin().lock();
    let mut transcript = String::new();
    let mut selection_completed = false;
    let mut target_address_sent = false;
    let mut stdin_closed = false;
    let master_fd = master_reader.as_raw_fd();
    let stdin_fd = std::io::stdin().as_raw_fd();

    loop {
        let mut pollfds = [
            libc::pollfd {
                fd: master_fd,
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: if stdin_closed || !selection_completed {
                    -1
                } else {
                    stdin_fd
                },
                events: libc::POLLIN,
                revents: 0,
            },
        ];

        let rc = unsafe { libc::poll(pollfds.as_mut_ptr(), pollfds.len() as _, 100) };
        if rc < 0 {
            let err = std::io::Error::last_os_error();
            if err.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return Err(err.into());
        }

        if pollfds[0].revents & libc::POLLIN != 0 {
            let mut buf = [0_u8; 4096];
            let read = master_reader.read(&mut buf)?;
            if read == 0 {
                break;
            }

            stdout.write_all(&buf[..read])?;
            stdout.flush()?;

            if !selection_completed {
                let chunk = String::from_utf8_lossy(&buf[..read]);
                transcript.push_str(&chunk);
                if transcript.len() > 64 * 1024 {
                    let drain = transcript.len().saturating_sub(64 * 1024);
                    transcript.drain(..drain);
                }

                if contains_wallix_prompt(&transcript) {
                    let selection = if let Some(id) = selected_id {
                        Ok(id.to_string())
                    } else {
                        parse_wallix_menu(&transcript)
                            .and_then(|entries| select_id_for_server(&entries, server))
                    };

                    match selection {
                        Ok(id) => {
                            master_writer.write_all(id.as_bytes())?;
                            master_writer.write_all(b"\n")?;
                            master_writer.flush()?;
                            selection_completed = true;
                        }
                        Err(err) if server.wallix_fail_if_menu_match_error => {
                            if is_wallix_menu_matching_error(&err)
                                && let Some((current, total)) =
                                    parse_wallix_page_position(&transcript)
                                && current < total
                            {
                                master_writer.write_all(b"n\n")?;
                                master_writer.flush()?;
                                transcript.clear();
                                continue;
                            }

                            if is_wallix_menu_matching_error(&err) {
                                // Fallback manuel: l'utilisateur choisit lui-même dans le menu.
                                selection_completed = true;
                                continue;
                            }

                            unsafe {
                                libc::kill(child.as_raw(), libc::SIGTERM);
                            }
                            let _ = waitpid(child, Some(WaitPidFlag::WNOHANG));
                            return Err(err);
                        }
                        Err(_) => {
                            selection_completed = true;
                        }
                    }
                }
            } else if !target_address_sent {
                let chunk = String::from_utf8_lossy(&buf[..read]);
                transcript.push_str(&chunk);
                if transcript.len() > 64 * 1024 {
                    let drain = transcript.len().saturating_sub(64 * 1024);
                    transcript.drain(..drain);
                }

                if contains_wallix_target_address_prompt(&transcript) {
                    master_writer.write_all(server.host.as_bytes())?;
                    master_writer.write_all(b"\n")?;
                    master_writer.flush()?;
                    target_address_sent = true;
                }
            }
        }

        if pollfds[1].revents & libc::POLLIN != 0 {
            let mut buf = [0_u8; 4096];
            let read = stdin.read(&mut buf)?;
            if read == 0 {
                stdin_closed = true;
            } else {
                master_writer.write_all(&buf[..read])?;
                master_writer.flush()?;
            }
        }

        match waitpid(child, Some(WaitPidFlag::WNOHANG)) {
            Ok(WaitStatus::StillAlive) => {}
            Ok(_) => return Ok(()),
            Err(err) => {
                return Err(anyhow::anyhow!("Failed to wait for Wallix session: {err}"));
            }
        }
    }

    if !selection_completed {
        return Err(anyhow::anyhow!(
            "Wallix session exited before menu auto-selection completed"
        ));
    }

    Ok(())
}

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
            namespace: String::new(),
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
            tunnels: vec![],
            tags: vec![],
            control_master: false,
            control_path: String::new(),
            control_persist: "10m".to_string(),
            pre_connect_hook: None,
            post_disconnect_hook: None,
            hook_timeout_secs: 5,
            wallix_group: None,
            wallix_account: "default".to_string(),
            wallix_protocol: "SSH".to_string(),
            wallix_auto_select: true,
            wallix_fail_if_menu_match_error: true,
            wallix_selection_timeout_secs: 8,
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

    // ── mode Wallix ──────────────────────────────────────────────────────────

    #[test]
    fn wallix_basic() {
        let mut s = base_server();
        s.bastion_host = Some("bastion.example.com".into());
        s.bastion_user = Some("buser".into());
        let args = build_ssh_args(&s, ConnectionMode::Wallix, false).unwrap();
        let l_pos = args.iter().position(|a| a == "-l").expect("-l present");
        // template: {target_user}@%n:SSH:{bastion_user}
        assert_eq!(args[l_pos + 1], "admin@10.0.0.1:SSH:buser");
        assert!(args.contains(&"bastion.example.com".to_string()));
    }

    #[test]
    fn wallix_with_port() {
        let mut s = base_server();
        s.bastion_host = Some("bastion.example.com:8022".into());
        s.bastion_user = Some("buser".into());
        let args = build_ssh_args(&s, ConnectionMode::Wallix, false).unwrap();
        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"8022".to_string()));
        assert!(args.contains(&"bastion.example.com".to_string()));
    }

    #[test]
    fn wallix_fallback_user() {
        let mut s = base_server();
        s.bastion_host = Some("bastion.example.com".into());
        s.bastion_user = None; // fallback → "root"
        let args = build_ssh_args(&s, ConnectionMode::Wallix, false).unwrap();
        let l_pos = args.iter().position(|a| a == "-l").expect("-l present");
        assert!(args[l_pos + 1].ends_with(":SSH:root"));
    }

    #[test]
    fn wallix_missing_host_returns_error() {
        let s = base_server(); // bastion_host = None
        let err = build_ssh_args(&s, ConnectionMode::Wallix, false).unwrap_err();
        assert!(err.to_string().contains("Wallix host not configured"));
    }

    #[test]
    fn wallix_custom_template() {
        let mut s = base_server();
        s.bastion_host = Some("bastion.example.com".into());
        s.bastion_user = Some("buser".into());
        s.bastion_template = "{bastion_user}+{target_user}@{target_host}".into();
        let args = build_ssh_args(&s, ConnectionMode::Wallix, false).unwrap();
        let l_pos = args.iter().position(|a| a == "-l").expect("-l present");
        assert_eq!(args[l_pos + 1], "buser+admin@10.0.0.1");
    }

    #[test]
    fn wallix_bastion_args_use_bastion_identity_only_for_menu_automation() {
        let mut s = base_server();
        s.bastion_host = Some("bastion.example.com:8022".into());
        s.bastion_user = Some("demo_user".into());
        let args = build_wallix_bastion_args(&s, false).unwrap();

        assert!(args.contains(&"-l".to_string()));
        assert!(args.contains(&"demo_user".to_string()));
        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"8022".to_string()));
        assert_eq!(args.last().unwrap(), "bastion.example.com");
    }

    #[test]
    fn wallix_menu_prompt_detection_supports_ascii_prompt() {
        assert!(contains_wallix_prompt(
            "Tapez h pour l'aide, ctrl-D pour quitter\n > "
        ));
    }

    #[test]
    fn wallix_target_address_prompt_detection_supports_french_prompt() {
        assert!(contains_wallix_target_address_prompt(
            "Account successfully checked out\nAdresse cible (dans 10.242.23.24/29): "
        ));
    }

    #[test]
    fn wallix_page_position_parser_reads_page_numbers() {
        let line = "| ID | Cible (page 1/16)                       | Autorisation";
        assert_eq!(parse_wallix_page_position(line), Some((1, 16)));
    }

    // ── invariant destination ─────────────────────────────────────────────────

    /// Garantit que la destination (`user@host`) est toujours le dernier argument,
    /// quelle que soit la combinaison d'options. Cet invariant est utilisé par
    /// `build_tunnel_args` et `probe` pour insérer des options juste avant la cible.
    #[test]
    fn destination_is_last() {
        // Direct avec clé + options + port non-standard
        let mut s = base_server();
        s.ssh_key = "~/.ssh/id_ed25519".into();
        s.ssh_options = vec!["StrictHostKeyChecking=no".into(), "-T".into()];
        s.port = 2222;
        let args = build_ssh_args(&s, ConnectionMode::Direct, true).unwrap();
        assert_eq!(args.last().unwrap(), "admin@10.0.0.1");

        // Jump avec clé + port dans l'hôte
        let mut s2 = base_server();
        s2.ssh_key = "~/.ssh/id_ed25519".into();
        s2.host = "10.0.0.1:2222".into();
        s2.jump_host = Some("juser@jump.example.com:22".into());
        let args2 = build_ssh_args(&s2, ConnectionMode::Jump, false).unwrap();
        assert_eq!(args2.last().unwrap(), "admin@10.0.0.1");

        // Direct minimal — destination = dernier arg même sans options
        let s3 = base_server();
        let args3 = build_ssh_args(&s3, ConnectionMode::Direct, false).unwrap();
        assert_eq!(args3.last().unwrap(), "admin@10.0.0.1");
    }
}
