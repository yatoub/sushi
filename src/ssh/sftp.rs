//! Transfert de fichiers SFTP natif via `libssh2`.
//!
//! Remplace le spawn système `scp` de [`crate::ssh::scp`] par une session
//! SSH pure Rust utilisant la crate `ssh2` (wrapper autour de `libssh2`).
//!
//! ## Interface publique
//!
//! [`spawn_sftp`] retourne un [`std::sync::mpsc::Receiver<ScpEvent>`] identique
//! à celui de `spawn_scp`, ce qui permet de remplacer l'appel côté [`crate::app::App`]
//! sans modifier la logique de polling ni l'UI.
//!
//! ## Avantages vs spawn `scp`
//!
//! - Progression précise (taille du fichier connue dès l'ouverture).
//! - Aucune dépendance sur l'utilitaire `scp` installé (OpenSSH ≥ 9.0 le déprécie).
//! - Gestion propre des erreurs SFTP (codes d'état standardisés).
//! - Compatible avec les serveurs SFTP-only (sans shell).
//!
//! ## Authentification (par priorité)
//!
//! 1. Agent SSH (`$SSH_AUTH_SOCK`)
//! 2. Clé explicite dans `server.ssh_key` (tilde expandé)
//! 3. Clés par défaut : `~/.ssh/id_ed25519`, `~/.ssh/id_rsa`, `~/.ssh/id_ecdsa`
//!
//! ## Modes supportés
//!
//! | Mode     | Support |
//! |----------|---------|
//! | Direct   | ✔ session SSH2 directe |
//! | Jump     | ✔ `direct-tcpip` via loopback bridge |
//! | Wallix   | ✖ retourne une erreur |

use crate::config::{ConnectionMode, ResolvedServer};
use crate::ssh::scp::{ScpDirection, ScpEvent};
use anyhow::Result;
use std::sync::mpsc;

// ─── API publique ─────────────────────────────────────────────────────────────

/// Lance un transfert SFTP en arrière-plan dans un thread dédié.
///
/// Retourne immédiatement un [`mpsc::Receiver<ScpEvent>`] qui émettra :
/// - [`ScpEvent::Progress`] (0–100) à chaque chunk de 64 KiB transféré
/// - [`ScpEvent::Done`] à la fin (succès)
/// - [`ScpEvent::Error`] si une erreur irrécupérable survient
///
/// Contrairement à `spawn_scp`, aucun PID n'est retourné (pas de sous-processus).
/// Le thread SFTP peut être abandonné en droppant le `Receiver` — il détectera
/// `SendError` et se terminera proprement.
///
/// **Mode Wallix non supporté** — retourne une erreur immédiatement.
#[cfg(unix)]
pub fn spawn_sftp(
    server: &ResolvedServer,
    mode: ConnectionMode,
    direction: ScpDirection,
    local: &str,
    remote: &str,
) -> Result<mpsc::Receiver<ScpEvent>> {
    if mode == ConnectionMode::Wallix {
        anyhow::bail!("SFTP non disponible en mode Wallix");
    }

    let server = server.clone();
    let local = shellexpand::tilde(local).into_owned();
    let remote = remote.to_string();

    let (tx, rx) = mpsc::channel::<ScpEvent>();

    std::thread::spawn(move || {
        let result = transfer_inner(&server, mode, &direction, &local, &remote, &tx);
        match result {
            Ok(()) => {
                let _ = tx.send(ScpEvent::Done(true));
            }
            Err(e) => {
                let _ = tx.send(ScpEvent::Error(e.to_string()));
            }
        }
    });

    Ok(rx)
}

/// Stub non-Unix — SFTP natif non disponible.
#[cfg(not(unix))]
pub fn spawn_sftp(
    _server: &ResolvedServer,
    _mode: ConnectionMode,
    _direction: ScpDirection,
    _local: &str,
    _remote: &str,
) -> Result<mpsc::Receiver<ScpEvent>> {
    anyhow::bail!("SFTP non disponible sur cette plateforme")
}

// ─── Implémentation Unix ──────────────────────────────────────────────────────

