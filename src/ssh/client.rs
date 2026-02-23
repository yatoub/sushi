use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ssh2::Session;

use crate::config::ResolvedServer;

pub struct SshClient {
    pub server: ResolvedServer,
}

impl SshClient {
    pub fn new(server: ResolvedServer) -> Self {
        Self { server }
    }

    pub fn connect(&self) -> Result<()> {
        println!("Connecting to {} ({}) ...", self.server.name, self.server.host);
        
        let tcp = TcpStream::connect(format!("{}:22", self.server.host))
            .context(format!("Failed to connect to {}", self.server.host))?;

        let mut sess = Session::new().context("Failed to create SSH session")?;
        sess.set_tcp_stream(tcp);
        sess.handshake().context("SSH handshake failed")?;

        // Authenticate
        // 1. Try Key File provided in config
        let key_path = shellexpand::tilde(&self.server.ssh_key);
        let p = Path::new(key_path.as_ref());

        // Simple auth strategy: try the key file provided in config
        if sess.userauth_pubkey_file(&self.server.user, None, p, None).is_err() {
            // Fallback: Try agent if key file fails
            // Simplified agent usage
            let mut agent = sess.agent().context("Failed to init SSH agent")?;
            agent.connect().context("Failed to connect to SSH agent")?;
            agent.list_identities().context("Failed to list SSH identities")?;
             
            for identity in agent.identities().context("Failed to get identities")? {
                if agent.userauth(&self.server.user, &identity).is_ok() {
                    break;
                }
            }
        }

        if !sess.authenticated() {
             return Err(anyhow::anyhow!("Authentication failed for {}", self.server.user));
        }

        let mut channel = sess.channel_session().context("Failed to create channel")?;
        
        // Request PTY
        let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
        channel.request_pty("xterm-256color", None, Some((cols.into(), rows.into(), 0, 0)))
            .context("Failed to request PTY")?;
        
        channel.shell().context("Failed to start shell")?;

        // RAW MODE
        // Note: The calling App should probably handle suspending its own UI drawing/events before calling this.
        // We assume we take over the terminal here.
        enable_raw_mode().context("Failed to enable raw mode")?;

        let mut channel_in = channel.clone();
        let (tx, rx) = mpsc::channel();

        // Input Thread: Local Stdin -> SSH Channel
        thread::spawn(move || {
            let mut stdin = io::stdin();
            let mut buf = [0u8; 1024];
            loop {
                match stdin.read(&mut buf) {
                    Ok(n) if n > 0 => {
                        if channel_in.write_all(&buf[..n]).is_err() {
                            break;
                        }
                        let _ = channel_in.flush();
                    }
                    Ok(_) => break, // EOF
                    Err(_) => break, // Error
                }
            }
            let _ = tx.send(()); // Signal exit
        });

        // Main Loop: SSH Channel -> Local Stdout
        let mut stdout = io::stdout();
        let mut buf = [0u8; 1024];
        
        loop {
            // Check if input thread died (e.g. local stdin closed)
            if let Ok(_) | Err(TryRecvError::Disconnected) = rx.try_recv() {
                // If it's empty, it's alive. If disconnected or Ok, it's done.
                 if let Err(TryRecvError::Empty) = rx.try_recv() {
                     // Still running
                 } else {
                     break; 
                 }
            }

            match channel.read(&mut buf) {
                Ok(n) if n > 0 => {
                    stdout.write_all(&buf[..n])?;
                    stdout.flush()?;
                }
                Ok(_) => {
                    if channel.eof() {
                        break;
                    }
                    // Avoid busy wait if blocking read returns 0 (which shouldn't happen unless EOF really)
                    // But ssh2 might behave differently if non-blocking is set (it isn't here).
                    thread::sleep(Duration::from_millis(10));
                }
                Err(_) => break,
            }
        }

        // Cleanup
        disable_raw_mode()?;
        let _ = channel.close();
        let _ = channel.wait_close();
        
        println!("\r\nConnection closed.");
        Ok(())
    }
}
