use crate::config::{ConnectionMode, ResolvedServer};
use anyhow::Result;
use libc;
use std::fs::File;
use std::io::Read;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::mpsc;

// ─── Types publics ────────────────────────────────────────────────────────────

/// Sens du transfert SCP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScpDirection {
    /// Envoi : local → serveur.
    Upload,
    /// Récupération : serveur → local.
    Download,
}

impl ScpDirection {
    /// Libellé court en français.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Upload => "Upload",
            Self::Download => "Download",
        }
    }
}

/// Évènement émis par le thread de surveillance SCP.
#[derive(Debug)]
pub enum ScpEvent {
    /// Progression (0–100).
    Progress(u8),
    /// Transfert terminé — `true` = succès.
    Done(bool),
    /// Erreur irrécupérable (lancement impossible, etc.).
    Error(String),
}

// ─── Construction des arguments ───────────────────────────────────────────────

/// Construit la liste d'arguments pour la commande `scp`.
///
/// Le format produit est :
/// ```text
/// scp [-F /dev/null] [-i key] [-P port] [-J jump] [-o opt...] src dst
/// ```
///
/// - `local` : chemin local (tilde `~` accepté — expandé à la soumission).
/// - `remote` : chemin distant **sans** le préfixe `user@host:` (ajouté ici).
///
/// **Non disponible** en mode [`ConnectionMode::Wallix`].
pub fn build_scp_args(
    server: &ResolvedServer,
    mode: ConnectionMode,
    direction: &ScpDirection,
    local: &str,
    remote: &str,
) -> Result<Vec<String>> {
    if mode == ConnectionMode::Wallix {
        anyhow::bail!("SCP non disponible en mode Wallix");
    }

    let mut args: Vec<String> = Vec::new();

    // Désactivation du fichier de configuration système SSH si nécessaire.
    if !server.use_system_ssh_config {
        args.push("-F".into());
        args.push("/dev/null".into());
    }

    // Clé SSH.
    if !server.ssh_key.is_empty() {
        let expanded = shellexpand::tilde(&server.ssh_key);
        args.push("-i".into());
        args.push(expanded.into_owned());
    }

    // Options SSH (scp accepte -o, idem ssh).
    for opt in &server.ssh_options {
        if opt.starts_with('-') {
            args.push(opt.clone());
        } else {
            args.push("-o".into());
            args.push(opt.clone());
        }
    }

    // Port — scp utilise -P (majuscule), contrairement à ssh.
    let (host, embedded_port) = split_host_port(&server.host);
    let port = embedded_port
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(server.port);
    if port != 22 {
        args.push("-P".into());
        args.push(port.to_string());
    }

    // Jump host.
    if mode == ConnectionMode::Jump {
        let jump_str = server.jump_host.as_deref().unwrap_or("");
        if jump_str.is_empty() {
            anyhow::bail!("Jump host non configuré pour ce serveur");
        }
        args.push("-J".into());
        args.push(jump_str.to_string());
    }

    // Source et destination.
    let local_expanded = shellexpand::tilde(local).into_owned();
    let remote_addr = format!("{}@{}:{}", server.user, host, remote);

    match direction {
        ScpDirection::Upload => {
            args.push(local_expanded);
            args.push(remote_addr);
        }
        ScpDirection::Download => {
            args.push(remote_addr);
            args.push(local_expanded);
        }
    }

    Ok(args)
}