#[cfg(unix)]
fn transfer_inner(
    server: &ResolvedServer,
    mode: ConnectionMode,
    direction: &ScpDirection,
    local: &str,
    remote: &str,
    tx: &mpsc::Sender<ScpEvent>,
) -> Result<()> {
    use ssh2::Session;
    use std::fs::File;
    use std::path::{Path, PathBuf};

    // ── 1. Ouvre la session SSH ───────────────────────────────────────────────
    let sess: Session = open_session(server, mode)?;

    // ── 2. Ouvre le sous-système SFTP ────────────────────────────────────────
    let sftp = sess.sftp()?;

    // ── 3. Résolution du chemin distant (tilde → chemin absolu) ──────────────
    let raw_remote: PathBuf = resolve_remote_path(&sftp, remote);

    // Si le chemin distant se termine par '/' ou pointe vers un répertoire
    // existant, on y ajoute le nom du fichier local (comportement scp).
    let remote_path: PathBuf =
        if remote.ends_with('/') || remote.ends_with('\\') || raw_remote.as_os_str().is_empty() {
            // Chemin explicitement terminé par slash → c'est forcément un dossier.
            let filename = Path::new(local)
                .file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new("transfer"));
            raw_remote.join(filename)
        } else {
            // On tente un stat() : si le chemin distant est un répertoire existant,
            // on y ajoute le nom du fichier local.
            match sftp.stat(&raw_remote) {
                Ok(stat) if stat.is_dir() => {
                    let filename = Path::new(local)
                        .file_name()
                        .unwrap_or_else(|| std::ffi::OsStr::new("transfer"));
                    raw_remote.join(filename)
                }
                _ => raw_remote,
            }
        };

    // ── 4. Transfert avec progression ────────────────────────────────────────
    match direction {
        ScpDirection::Upload => {
            let local_path = Path::new(local);
            let file_size = std::fs::metadata(local_path).map(|m| m.len()).unwrap_or(0);
            let mut src = File::open(local_path)?;
            let mut dst = sftp.create(&remote_path)?;
            copy_with_progress(&mut src, &mut dst, file_size, tx)?;
        }
        ScpDirection::Download => {
            let file_size = sftp
                .stat(&remote_path)
                .map(|s| s.size.unwrap_or(0))
                .unwrap_or(0);
            let mut src = sftp.open(&remote_path)?;
            let mut dst = File::create(Path::new(local))?;
            copy_with_progress(&mut src, &mut dst, file_size, tx)?;
        }
    }

    Ok(())
}

/// Ouvre une session SSH authentifiée selon le mode de connexion.
#[cfg(unix)]
fn open_session(server: &ResolvedServer, mode: ConnectionMode) -> Result<ssh2::Session> {
    use std::net::TcpStream;

    match mode {
        ConnectionMode::Direct => {
            let (host, port) = resolve_host_port(server);
            let tcp = TcpStream::connect(format!("{}:{}", host, port))?;
            let mut sess = ssh2::Session::new()?;
            sess.set_tcp_stream(tcp);
            sess.handshake()?;
            auth_session(&mut sess, &server.user, &server.ssh_key)?;
            Ok(sess)
        }
        ConnectionMode::Jump => {
            let jump_str = server
                .jump_host
                .as_deref()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| anyhow::anyhow!("Jump host non configuré pour ce serveur"))?;
            open_session_via_jump(jump_str, server)
        }
        ConnectionMode::Wallix => anyhow::bail!("SFTP non disponible en mode Wallix"),
    }
}

/// Extrait `(host, port)` depuis `server.host` (qui peut contenir `"host:port"`).
#[cfg(unix)]
fn resolve_host_port(server: &ResolvedServer) -> (String, u16) {
    if let Some((h, p)) = server.host.split_once(':') {
        (h.to_string(), p.parse::<u16>().unwrap_or(server.port))
    } else {
        (server.host.clone(), server.port)
    }
}

/// Authentifie la session SSH par priorité :
/// 1. Agent SSH (`$SSH_AUTH_SOCK`)
/// 2. Clé explicite (`ssh_key`, tilde expandé)
/// 3. Clés par défaut (`~/.ssh/id_ed25519`, `~/.ssh/id_rsa`, `~/.ssh/id_ecdsa`)
#[cfg(unix)]
fn auth_session(sess: &mut ssh2::Session, username: &str, ssh_key: &str) -> Result<()> {
    use std::path::PathBuf;

    // 1) Agent SSH
    if let Ok(mut agent) = sess.agent()
        && agent.connect().is_ok()
        && agent.list_identities().is_ok()
    {
        let identities = agent.identities().unwrap_or_default();
        for identity in &identities {
            if agent.userauth(username, identity).is_ok() && sess.authenticated() {
                return Ok(());
            }
        }
    }

    // 2) Clé explicite
    if !ssh_key.is_empty() {
        let expanded = shellexpand::tilde(ssh_key).to_string();
        let key_path = PathBuf::from(&expanded);
        if sess
            .userauth_pubkey_file(username, None, &key_path, None)
            .is_ok()
            && sess.authenticated()
        {
            return Ok(());
        }
    }

    // 3) Clés par défaut
    for key in &["~/.ssh/id_ed25519", "~/.ssh/id_rsa", "~/.ssh/id_ecdsa"] {
        let expanded = shellexpand::tilde(key).to_string();
        let key_path = PathBuf::from(&expanded);
        if key_path.exists()
            && sess
                .userauth_pubkey_file(username, None, &key_path, None)
                .is_ok()
            && sess.authenticated()
        {
            return Ok(());
        }
    }

    anyhow::bail!(
        "Authentification SSH échouée pour {} (agent SSH + clés par défaut épuisés)",
        username
    )
}

