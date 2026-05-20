# Advanced SSH Features

## SSH Certificates (`ssh_cert`)

Pass a signed SSH certificate alongside the private key:

```yaml
servers:
  - name: bastion-jump
    host: jump.example.com
    ssh_key: "~/.ssh/id_ed25519"
    ssh_cert: "~/.ssh/id_ed25519-cert.pub"
```

susshi passes both as `-i <ssh_key> -i <ssh_cert>` to the `ssh` binary. Use this when your CA signs short-lived certificates for authentication.

## SSH Agent Socket (`ssh_agent_sock`)

Route a server's connections through a dedicated SSH agent socket instead of the default `SSH_AUTH_SOCK`:

```yaml
servers:
  - name: secure-host
    host: 198.51.100.50
    ssh_agent_sock: "/run/user/1000/gnupg/S.gpg-agent.ssh"
```

susshi sets `SSH_AUTH_SOCK` to the given path and passes `-o IdentityAgent=<path>` to `ssh`. Useful for:

- GPG-based SSH agents (e.g., `gpg-agent` with `enable-ssh-support`)
- Per-server agent isolation (different keys for different environments)

Unix only. Has no effect on Windows.

## Agent Forwarding (`agent_forwarding`)

Enable SSH agent forwarding with the `-A` flag:

```yaml
defaults:
  agent_forwarding: false  # default

groups:
  - name: "Jump Infrastructure"
    agent_forwarding: true
```

Inheritable at any config level. Avoid enabling globally — forward only to hosts you trust.

## SSH ControlMaster

Reuse SSH connections for the same host. Subsequent connections open nearly instantly without a new handshake.

```yaml
defaults:
  control_master: true
  control_path: "~/.ssh/ctl/%h_%p_%r"  # default socket location
  control_persist: "10m"               # keep master alive 10 min after disconnect
```

susshi automatically creates the parent directory of the socket path.

> **Note:** ControlMaster is not supported in Wallix mode and is automatically disabled for those connections.

## Extra SSH Options (`ssh_options`)

Pass arbitrary `ssh -o` options at any config level:

```yaml
defaults:
  ssh_options:
    - "StrictHostKeyChecking=no"
    - "UserKnownHostsFile=/dev/null"

groups:
  - name: "Trusted LAN"
    ssh_options:
      - "StrictHostKeyChecking=yes"
```

Options at a lower level **replace** the inherited list entirely. To extend the parent list, repeat the inherited options alongside the new ones.

## System SSH Config (`use_system_ssh_config`)

Let `ssh` resolve hosts, users, and ports from `~/.ssh/config`:

```yaml
defaults:
  use_system_ssh_config: false  # default
```

When `true`, susshi does not suppress `~/.ssh/config` — the standard SSH config file is applied for all connections. Useful when you already have a rich `~/.ssh/config` and want susshi to complement it rather than replace it.