/// Lance un transfert SCP en sous-processus non-bloquant.
///
/// Un pseudo-terminal (PTY) est créé et connecté à stdout+stderr de
/// `scp` afin que celui-ci détecte un vrai terminal et émette la barre
/// de progression en temps réel (sans PTY, `scp` désactive la progression
/// dès qu'il détecte un pipe via `isatty()`).
///
/// Retourne immédiatement un [`mpsc::Receiver`] qui émettra :
/// - [`ScpEvent::Progress`] au fur et à mesure (0–100)
/// - [`ScpEvent::Done`] à la fin
/// - [`ScpEvent::Error`] si le lancement échoue
pub fn spawn_scp(
    server: &ResolvedServer,
    mode: ConnectionMode,
    direction: ScpDirection,
    local: &str,
    remote: &str,
) -> Result<(mpsc::Receiver<ScpEvent>, u32)> {
    let args = build_scp_args(server, mode, &direction, local, remote)?;

    // Crée un PTY : le processus scp recevra le fd esclave comme stdin/stdout/stderr
    // et croira être connecté à un vrai terminal -> progression activée.
    //
    // OpenSSH vérifie dans progressmeter.c : `getpgrp() == tcgetpgrp(STDERR_FILENO)`
    // → scp doit être chef de session (`setsid()`) et le PTY esclave doit être son
    //   terminal de contrôle (`TIOCSCTTY`).
    let winsz = nix::pty::Winsize {
        ws_row: 50,
        ws_col: 220,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let pty = nix::pty::openpty(Some(&winsz), None)
        .map_err(|e| anyhow::anyhow!("Impossible de créer un PTY : {}", e))?;

    // Convertit le fd esclave en File pour pouvoir le dupliquer trois fois.
    let slave_file = File::from(pty.slave);
    let slave_stdin = slave_file
        .try_clone()
        .map_err(|e| anyhow::anyhow!("dup(slave/stdin) : {}", e))?;
    let slave_stderr = slave_file
        .try_clone()
        .map_err(|e| anyhow::anyhow!("dup(slave/stderr) : {}", e))?;

    let mut cmd = Command::new("scp");
    cmd.args(&args)
        .env("TERM", "xterm-256color")
        .stdin(Stdio::from(slave_stdin)) // PTY slave — scp pense être dans un tty
        .stdout(Stdio::from(slave_file)) // idem stdout
        .stderr(Stdio::from(slave_stderr)); // idem stderr

    // Après le fork, avant l'exec, dans le processus enfant :
    //   1) setsid()    — nouveau groupe de session, détache du terminal de contrôle
    //   2) TIOCSCTTY   — fd 1 (= slave PTY, déjà dup2'd par Command) devient le
    //                    terminal de contrôle → can_output() retournera true
    unsafe {
        cmd.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            // fd 1 est déjà le slave PTY (Command::spawn fait les dup2 avant pre_exec)
            if libc::ioctl(1, libc::TIOCSCTTY as libc::c_ulong, 0i32) == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| anyhow::anyhow!("Impossible de lancer scp : {}", e))?;

    let (tx, rx) = mpsc::channel::<ScpEvent>();

    // Le master du PTY reçoit tout ce que scp écrit dans son « terminal ».
    // scp écrase la même ligne avec \r — on lire octet par octet et on
    // découpe sur \r OU \n pour envoyer chaque mise à jour immédiatement.
    let child_pid = child.id();
    let mut master = File::from(pty.master);

    std::thread::spawn(move || {
        // O_NONBLOCK : read() retourne WouldBlock si pas de données,
        // ce qui permet de détecter la fin de scp via try_wait() sans rester bloqué.
        unsafe {
            use std::os::unix::io::AsRawFd;
            libc::fcntl(master.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK);
        }

        let mut buf = Vec::<u8>::with_capacity(256);
        let mut byte = [0u8; 1];
        loop {
            match master.read(&mut byte) {
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // Pas de données disponibles : vérifie si scp est terminé.
                    match child.try_wait() {
                        Ok(Some(_)) => break, // scp terminé, on sort
                        _ => std::thread::sleep(std::time::Duration::from_millis(10)),
                    }
                }
                Ok(0) | Err(_) => {
                    // EOF ou EIO — émet la dernière ligne partielle si présente
                    if !buf.is_empty() {
                        let line = String::from_utf8_lossy(&buf);
                        if let Some(pct) = parse_progress(&line) {
                            let _ = tx.send(ScpEvent::Progress(pct));
                        }
                    }
                    break;
                }
                Ok(_) => match byte[0] {
                    b'\r' | b'\n' => {
                        if !buf.is_empty() {
                            let line = String::from_utf8_lossy(&buf);
                            if let Some(pct) = parse_progress(&line) {
                                let _ = tx.send(ScpEvent::Progress(pct));
                            }
                            buf.clear();
                        }
                    }
                    b => buf.push(b),
                },
            }
        }
        match child.wait() {
            Ok(status) => {
                let _ = tx.send(ScpEvent::Done(status.success()));
            }
            Err(e) => {
                let _ = tx.send(ScpEvent::Error(e.to_string()));
            }
        }
    });

    Ok((rx, child_pid))
}

