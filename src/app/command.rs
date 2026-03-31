use super::*;

impl App {
    /// Lance une commande SSH non-interactive dans un thread dedie.
    /// Stocke le resultat via `cmd_rx`.
    pub fn start_cmd(&mut self, server: &ResolvedServer, cmd: String) {
        let host = server.host.clone();
        let user = server.user.clone();
        let port = server.port;
        let key = server.ssh_key.clone();
        let cmd_clone = cmd.clone();

        let (tx, rx) = mpsc::channel();
        self.cmd_state = CmdState::Running(cmd.clone());
        self.cmd_rx = Some(rx);

        std::thread::spawn(move || {
            let mut args = vec![
                "-o".to_string(),
                "BatchMode=yes".to_string(),
                "-o".to_string(),
                "ConnectTimeout=10".to_string(),
                "-p".to_string(),
                port.to_string(),
            ];
            if !key.is_empty() {
                let expanded = shellexpand::tilde(&key).to_string();
                args.push("-i".to_string());
                args.push(expanded);
            }
            args.push(format!("{}@{}", user, host));
            args.push(cmd_clone.clone());

            let result = std::process::Command::new("ssh").args(&args).output();

            match result {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                    let combined = if stderr.is_empty() {
                        stdout
                    } else if stdout.is_empty() {
                        stderr
                    } else {
                        format!("{}\n---\n{}", stdout, stderr)
                    };
                    let _ = tx.send((combined, out.status.success()));
                }
                Err(e) => {
                    let _ = tx.send((e.to_string(), false));
                }
            }
        });
    }

    /// Verifie si le thread de commande a produit un resultat et met a jour `cmd_state`.
    pub fn poll_cmd(&mut self) {
        let done = if let Some(rx) = &self.cmd_rx {
            rx.try_recv().ok()
        } else {
            None
        };
        if let Some((output, exit_ok)) = done {
            let cmd = match &self.cmd_state {
                CmdState::Running(c) => c.clone(),
                _ => String::new(),
            };
            self.cmd_state = CmdState::Done {
                cmd,
                output,
                exit_ok,
            };
            self.cmd_rx = None;
        }
    }

    /// Reinitialise l'etat de la commande ad-hoc.
    pub fn reset_cmd(&mut self) {
        self.cmd_state = CmdState::Idle;
        self.cmd_rx = None;
    }
}
