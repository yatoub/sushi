# TUI Guide

susshi provides an interactive terminal UI for browsing and connecting to servers.

## Navigation

| Key | Action |
| --- | --- |
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Space` | Expand/collapse group |
| `Enter` | Connect / toggle group |
| `q` | Quit |

Mouse:

- Click: select item.
- Double-click: connect.

## Search and Filters

| Key | Action |
| --- | --- |
| `/` | Enter search mode |
| `Ctrl+U` | Clear search query |
| `Esc` | Cancel search |

Search behavior:

- Matches server name and hostname.
- Supports tag filters with `#tag` tokens.
- Multiple tags are AND-combined.
- Mixed queries are allowed (`api #prod`).
- `defaults.default_filter` pre-applies a startup filter.

## Connection Controls

| Key | Action |
| --- | --- |
| `Tab` | Cycle mode (Direct/Jump/Wallix) |
| `1` | Select Direct |
| `2` | Select Jump |
| `3` | Select Wallix |
| `v` | Toggle verbose SSH mode |
| `y` | Copy generated SSH command |

## Productivity Features

| Key | Action |
| --- | --- |
| `r` | Hot-reload all config files |
| `f` | Toggle favorite |
| `F` | Favorites-only view |
| `C` | Collapse all groups/environments/namespaces |
| `H` | Toggle recent-sort view |

State is persisted in `~/.susshi_state.json` (favorites, expanded nodes, sort mode, last seen connection timestamps, tunnel overrides).

## Diagnostics and Commands

| Key | Action |
| --- | --- |
| `d` | Run quick diagnostics |
| `x` | Run ad-hoc SSH command |

Diagnostics include:

- Kernel, OS info, CPU model/core count, load average.
- RAM and disk usage bars.
- Extra filesystem checks from `probe_filesystems`.

Ad-hoc command output is displayed in the detail pane (up to 20 lines) with colored exit status.

## Tunnels and SCP

| Key | Action |
| --- | --- |
| `T` | Open SSH tunnel manager |
| `s` | Open SCP transfer form |

Tunnels:

- Start, stop, add, edit, and delete local forwards.
- Can come from config (`tunnels`) or user overrides.
- Active tunnels show live status badges in the UI.

SCP:

- Upload/download from an in-TUI form.
- Live transfer progress through PTY-backed OpenSSH output.

## Error Overlay

Connection errors are shown as an in-TUI overlay.

Dismiss with `Enter`, `Esc`, or `q`.