// ─── Helpers privés ───────────────────────────────────────────────────────────

/// Sépare `"host:port"` → `("host", Some("port"))`.
fn split_host_port(s: &str) -> (&str, Option<&str>) {
    if let Some((host, port)) = s.split_once(':') {
        (host, Some(port))
    } else {
        (s, None)
    }
}

/// Extrait un pourcentage depuis une ligne de sortie `scp`.
///
/// Reconnaît les lignes de progression `scp` du type :
/// ```text
/// filename.tar.gz   38%  125 MB  12.3 MB/s    00:08 ETA
/// ```
fn parse_progress(line: &str) -> Option<u8> {
    let pct_pos = line.find('%')?;
    if pct_pos == 0 {
        return None;
    }
    // Chercher les chiffres immédiatement avant le '%'.
    let before = &line[..pct_pos];
    let digits: String = before
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse::<u8>().ok().filter(|&p| p <= 100)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

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
        }
    }

    // ── build_scp_args ────────────────────────────────────────────────────────

    #[test]
    fn direct_upload_basic() {
        let s = base_server();
        let args = build_scp_args(
            &s,
            ConnectionMode::Direct,
            &ScpDirection::Upload,
            "/tmp/local.txt",
            "/home/admin/remote.txt",
        )
        .unwrap();

        // Dernier arg = destination distante (upload : src local, dst remote)
        assert_eq!(
            args.last().unwrap(),
            "admin@10.0.0.1:/home/admin/remote.txt"
        );
        // Avant-dernier = source locale
        assert_eq!(args[args.len() - 2], "/tmp/local.txt");
    }

    #[test]
    fn direct_download_basic() {
        let s = base_server();
        let args = build_scp_args(
            &s,
            ConnectionMode::Direct,
            &ScpDirection::Download,
            "/tmp/local.txt",
            "/home/admin/remote.txt",
        )
        .unwrap();

        // Download : src = remote, dst = local
        assert_eq!(args.last().unwrap(), "/tmp/local.txt");
        assert_eq!(
            args[args.len() - 2],
            "admin@10.0.0.1:/home/admin/remote.txt"
        );
    }

    #[test]
    fn uses_uppercase_p_for_port() {
        let mut s = base_server();
        s.port = 2222;
        let args =
            build_scp_args(&s, ConnectionMode::Direct, &ScpDirection::Upload, "f", "r").unwrap();
        let p_pos = args.iter().position(|a| a == "-P").expect("-P manquant");
        assert_eq!(args[p_pos + 1], "2222");
        // Pas de -p minuscule
        assert!(!args.contains(&"-p".to_string()));
    }

    #[test]
    fn port_embedded_in_host() {
        let mut s = base_server();
        s.host = "10.0.0.1:2222".into();
        let args =
            build_scp_args(&s, ConnectionMode::Direct, &ScpDirection::Upload, "f", "r").unwrap();
        // Port extrait de l'hôte
        let p_pos = args.iter().position(|a| a == "-P").expect("-P manquant");
        assert_eq!(args[p_pos + 1], "2222");
        // host dans la destination sans le port
        assert!(args.last().unwrap().starts_with("admin@10.0.0.1:"));
    }

    #[test]
    fn no_port_flag_for_22() {
        let s = base_server(); // port = 22
        let args =
            build_scp_args(&s, ConnectionMode::Direct, &ScpDirection::Upload, "f", "r").unwrap();
        assert!(!args.contains(&"-P".to_string()));
    }

    #[test]
    fn jump_mode_includes_j_flag() {
        let mut s = base_server();
        s.jump_host = Some("juser@jump.example.com".into());
        let args =
            build_scp_args(&s, ConnectionMode::Jump, &ScpDirection::Upload, "f", "r").unwrap();
        let j_pos = args.iter().position(|a| a == "-J").expect("-J manquant");
        assert_eq!(args[j_pos + 1], "juser@jump.example.com");
    }

    #[test]
    fn wallix_returns_error() {
        let s = base_server();
        let err = build_scp_args(&s, ConnectionMode::Wallix, &ScpDirection::Upload, "f", "r")
            .unwrap_err();
        assert!(err.to_string().contains("Wallix"));
    }

    #[test]
    fn with_ssh_key() {
        let mut s = base_server();
        s.ssh_key = "/home/user/.ssh/id_ed25519".into();
        let args =
            build_scp_args(&s, ConnectionMode::Direct, &ScpDirection::Upload, "f", "r").unwrap();
        let i_pos = args.iter().position(|a| a == "-i").expect("-i manquant");
        assert_eq!(args[i_pos + 1], "/home/user/.ssh/id_ed25519");
    }

    // ── parse_progress ────────────────────────────────────────────────────────

    #[test]
    fn parse_progress_typical_line() {
        let line = "dump.sql                    38%  125 MB  12.3 MB/s   00:08 ETA";
        assert_eq!(parse_progress(line), Some(38));
    }

    #[test]
    fn parse_progress_100() {
        let line = "dump.sql                   100%  234 MB  15.0 MB/s   00:00    ";
        assert_eq!(parse_progress(line), Some(100));
    }

    #[test]
    fn parse_progress_no_percent() {
        assert_eq!(parse_progress("debug1: Connecting to host..."), None);
    }

    #[test]
    fn parse_progress_percent_at_start() {
        assert_eq!(parse_progress("%invalid"), None);
    }

    #[test]
    fn parse_progress_zero() {
        let line = "backup.tar.gz                    0%    0     0.0KB/s   --:-- ETA";
        assert_eq!(parse_progress(line), Some(0));
    }

    #[test]
    fn ssh_options_added_with_o_flag() {
        let mut s = base_server();
        s.ssh_options = vec!["StrictHostKeyChecking=no".into(), "BatchMode=yes".into()];
        let args =
            build_scp_args(&s, ConnectionMode::Direct, &ScpDirection::Upload, "f", "r").unwrap();
        // Les deux options doivent être introduites par -o
        let o_count = args.iter().filter(|a| *a == "-o").count();
        assert_eq!(o_count, 2, "doit avoir 2 drapeaux -o");
        assert!(args.iter().any(|a| a == "StrictHostKeyChecking=no"));
        assert!(args.iter().any(|a| a == "BatchMode=yes"));
        // La destination reste dernière
        assert!(args.last().unwrap().contains("admin@10.0.0.1:"));
    }

    #[test]
    fn use_system_ssh_config_omits_f_flag() {
        let mut s = base_server();
        s.use_system_ssh_config = true;
        let args =
            build_scp_args(&s, ConnectionMode::Direct, &ScpDirection::Upload, "f", "r").unwrap();
        // -F /dev/null ne doit PAS être présent
        assert!(
            !args.contains(&"-F".to_string()),
            "-F ne doit pas être ajouté quand use_system_ssh_config=true"
        );
    }

    #[test]
    fn f_dev_null_present_when_not_using_system_config() {
        let s = base_server(); // use_system_ssh_config = false
        let args =
            build_scp_args(&s, ConnectionMode::Direct, &ScpDirection::Upload, "f", "r").unwrap();
        let f_pos = args.iter().position(|a| a == "-F").expect("-F absent");
        assert_eq!(args[f_pos + 1], "/dev/null");
    }

    #[test]
    fn jump_missing_host_returns_error() {
        let s = base_server(); // jump_host = None
        let result = build_scp_args(&s, ConnectionMode::Jump, &ScpDirection::Upload, "f", "r");
        assert!(result.is_err(), "Jump sans jump_host doit échouer");
    }
}
