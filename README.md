# 🍣 susshi

[![CI](https://github.com/yatoub/susshi/actions/workflows/ci.yml/badge.svg)](https://github.com/yatoub/susshi/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/susshi.svg)](https://crates.io/crates/susshi)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENCE)
[![Security Audit](https://github.com/yatoub/susshi/actions/workflows/ci.yml/badge.svg?event=push&label=audit)](https://github.com/yatoub/susshi/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/yatoub/susshi/branch/master/graph/badge.svg)](https://codecov.io/gh/yatoub/susshi)

**susshi** is a terminal-based SSH connection manager written in Rust — hierarchical inventory, Catppuccin-themed TUI, jump hosts, Wallix bastions, tunnels, and SCP in one place.

![susshi TUI screenshot](docs/susshi.png)

## Installation

```bash
# Linux x86_64
wget https://github.com/yatoub/susshi/releases/latest/download/susshi-linux-amd64
chmod +x susshi-linux-amd64 && sudo mv susshi-linux-amd64 /usr/local/bin/susshi

# Linux x86_64 (musl fallback)
wget https://github.com/yatoub/susshi/releases/latest/download/susshi-linux-amd64-musl

# macOS Apple Silicon
wget https://github.com/yatoub/susshi/releases/latest/download/susshi-macos-arm64

# macOS Intel
wget https://github.com/yatoub/susshi/releases/latest/download/susshi-macos-amd64

# Arch Linux
paru -S susshi-bin
```

For DEB/RPM packages and legacy glibc builds see the [releases page](https://github.com/yatoub/susshi/releases/latest).

> **Windows:** partial support — TUI and config parsing work, interactive SSH (PTY) and Wallix are Unix-only.

## Quick Start

Create `~/.susshi.yml`:

```yaml
defaults:
  user: "ops-user"
  ssh_key: "~/.ssh/id_ed25519"
  theme: mocha  # latte | frappe | macchiato | mocha

groups:
  - name: "Production"
    servers:
      - name: "api-01"
        host: "198.51.100.10"
        mode: "direct"
```

```bash
susshi          # open TUI
susshi --direct ops-user@198.51.100.10   # one-shot connection
```

Full annotated example: [examples/full_config.yaml](examples/full_config.yaml)

## Essential Keybindings

| Key | Action |
| --- | --- |
| `j` / `k` | Move down / up |
| `Enter` | Connect or toggle group |
| `/` | Search |
| `Tab` | Switch mode (Direct / Jump / Wallix) |
| `f` / `F` | Favorite toggle / favorites-only view |
| `T` | Tunnel manager |
| `h` | Keyboard help overlay |
| `q` | Quit |

## Documentation

| Guide | Description |
| --- | --- |
| [docs/configuration.md](docs/configuration.md) | Full config schema, inheritance model, includes, `_vars` |
| [docs/tui.md](docs/tui.md) | TUI navigation, search, keybindings, diagnostics |
| [docs/cli.md](docs/cli.md) | One-shot connection, `--validate`, `--exec-group` |
| [docs/import-export.md](docs/import-export.md) | Import from `~/.ssh/config`, export to Ansible/Terraform/Nmap/CSV |
| [docs/wallix.md](docs/wallix.md) | Wallix bastion configuration and troubleshooting |
| [docs/tunnels.md](docs/tunnels.md) | SSH tunnel configuration and TUI manager |
| [docs/scp.md](docs/scp.md) | In-TUI SCP file transfers |
| [docs/hooks.md](docs/hooks.md) | Pre/post-connect shell hooks |
| [docs/ssh-advanced.md](docs/ssh-advanced.md) | Certificates, agent sockets, ControlMaster, agent forwarding |
| [docs/troubleshooting.md](docs/troubleshooting.md) | Diagnostics, common issues, state file |

## Contributing

Contributions are welcome. Please open a Pull Request — see [CONTRIBUTING.md](CONTRIBUTING.md) for conventions.

## License

This project is licensed under the MIT License.