/// Ouvre une session SSH vers la cible en passant par un jump host.
///
/// ## Principe
///
/// 1. Connexion TCP → jump host, session SSH2, authentification.
/// 2. Ouverture d'un channel `direct-tcpip` vers la cible.
/// 3. Création d'un listener loopback éphémère sur `127.0.0.1:0`.
/// 4. Thread bridge : accepte la connexion loopback et copie bidirectionnellement
///    entre le socket loopback et le channel `direct-tcpip`.
/// 5. La session cible se connecte au listener loopback (qui voit un TcpStream normal).
///
/// La session du jump host et le channel restent vivants dans le thread bridge
/// pour toute la durée du transfert.
#[cfg(unix)]
fn open_session_via_jump(jump_str: &str, server: &ResolvedServer) -> Result<ssh2::Session> {
    use std::net::{TcpListener, TcpStream};

    // ── Parse le premier saut (multi-hop : seul le premier est utilisé en v0.11) ──
    let first = jump_str.split(',').next().unwrap_or(jump_str);
    let (jump_user, jump_host_port) = match first.split_once('@') {
        Some((u, hp)) => (u, hp),
        None => (server.user.as_str(), first),
    };
    let (jump_host, jump_port) = match jump_host_port.split_once(':') {
        Some((h, p)) => (h, p.parse::<u16>().unwrap_or(22)),
        None => (jump_host_port, 22u16),
    };

    // ── 1. Session SSH vers le jump host ──────────────────────────────────────
    let jump_tcp = TcpStream::connect(format!("{}:{}", jump_host, jump_port))?;
    let mut jump_sess = ssh2::Session::new()?;
    jump_sess.set_tcp_stream(jump_tcp);
    jump_sess.handshake()?;
    auth_session(&mut jump_sess, jump_user, &server.ssh_key)?;

    // ── 2. Channel direct-tcpip vers la cible ─────────────────────────────────
    let (target_host, target_port) = resolve_host_port(server);
    let channel = jump_sess.channel_direct_tcpip(&target_host, target_port, None)?;

    // ── 3. Listener loopback éphémère ─────────────────────────────────────────
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let local_addr = listener.local_addr()?;

    // ── 4. Thread bridge ──────────────────────────────────────────────────────
    std::thread::spawn(move || {
        // `jump_sess` et `channel` sont déplacés dans `bridge_bidirectional`
        // qui les garde vivants pour toute la durée du transfert.
        if let Ok((stream, _)) = listener.accept() {
            bridge_bidirectional(jump_sess, channel, stream);
        }
    });

    // ── 5. Session cible via le loopback ──────────────────────────────────────
    let target_tcp = TcpStream::connect(local_addr)?;
    let mut target_sess = ssh2::Session::new()?;
    target_sess.set_tcp_stream(target_tcp);
    target_sess.handshake()?;
    auth_session(&mut target_sess, &server.user, &server.ssh_key)?;

    Ok(target_sess)
}

/// Copie bidirectionnelle non-bloquante entre un channel SSH2 et un TcpStream.
///
/// Utilisé par le thread bridge dans [`open_session_via_jump`].
/// Utilise un polling non-bloquant avec sleep de 1 ms quand aucun octet n'est
/// disponible dans les deux sens (acceptable pour des transferts SFTP).
#[cfg(unix)]
fn bridge_bidirectional(
    sess: ssh2::Session,
    mut channel: ssh2::Channel,
    mut stream: std::net::TcpStream,
) {
    use std::io::{ErrorKind, Read, Write};
    use std::time::Duration;

    sess.set_blocking(false);
    stream.set_nonblocking(true).ok();
    stream.set_nodelay(true).ok();

    let mut buf = vec![0u8; 4096];

    loop {
        let mut idle = true;

        // channel → stream
        match channel.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                idle = false;
                if stream.write_all(&buf[..n]).is_err() {
                    break;
                }
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {}
            Err(_) => break,
        }

        // stream → channel
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                idle = false;
                if channel.write_all(&buf[..n]).is_err() {
                    break;
                }
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {}
            Err(_) => break,
        }

        if idle {
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    let _ = channel.close();
}

