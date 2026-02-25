# 🍣 Sushi

**Sushi** is a modern, terminal-based SSH connection manager written in Rust. It helps you organize your servers into groups and environments, handle complex connection scenarios (jumphosts, bastions), and connect quickly with a beautiful TUI with Catppuccin theme.

## ✨ Features

- **Hierarchical Organization**: Structure your infrastructure with Groups, Environments, and Servers.
- **Connection Modes**:
  - **Direct**: Standard SSH connection.
  - **Jump/Rebond**: Connect via a jump host (`-J`).
  - **Bastion**: Connect via a hardened bastion using custom login string.
  - **Mode Inheritance**: Connection mode inherits from defaults → group → environment → server.
- **Configuration Inheritance**: Define defaults globally or at the group/environment level to avoid repetition.
  - Full cascading support for user, ssh_key, mode, port, options, and connection configs.
  - `use_system_ssh_config`: opt in to respecting `~/.ssh/config` (ControlMaster, aliases…).
- **CLI — connect without opening the TUI**:
  - `--direct`, `--rebond`, `--bastion` `[user@]host[:port]` — instant SSH connection.
  - `-u/--user`, `-p/--port`, `-k/--key` — override any SSH parameter.
  - `-c/--config` — use an alternate configuration file.
  - `-v/--verbose` — enable SSH verbose output.
- **Advanced Search**:
  - **Multi-field Search**: Search by server name OR hostname.
  - **Live Results Counter**: See matching servers in real-time (e.g., "45 / 387 servers").
  - **Visual Feedback**: Color-coded borders (sapphire during search, green for results, red for no match).
  - **Smart Expansion**: Auto-expands groups when searching.
- **Interactive TUI**:
  - **Mouse Support**: Click to select, double-click to connect.
  - **Configurable Theme**: Choose from four Catppuccin flavors — `latte`, `frappe`, `macchiato`, or `mocha` (default) via `defaults.theme` in your config.
  - **Verbose Mode**: Toggle SSH verbose output with `v`.
  - **Rich Detail Pane**: Shows port (highlighted when non-standard), connection mode, jump host, and bastion host.
  - **Clipboard**: Copy the SSH command for any server with `y` (requires a running clipboard manager on Linux).
- **State Persistence**: Expanded groups are saved to `~/.sushi_state.json` and restored on next launch.
- **In-TUI Error Screen**: Connection errors are displayed as an overlay instead of crashing — press `Enter`/`Esc`/`q` to dismiss.
- **Smart Sorting**: Automatically sorts groups and servers alphabetically.

## 🚀 Installation

### Prerequisites

