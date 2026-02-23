# 🍣 Sushi

**Sushi** is a modern, terminal-based SSH connection manager written in Rust. It helps you organize your servers into groups and environments, handle complex connection scenarios (jumphosts, bastions), and connect quickly with a beautiful TUI.

![Sushi TUI](https://raw.githubusercontent.com/catppuccin/catppuccin/main/assets/palette/mocha.png) 
*(Note: Uses Catppuccin Mocha theme)*

## ✨ Features

- **Hierarchical Organization**: Structure your infrastructure with Groups, Environments, and Servers.
- **Connection Modes**:
  - **Direct**: Standard SSH connection.
  - **Jump/Rebond**: Connect via a jump host (`-J`).
  - **Bastion**: Connect via a hardened bastion using `ProxyCommand`.
- **Configuration Inheritance**: Define defaults globally or at the group/environment level to avoid repetition.
- **Interactive TUI**:
  - **Search**: Quickly filter servers with `/`.
  - **Mouse Support**: Click to select and connect.
  - **Beautiful UI**: Styled with the Catppuccin Mocha palette.
- **Smart Sorting**: Automatically sorts groups and servers alphabetically.

## 🚀 Installation

### Prerequisites

- [Rust & Cargo](https://rustup.rs/) (latest stable version)
- A terminal with truecolor support (e.g., Alacritty, Kitty, WezTerm, iTerm2).

### Build from Source

```bash
git clone https://github.com/yourusername/sushi.git
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
  ssh_port: 22
  ssh_options:
    - "-o StrictHostKeyChecking=no"

groups:
  - name: "Production"
    user: "prod_user" # Overrides default user
    environments:
      - name: "AWS"
        servers:
          - name: "Web Server 01"
            host: "10.0.1.5"
            mode: "bastion" # Force bastion mode
            bastion:
              host: "bastion.aws.example.com"
              user: "ubuntu"
              template: "ssh -W %h:%p {user}@{host}"

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
- **`groups`**: The top-level hierarchy. Can contain `environments` or direct `servers`.
- **`environments`**: A sub-grouping under a Group.
- **`servers`**: The actual connection endpoints.
  - `mode`: Can be `direct`, `jump`, or `bastion`.
    - `jump`: Requires a `rebond` config (host, user) either in defaults or parent.
    - `bastion`: Requires a `bastion` config (host, user, template).

## ⌨️ Keybindings

| Key | Action |
| --- | --- |
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |
| `/` | Search server by name |
| `q` | Quit application |
| `Space` | Toggle expand/collapse group |
| `Enter` | Connect to server / Toggle group |
| `Tab` | Cycle connection mode (Direct/Rebond/Bastion) |
| `1` | Select **Direct** mode |
| `2` | Select **Rebond** mode |
| `3` | Select **Bastion** mode |
| `Esc` | Cancel search |

## 🎨 Theme

Sushi uses the **Catppuccin Mocha** color palette for a soothing and high-contrast look.
- **Blue**: Borders
- **Mauve**: Group Headers
- **Green**: Environment Headers
- **Surface2**: Selection Background

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📄 License

This project is licensed under the MIT License.