/// Tente d'expand le `~` du chemin distant via `sftp.realpath(".")`.
///
/// Si le serveur SFTP ne supporte pas `realpath`, retourne le chemin tel quel
/// (la plupart des serveurs modernes le supportent).
#[cfg(unix)]
fn resolve_remote_path(sftp: &ssh2::Sftp, remote: &str) -> std::path::PathBuf {
    use std::path::PathBuf;

    if remote.starts_with('~') {
        let home = sftp
            .realpath(std::path::Path::new("."))
            .unwrap_or_else(|_| PathBuf::from(""));
        if remote == "~" {
            home
        } else {
            // "~/foo/bar" → home.join("foo/bar")
            let tail = remote
                .trim_start_matches("~/")
                .trim_start_matches('~')
                .trim_start_matches('/');
            home.join(tail)
        }
    } else {
        PathBuf::from(remote)
    }
}

/// Copie `src` vers `dst` par chunks de 64 KiB en émettant [`ScpEvent::Progress`].
///
/// N'émet un événement que si le pourcentage change (évite de saturer le channel).
/// Garantit l'émission de `100%` en fin de transfert même si `total == 0`.
#[cfg(unix)]
fn copy_with_progress(
    src: &mut dyn std::io::Read,
    dst: &mut dyn std::io::Write,
    total: u64,
    tx: &mpsc::Sender<ScpEvent>,
) -> Result<()> {
    let mut buf = vec![0u8; 65536];
    let mut transferred: u64 = 0;
    let mut last_pct: u8 = 0;

    loop {
        let n = src.read(&mut buf)?;
        if n == 0 {
            break;
        }
        dst.write_all(&buf[..n])?;
        transferred += n as u64;

        let pct = if total > 0 {
            ((transferred * 100) / total).min(100) as u8
        } else {
            0
        };

        if pct != last_pct {
            // Arrête si le receiver est droppé (transfert annulé côté TUI).
            if tx.send(ScpEvent::Progress(pct)).is_err() {
                anyhow::bail!("transfert annulé");
            }
            last_pct = pct;
        }
    }

    // Garantit 100% même si total=0 ou par arrondi.
    if last_pct < 100 {
        let _ = tx.send(ScpEvent::Progress(100));
    }

    Ok(())
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConnectionMode;

    /// Vérifie que `spawn_sftp` retourne immédiatement une erreur en mode Wallix
    /// sans lancer de thread ni de connexion réseau.
    #[test]
    #[cfg(unix)]
    fn wallix_returns_error_immediately() {
        let server = base_server();
        let result = spawn_sftp(
            &server,
            ConnectionMode::Wallix,
            ScpDirection::Upload,
            "/tmp/file",
            "/remote/file",
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Wallix"));
    }

    /// Vérifie que `spawn_sftp` retourne une erreur (via le channel) quand le
    /// serveur SSH est injoignable — sans bloquer indéfiniment.
    #[test]
    #[cfg(unix)]
    fn unreachable_server_emits_error_event() {
        let mut server = base_server();
        // Port invalide sur loopback — connexion refusée quasi-instantanément.
        server.host = "127.0.0.1".into();
        server.port = 1; // port fermé garanti

        let rx = spawn_sftp(
            &server,
            ConnectionMode::Direct,
            ScpDirection::Upload,
            "/tmp/file",
            "/remote/file",
        )
        .expect("spawn_sftp ne doit pas échouer immédiatement");

        // Le thread doit émettre un ScpEvent::Error (connexion refusée).
        let event = rx.recv_timeout(std::time::Duration::from_secs(5));
        assert!(
            matches!(event, Ok(ScpEvent::Error(_))),
            "attendu ScpEvent::Error, obtenu {:?}",
            event
        );
    }

    fn base_server() -> ResolvedServer {
        ResolvedServer {
            namespace: String::new(),
            group_name: String::new(),
            env_name: String::new(),
            name: "test".into(),
            host: "127.0.0.1".into(),
            user: "test".into(),
            port: 22,
            ssh_key: String::new(),
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
}