- [Rust & Cargo](https://rustup.rs/)
- A terminal with truecolor support (e.g., Alacritty, Kitty, WezTerm, iTerm2, Tilix).

### Build from Source

```bash
git clone https://github.com/yatoub/sushi.git
cd sushi
cargo build --release
sudo cp target/release/sushi /usr/local/bin/
```

## ⚙️ Configuration

Sushi looks for a configuration file at `~/.sushi.yml`.  
A fully annotated example covering every feature is available at [`examples/full_config.yaml`](examples/full_config.yaml).

### Example `~/.sushi.yml`

```yaml
defaults:
  user: "admin"
  ssh_key: "~/.ssh/id_ed25519"
  ssh_port: 22
  theme: mocha  # latte | frappe | macchiato | mocha (default)
  ssh_options:
    - "StrictHostKeyChecking=no"
    - "UserKnownHostsFile=/dev/null"
  # Set to true to honour ~/.ssh/config (ControlMaster, aliases, etc.)
  use_system_ssh_config: false
  rebond:
    - host: "jump.example.com"
      user: "jump"
  # Multi-hop example:
  # rebond:
  #   - host: "jump1.example.com"
  #     user: "jump"
  #   - host: "jump2.example.com"
  #     user: "jump"
  bastion:
    host: "bastion.example.com"
    user: "bastion"

groups:
  # Level 3: Group → Environment → Server
  - name: "Projet Alpha"
    user: "dev"             # overrides default user for this group
    environments:
      - name: "Production"
        servers:
          - name: "web-01"
            host: "192.168.1.10"
            mode: "direct"
          - name: "db-01"
            host: "192.168.1.11"  # inherits mode from defaults
      - name: "Staging"
        servers:
          - name: "web-stg"
            host: "192.168.1.20"
            mode: "jump"          # override at server level

  # Level 2: Group → Server
  - name: "Infrastructure"
    servers:
      - name: "proxmox-host"
        host: "192.168.1.100"
        mode: "direct"
      - name: "internal-nas"
        host: "192.168.1.200"
        user: "root"
        mode: "bastion"

  # Level 1: Single server at root
  - name: "Raspberry-Pi-Home"
    host: "raspberrypi.local"
    user: "pi"
    mode: "direct"
```

> See [`examples/full_config.yaml`](examples/full_config.yaml) for a complete reference with all options and inline comments.

### Configuration Breakdown

- **`defaults`**: Global settings applied to all servers unless overridden.
  - `mode`: Default connection mode (`direct`, `jump`, or `bastion`).
  - `theme`: UI color theme — `latte`, `frappe`, `macchiato`, or `mocha` (default).
  - `rebond`: Jump host configuration (required when using `jump` mode).
  - `bastion`: Bastion configuration (required when using `bastion` mode).
  - `use_system_ssh_config`: Set to `true` to honour `~/.ssh/config` instead of passing `-F /dev/null`. Defaults to `false`.
- **`groups`**: The top-level hierarchy. Can contain `environments` or direct `servers`.
  - Can override any default setting including `mode`.
- **`environments`**: A sub-grouping under a Group.
  - Inherits from group and can override any setting.
- **`servers`**: The actual connection endpoints.
  - Inherits all settings through the chain: defaults → group → environment → server.
  - `mode`: Connection mode inherits but can be overridden at server level.

## ⌨️ Keybindings

| Key | Action |
| --- | --- |
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |
| `/` | Enter search mode (search by name or hostname) |
| `v` | Toggle verbose SSH mode (`-v`) |
| `q` | Quit application |
| `Space` | Toggle expand/collapse group |
| `Enter` | Connect to server / Toggle group |
| `Tab` | Cycle connection mode (Direct/Rebond/Bastion) |
| `1` | Select **Direct** mode |
| `2` | Select **Rebond** mode |
| `3` | Select **Bastion** mode |
| `Ctrl+U` | Clear search query (while in search mode) |
| `y` | Copy SSH command to clipboard |
| `Esc` | Cancel search / dismiss error overlay |

### Mouse Support

- **Click**: Select server or change tab
- **Double-click**: Connect to selected server

## 🖥️ CLI Usage

Sushi can connect directly without opening the TUI:

```bash
# Connect directly
sushi --direct root@myserver
sushi --direct admin@10.0.1.5:2222

# Connect via jump host
sushi --rebond root@192.168.1.50

# Connect via bastion
sushi --bastion web-01.prod.example.com

# Override SSH parameters
sushi --direct myserver.com --user deploy --port 2222 --key ~/.ssh/deploy_rsa

# Use a custom config file
sushi --config ~/work/.sushi.yml

# Show all options
sushi --help
```

## 🎨 Theme & UI

Sushi uses the [Catppuccin](https://github.com/catppuccin/catppuccin) palette. Choose a flavor in your config:

```yaml
defaults:
  theme: mocha   # latte | frappe | macchiato | mocha (default)
```

### Color Scheme (Mocha)

- **Blue**: Default borders
- **Sapphire**: Active search border
- **Green**: Successful search results, verbose mode active, environment headers
- **Red**: No search results, error overlay border
- **Sky**: Active connection mode tab, jump / bastion host in detail pane
- **Yellow**: Port number when different from 22
- **Mauve**: Group headers
- **Surface2**: Selection background

### UI Elements

- **Search Bar**: Dynamic title showing result count ("🔍 45 / 387 servers")
- **Connection Modes**: Tab interface with visual highlight
- **Verbose Toggle**: Checkbox indicator (☐/☑) with color feedback
- **Detail Pane**: Port, mode, identity file, jump/bastion host, SSH options
- **Error Overlay**: Centered popup for connection errors — press `Enter` to dismiss

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📄 License

This project is licensed under the MIT License.
