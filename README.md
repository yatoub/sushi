# 🍣 susshi

**susshi** is a modern, terminal-based SSH connection manager written in Rust. It helps you organize your servers into groups and environments, handle complex connection scenarios (jumphosts, bastions), and connect quickly with a beautiful TUI with Catppuccin theme.

## ✨ Features

- **Hierarchical Organization**: Structure your infrastructure with Groups, Environments, and Servers.
- **Multi-file Configuration**: Split your config by team or perimeter with `includes:`. Each included file is shown as a collapsible **namespace** in the TUI. Nested includes are resolved recursively. Missing or circular includes are non-fatal warnings.
- **Connection Modes**:
  - **Direct**: Standard SSH connection.
  - **Jump/Rebond**: Connect via one or more jump hosts (`-J`). Supports multi-hop chains.
  - **Wallix**: Connect via a Wallix server using custom login string.
  - **Mode Inheritance**: Connection mode inherits from defaults → group → environment → server.
- **Configuration Inheritance**: Define defaults globally or at the group/environment level to avoid repetition.
  - Full cascading support for user, ssh_key, mode, port, options, and connection configs.
  - `use_system_ssh_config`: opt in to respecting `~/.ssh/config` (ControlMaster, aliases…).
- **CLI — connect without opening the TUI**:
  - `--direct`, `--jump`, `--wallix` `[user@]host[:port]` — instant SSH connection.
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
  - **Rich Detail Pane**: Shows port (highlighted when non-standard), connection mode, jump host, bastion host, SSH options, and last connection timestamp.
  - **Quick Diagnostic** (`d`): Press `d` on any server to run a non-blocking SSH probe. The detail pane displays a **System** block with kernel version, OS name and version (from `/etc/os-release`), CPU model and logical core count, load average, and color-coded RAM/Disk progress bars (green < 60%, yellow 60–85%, red > 85%). Additional mount points configured via `probe_filesystems` are also shown: each extra path gets its own bar, or a yellow `⚠ /path — not mounted` warning if absent. An animated spinner shows while the probe is running. Press `d` again to refresh; changing server resets it.
  - **Ad-hoc Command** (`x`): Run any non-interactive SSH command on the selected server directly from the TUI. The output (up to 20 lines) is displayed in the detail pane with a colored exit status indicator.
  - **Clipboard**: Copy the SSH command for any server with `y` (requires a running clipboard manager on Linux).
  - **Hot Reload** (`r`): Reload all configuration files (main + includes) without restarting. The tree updates in place and the current expansion state is preserved.
  - **Favorites** (`f` / `F`): Mark any server as a favorite (⭐). Press `F` to toggle the favorites-only view — the tree filters all groups, environments, and namespaces accordingly.
  - **Collapse All** (`C`): Press `C` to instantly collapse all expanded groups, namespaces, and environments and jump back to the top of the list.
  - **Recent Sort** (`H`): Switch between alphabetical order and a flat list sorted by most-recently-used server.
- **Connection History**: The last connection timestamp for each server is stored in `~/.susshi_state.json` and displayed in the detail pane (e.g., "il y a 2 h" / "2 h ago").
- **YAML Validation**: Unknown fields in any config file are detected and reported as non-blocking `ValidationWarning` entries at startup.
- **State Persistence**: Expanded groups, favorites, last-seen timestamps, and sort mode are saved to `~/.susshi_state.json` and restored on next launch.
- **Keep-Open Mode**: Set `keep_open: true` in `defaults` to automatically reopen the TUI after a connection closes — no need to relaunch susshi to switch to another server.
- **In-TUI Error Screen**: Connection errors are displayed as an overlay instead of crashing — press `Enter`/`Esc`/`q` to dismiss.
- **Smart Sorting**: Automatically sorts groups and servers alphabetically.

## 🚀 Installation

### Pre-built binaries

Download the latest binary directly — the URL always points to the most recent release:

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

### Package manager (AUR)

For Arch Linux users:

```bash
paru -S susshi-bin  # pre-compiled binary
paru -S susshi      # build from source
```

### Build from Source

