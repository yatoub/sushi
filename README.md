# 🍣 susshi

**susshi** is a modern, terminal-based SSH connection manager written in Rust.
It helps you organize servers, handle complex access flows (jump hosts, Wallix bastions), and connect fast through a clean Catppuccin-powered TUI.

## Table of Contents

- [🍣 susshi](#-susshi)
  - [Table of Contents](#table-of-contents)
  - [Quick Start](#quick-start)
  - [Why susshi](#why-susshi)
    - [Core Features](#core-features)
    - [Productivity Features](#productivity-features)
    - [Advanced Features](#advanced-features)
  - [Installation](#installation)
    - [Pre-built binaries](#pre-built-binaries)
    - [AUR (Arch Linux)](#aur-arch-linux)
    - [Build from source](#build-from-source)
  - [Configuration](#configuration)
    - [Minimal template](#minimal-template)
    - [Configuration docs](#configuration-docs)
  - [CLI Usage](#cli-usage)
  - [TUI Usage](#tui-usage)
    - [Essential keybindings](#essential-keybindings)
    - [Advanced keybindings](#advanced-keybindings)
  - [Advanced Guides](#advanced-guides)
  - [Contributing](#contributing)
  - [License](#license)

## Quick Start

Install and connect in less than 2 minutes.

```bash
# Linux x86_64
wget https://github.com/yatoub/susshi/releases/latest/download/susshi-linux-amd64
chmod +x susshi-linux-amd64
sudo mv susshi-linux-amd64 /usr/local/bin/susshi
```

Create `~/.susshi.yml`:

```yaml
defaults:
  user: "admin"
  theme: mocha

groups:
  - name: "Production"
    servers:
      - name: "api-01"
        host: "10.0.1.10"
        mode: "direct"
```

Use either mode:

```bash
# Open the TUI
susshi

# Direct one-shot connection
susshi --direct admin@10.0.1.10
```

For a complete config example, see [examples/full_config.yaml](examples/full_config.yaml).

## Why susshi

### Core Features

- Hierarchical inventory: groups, environments, servers.
- Mode inheritance: defaults -> group -> environment -> server.
- Connection modes: direct, jump (single or multi-hop), wallix.
- Multi-file config with `includes:` and recursive resolution.
- Hot reload (`r`) without restarting the app.

### Productivity Features

- Search by name/host with live result counter.
- Tag filtering with `#tag` tokens (AND semantics).
- Favorite servers (`f`) and favorites-only mode (`F`).
- Recent-sort view (`H`) by last used server.
- Clipboard copy of generated SSH command (`y`).

### Advanced Features

- Quick diagnostics (`d`) with system stats and filesystem checks.
- Ad-hoc non-interactive SSH command runner (`x`).
- SSH tunnel manager (`T`) with persistent user overrides.
- SCP transfer form (`s`) with live progress.
- Hooks (`pre_connect_hook`, `post_disconnect_hook`).
- `~/.ssh/config` import and Ansible inventory export.
- Variable interpolation with `_vars` and built-in `{{ index }}`.

## Installation

### Pre-built binaries

```bash
# Linux x86_64
wget https://github.com/yatoub/susshi/releases/latest/download/susshi-linux-amd64
chmod +x susshi-linux-amd64
sudo mv susshi-linux-amd64 /usr/local/bin/susshi

# macOS Intel
wget https://github.com/yatoub/susshi/releases/latest/download/susshi-macos-amd64

# macOS Apple Silicon
wget https://github.com/yatoub/susshi/releases/latest/download/susshi-macos-arm64
```

### AUR (Arch Linux)

```bash
paru -S susshi-bin  # pre-compiled binary
paru -S susshi      # build from source
```

### Build from source

Requires [Rust & Cargo](https://rustup.rs/) and a truecolor terminal.

```bash
git clone https://github.com/yatoub/susshi.git
cd susshi
cargo build --release
sudo cp target/release/susshi /usr/local/bin/
```

## Configuration

susshi reads `~/.susshi.yml` by default.

### Minimal template

```yaml
defaults:
  user: "admin"
  ssh_key: "~/.ssh/id_ed25519"
  theme: mocha  # latte | frappe | macchiato | mocha

groups:
  - name: "Infrastructure"
    servers:
      - name: "proxmox-host"
        host: "192.168.1.100"
        mode: "direct"
```

### Configuration docs

- Full annotated reference: [examples/full_config.yaml](examples/full_config.yaml)
- Full configuration guide: [docs/configuration.md](docs/configuration.md)
- Wallix behavior and auto-selection details: [docs/wallix.md](docs/wallix.md)

## CLI Usage

```bash
# Direct / jump / wallix one-shot connection
susshi --direct root@myserver
susshi --jump root@192.168.1.50
susshi --wallix web-01.prod.example.com

# Override SSH parameters
susshi --direct myserver.com --user deploy --port 2222 --key ~/.ssh/deploy_rsa

# Alternate config file
susshi --config ~/work/.susshi.yml

# Show all options
susshi --help
```

Detailed CLI examples: [docs/cli.md](docs/cli.md)

## TUI Usage

### Essential keybindings

| Key | Action |
| --- | --- |
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |
| `Enter` | Connect to server / Toggle group |
| `Space` | Toggle expand/collapse group |
| `/` | Enter search mode |
| `Esc` | Cancel search or close overlays |
| `v` | Toggle verbose SSH mode |
| `q` | Quit |

### Advanced keybindings

| Key | Action |
| --- | --- |
| `Tab`, `1`, `2`, `3` | Switch connection mode |
| `d` | Quick diagnostics |
| `x` | Ad-hoc SSH command |
| `T` | Tunnel manager |
| `s` | SCP transfer form |
| `f` / `F` | Favorite toggle / favorites-only view |
| `r` | Hot-reload configuration |
| `C` | Collapse all |
| `H` | Toggle recent-sort |
| `y` | Copy SSH command |

Detailed TUI behavior and visuals: [docs/tui.md](docs/tui.md)

## Advanced Guides

- Configuration model and inheritance: [docs/configuration.md](docs/configuration.md)
- Wallix matching and fallback flow: [docs/wallix.md](docs/wallix.md)
- Full CLI cookbook (import/export included): [docs/cli.md](docs/cli.md)
- TUI interactions, diagnostics, tunnels and SCP: [docs/tui.md](docs/tui.md)

## Contributing

Contributions are welcome. Please open a Pull Request.

## License

This project is licensed under the MIT License.
