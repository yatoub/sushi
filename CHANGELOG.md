# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

### Changed
- **Multi-jump SSH**: `rebond` in YAML is now a **list** of `JumpConfig` entries (`- host: … / user: …`), enabling SSH proxy-jump chains (`-J user1@h1,user2@h2`). Single-hop configs require wrapping the existing map in a list. The resolved `-J` string is pre-built at config resolution time and stored in `ResolvedServer.jump_host`; `jump_user` field removed.

---

## [0.4.1] — 2026-02-25

### Fixed
- Clipboard warning (`clipboard managers may not have seen the contents`) no longer leaks into the TUI. The `arboard::Clipboard` instance is now kept alive in `App` for the duration of the session instead of being dropped immediately after each copy.

---

## [0.4.0] — 2026-02-25

### Added
- **`ConnectionMode` enum**: replaced `"direct"/"jump"/"bastion"` strings and the `usize` integer with a typed enum throughout the codebase (`config`, `app`, `client`, `handlers`, `ui`). Typos in YAML are now rejected at deserialization.
- **CLI via `clap`**: `--config`, `--direct`, `--rebond`, `--bastion`, `--user`, `--port`, `--key`, `--verbose` flags. The `--direct/--rebond/--bastion` modes connect directly without launching the TUI.
- **`use_system_ssh_config`**: new field in `defaults` (YAML). When `true`, `-F /dev/null` is omitted so `~/.ssh/config` is honored (ControlMaster, aliases, identity files…).
- **Copy SSH command to clipboard** (`y`) via `arboard`. Feedback appears in green in the status bar for 3 seconds.
- **`Ctrl+U`** to clear the search query while in search mode.
- **Expansion state persistence** in `~/.sushi_state.json` (serde_json). Expanded groups/environments are restored on next startup.
- **In-TUI error screen**: `AppMode::Error(String)` renders a centered popup with a rounded red border. Enter/Esc/q dismiss the overlay. SSH errors (missing host, etc.) are caught before connecting.
- **Configurable Catppuccin theme**: `defaults.theme: latte | frappe | macchiato | mocha` in the YAML config. Default: `mocha`.
- **Enriched detail pane**: now shows the effective port (highlighted in yellow when ≠ 22), connection mode, jump host, and bastion host when configured.
- **`examples/full_config.yaml`**: fully documented reference file covering all 3 nesting levels (group → environment → server) and every available key.

### Changed
- `App::new()` now returns `Result<Self, ConfigError>` instead of silently swallowing errors with `unwrap_or_default()`.
- `get_visible_items()` is cached with a `dirty` flag — recomputed only when the config, search query, or expansion state changes.
- `build_ssh_args()` extracted as a pure, testable function from `connect()`.

### Quality
- 15 unit tests for `ssh/client.rs` (3 modes × normal cases + errors + edge cases).
- 22 tests total (6 app/config + 1 integration + 15 SSH client), 0 failures.

---

## [0.3.0] — 2026-02-XX

### Added
- Verbose mode (`-v`) toggled with the `v` key in the TUI.
- Search by host in addition to server name.
- Fixed connection mode inheritance along the defaults → group → env → server chain.

### Fixed
- Rust edition corrected from `2026` to `2024`.

---

## [0.2.0] — v0.1.1

### Added
- ratatui TUI with a group/environment/server tree view.
- SSH connection via `exec()` (process replacement).
- Keyboard navigation (↑/↓, Tab, 1/2/3, Enter, Space, /).
- Mouse click and double-click support.
- 4-level configuration inheritance (defaults → group → environment → server).
- Connection modes: Direct, Jump (ProxyJump), Bastion.
- GitHub Actions CI/CD pipeline.

---

## [0.1.0]

### Added
- First working version: TUI SSH manager with YAML config file support.

[0.4.1]: https://github.com/yatoub/sushi/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/yatoub/sushi/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/yatoub/sushi/compare/v0.1.1...v0.3.0
[0.2.0]: https://github.com/yatoub/sushi/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/yatoub/sushi/releases/tag/v0.1.0