Requires [Rust & Cargo](https://rustup.rs/) and a terminal with truecolor support (e.g., Alacritty, Kitty, WezTerm, iTerm2, Tilix).

```bash
git clone https://github.com/yatoub/susshi.git
cd susshi
cargo build --release
sudo cp target/release/susshi /usr/local/bin/
```

## ⚙️ Configuration

susshi looks for a configuration file at `~/.susshi.yml`.  
A fully annotated example covering every feature is available at [`examples/full_config.yaml`](examples/full_config.yaml).

### Multi-file configuration — `includes`

Split a large config into one YAML file per team or perimeter and reference them from the main file:

```yaml
# ~/.susshi.yml
includes:
  - label: "DEV"
    path: "~/.susshi_dev.yml"
  - label: "QUALIF"
    path: "~/.susshi_qualif.yml"
    merge_defaults: true   # propagate main-file defaults into this sub-file

defaults:
  user: "admin"

groups:
  - name: "Local"
    servers:
      - name: "dev-vm"
        host: "192.168.56.10"
```

- **`label`** — text shown as the namespace header (📦) in the TUI.
- **`path`** — absolute or `~`-expanded path. Relative paths are resolved from the directory of the main file.
- **`merge_defaults`** *(optional, default: `false`)* — when `true`, the main file's `defaults` are merged as a base layer for the included file's servers (still lower priority than the sub-file's own defaults and any group/env/server-level overrides).
- Each included file is a standard susshi YAML (`defaults`, `groups`…). Its `defaults` are **local** unless `merge_defaults: true` is set.
- Includes inside an included file are resolved **recursively**. Circular dependencies are detected and reported as non-blocking warnings.
- If a file is missing or unreadable, the remaining includes still load normally — a warning overlay is shown at startup.

### Example `~/.susshi.yml`

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
  # Set to true to reopen the TUI after each connection closes
  keep_open: false
  jump:
    - host: "jump.example.com"
      user: "jump"
  # Multi-hop example:
  # jump:
  #   - host: "jump1.example.com"
  #     user: "jump"
  #   - host: "jump2.example.com"
  #     user: "jump"
  wallix:
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
>
> **⚠️ Breaking change (v0.5.0)** — `jump` (formerly `rebond`) is now a **list** of jump hosts,
> even for a single hop. If you used the old map syntax, wrap it in a list:

```yaml
# Before (v0.4.x)
jump:
  host: "jump.example.com"
  user: "jump"

# After (v0.5.0+)
jump:
  - host: "jump.example.com"
    user: "jump"
```

### Configuration Breakdown

- **`includes`**: *(optional)* List of external YAML files to merge as namespaces.
  - `label`: Display name shown as the top-level collapsible namespace (📦) in the tree.
  - `path`: Path to the included file (absolute or `~`-expanded). Relative paths are resolved from the main file's directory.
  - `merge_defaults` *(optional)*: when `true`, the main file's `defaults` are applied as a base layer for the sub-file's servers.
  - The included file uses the same YAML schema as the main file. Its `defaults` apply only to that file's servers (unless `merge_defaults` is enabled). Nested includes are resolved recursively.
  - Circular dependencies are detected and reported as non-blocking warnings at startup.
  - Unknown YAML fields in any config file generate non-blocking `ValidationWarning` entries.
- **`defaults`**: Global settings applied to all servers unless overridden.
  - `mode`: Default connection mode (`direct`, `jump`, or `wallix`).
  - `theme`: UI color theme — `latte`, `frappe`, `macchiato`, or `mocha` (default).
  - `jump`: Jump host chain — a **list** of `{ host, user }` entries. SSH receives `-J user1@host1,user2@host2`. Even a single hop must be written as a list item.
  - `wallix`: Wallix/bastion configuration (required when using `bastion` mode).
  - `use_system_ssh_config`: Set to `true` to honour `~/.ssh/config` instead of passing `-F /dev/null`. Defaults to `false`.
  - `probe_filesystems`: List of extra mount points to inspect during the quick diagnostic (`d`). Uses **additive inheritance**: each level appends its paths to the parent list (unlike `user` or `ssh_key` which replace). If a path is not mounted on the target server a yellow `⚠` warning is shown instead of a progress bar.
  - `keep_open`: Set to `true` to reopen the TUI automatically after a connection closes. Defaults to `false` (historical behaviour: susshi exits after connecting).
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
| `Tab` | Cycle connection mode (Direct/Rebond/Wallix) |
| `1` | Select **Direct** mode |
| `2` | Select **Rebond** mode |
| `3` | Select **Wallix** mode |
| `Ctrl+U` | Clear search query (while in search mode) |
| `d` | Run quick SSH diagnostic (kernel, OS, CPU cores, load, RAM, disk) |
| `y` | Copy SSH command to clipboard |
| `r` | Hot-reload configuration files |
| `f` | Toggle favorite on selected server |
| `F` | Toggle favorites-only view |
| `C` | Collapse all expanded groups / environments / namespaces |
| `H` | Toggle recent-sort (flat list sorted by last connection) |
| `x` | Run ad-hoc SSH command on selected server |
| `Esc` | Cancel search / close ad-hoc prompt / dismiss error overlay |

### Mouse Support

- **Click**: Select server or change tab
- **Double-click**: Connect to selected server

## 🖥️ CLI Usage

susshi can connect directly without opening the TUI:

```bash
# Connect directly
susshi --direct root@myserver
susshi --direct admin@10.0.1.5:2222

# Connect via jump host
susshi --jump root@192.168.1.50

# Connect via bastion
susshi --wallix web-01.prod.example.com

# Override SSH parameters
susshi --direct myserver.com --user deploy --port 2222 --key ~/.ssh/deploy_rsa

# Use a custom config file
susshi --config ~/work/.susshi.yml

# Show all options
susshi --help
```

## 🎨 Theme & UI

susshi uses the [Catppuccin](https://github.com/catppuccin/catppuccin) palette. Choose a flavor in your config:

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
- **Lavender**: Namespace headers (📦 included files)
- **Mauve**: Group headers
- **Surface2**: Selection background

### UI Elements

- **Search Bar**: Dynamic title showing result count ("🔍 45 / 387 servers")
- **Connection Modes**: Tab interface with visual highlight
- **Verbose Toggle**: Checkbox indicator (☐/☑) with color feedback
  - **Detail Pane**: Port, mode, identity file, jump/bastion host, SSH options, last connection time, **ad-hoc command output** (when active), and **System** block (kernel, CPU, load, RAM/Disk bars, plus extra mount points) after running `d`
- **Error Overlay**: Centered popup for connection errors — press `Enter` to dismiss

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📄 License

This project is licensed under the MIT License.
