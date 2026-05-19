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
| `h` | Toggle keyboard help overlay |
| `o` | Open group overview dashboard |
| `\|` | Pin/unpin server in split pane |

State is persisted in `~/.susshi_state.json` (favorites, expanded nodes, sort mode, last seen connection timestamps, tunnel overrides).

## Diagnostics and Commands

| Key | Action |
| --- | --- |
| `d` | Run quick diagnostics on the selected server |
| `x` | Run ad-hoc SSH command on the selected server |

Diagnostics include:

- Kernel, OS info, CPU model/core count, load average.
- RAM and disk usage bars.
- Extra filesystem checks from `probe_filesystems`.

Ad-hoc command output is displayed in the detail pane (up to 20 lines) with colored exit status.

### Command history

When the ad-hoc command prompt (`x`) is open, use `↑` and `↓` to navigate previously run commands. The history is deduplicated (identical consecutive commands are not duplicated) and persists for the duration of the session.

## Group Overview Dashboard

Press `o` when a **group** or **environment** header is selected to open the overview dashboard.

susshi launches a parallel SSH probe on every server in the group and displays results in a live-updating overlay:

| Column | Description |
| --- | --- |
| ✓ / ✗ | Connection success or failure |
| Name | Server name |
| Host | SSH hostname |
| Status | Load average, RAM %, disk % on success; first line of error message on failure |

- Results arrive as threads complete; pending servers show `…`.
- Use `j`/`↓` and `k`/`↑` to scroll the list.
- Press `o` again or `Esc` to close.

The probe uses the current connection mode (Direct/Jump) selected in the tab bar.

## Split Pane

Press `|` on any **server** to pin it to a dedicated right panel.

The layout switches from a 2-column view (tree + details) to a 3-column view (tree + current details + pinned server). The pinned server panel shows name, host, user, port, group/environment, and connection mode.

Press `|` again on the same server (or on any non-server item) to unpin and return to the standard layout.

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

## Keyboard Help Overlay

Press `h` at any time to open an in-TUI reference of all keybindings. Press `h` again or `Esc` to close it.

## Error Overlay

Connection errors are shown as an in-TUI overlay.

Dismiss with `Enter`, `Esc`, or `q`.
