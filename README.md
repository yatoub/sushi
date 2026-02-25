# 🍣 Sushi

**Sushi** is a modern, terminal-based SSH connection manager written in Rust. It helps you organize your servers into groups and environments, handle complex connection scenarios (jumphosts, bastions), and connect quickly with a beautiful TUI.

![Sushi TUI](https://raw.githubusercontent.com/catppuccin/catppuccin/main/assets/palette/mocha.png) 
*(Note: Uses Catppuccin Mocha theme)*

## ✨ Features

- **Hierarchical Organization**: Structure your infrastructure with Groups, Environments, and Servers.
- **Connection Modes**:
  - **Direct**: Standard SSH connection.
  - **Jump/Rebond**: Connect via a jump host (`-J`).
  - **Bastion**: Connect via a hardened bastion using custom login string.
  - **Mode Inheritance**: Connection mode inherits from defaults → group → environment → server.
- **Configuration Inheritance**: Define defaults globally or at the group/environment level to avoid repetition.
  - Full cascading support for user, ssh_key, mode, port, options, and connection configs.
- **Advanced Search**:
  - **Multi-field Search**: Search by server name OR hostname.
  - **Live Results Counter**: See matching servers in real-time (e.g., "45 / 387 servers").
  - **Visual Feedback**: Color-coded borders (sapphire during search, green for results, red for no match).
  - **Smart Expansion**: Auto-expands groups when searching.
- **Interactive TUI**:
  - **Mouse Support**: Click to select, double-click to connect.
  - **Beautiful UI**: Styled with the Catppuccin Mocha palette.
  - **Verbose Mode**: Toggle SSH verbose output (`-v`) with a single keypress.
  - **Detailed View**: SSH options displayed as a clean, readable list.
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

### Example `~/.sushi.yml`

```yaml
defaults:
  user: "admin"
  ssh_key: "~/.ssh/id_rsa"
  mode: "jump"  # Default connection mode for all servers
  ssh_port: 22
  ssh_options:
    - "StrictHostKeyChecking=no"
    - "UserKnownHostsFile=/dev/null"
  rebond:
    host: "jumphost.example.com"
    user: "jumpuser"

groups:
  - name: "Production"
    user: "prod_user" # Overrides default user
    mode: "direct"    # Override mode for this group
    environments:
      - name: "AWS"
        mode: "bastion"  # Override mode for this environment
        bastion:
          host: "bastion.aws.example.com"
          user: "ubuntu"
          template: "{target_user}@%n:SSH:{bastion_user}"
        servers:
          - name: "Web Server 01"
            host: "10.0.1.5"
            # Inherits bastion mode from environment

  - name: "Staging"
    servers:
      - name: "API Server"
        host: "192.168.1.50"
        # Uses defaults

  # Single server at root
  - name: "My VPS"
    host: "vps.example.org"
    user: "root"
```

### Configuration Breakdown

- **`defaults`**: Global settings applied to all servers unless overridden.
  - `mode`: Default connection mode (`direct`, `jump`, or `bastion`).
  - `rebond`: Jump host configuration (required when using `jump` mode).
  - `bastion`: Bastion configuration (required when using `bastion` mode).
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
| `Esc` | Cancel/clear search |

### Mouse Support

- **Click**: Select server or change tab
- **Double-click**: Connect to selected server

## 🎨 Theme & UI

Sushi uses the **Catppuccin Mocha** color palette for a soothing and high-contrast look.

### Color Scheme

- **Blue**: Default borders
- **Sapphire**: Active search border
- **Green**: Successful search results, verbose mode active, environment headers
- **Red**: No search results
- **Sky**: Active connection mode tab
- **Mauve**: Group headers
- **Surface2**: Selection background

### UI Elements

- **Search Bar**: Dynamic title showing result count ("🔍 45 / 387 servers")
- **Connection Modes**: Tab interface with visual highlight
- **Verbose Toggle**: Checkbox indicator (☐/☑) with color feedback
- **Server Details**: Clean list view for SSH options

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📄 License

This project is licensed under the MIT License.
