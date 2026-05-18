# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this project is

**Susshi** is a terminal-based SSH connection manager (TUI) written in Rust. It provides a Catppuccin-themed interface for organizing servers with hierarchical groups/environments, multiple connection modes (direct, jump hops, Wallix bastion), tunnels, SCP transfers, and ad-hoc command running.

## Commands

```bash
cargo build --release          # release build (optimized for size)
cargo test --verbose           # run tests
cargo fmt --all -- --check     # check formatting
cargo clippy --all-targets --all-features -- -D warnings  # strict lint
cargo build --features openssl-vendored  # musl targets needing vendored OpenSSL
```

CI also tests these targets: `x86_64-unknown-linux-musl`, `x86_64-pc-windows-msvc`, `x86_64-apple-darwin`, `aarch64-apple-darwin`.

## Architecture

### Entry point and state machine

[main.rs](src/main.rs) handles CLI parsing (clap), terminal initialization via crossterm, and the main event loop. [app.rs](src/app.rs) + [app/](src/app/) define the application state machine split across 11 submodules:

- `core_state.rs` — `AppMode` enum and central state struct
- `lifecycle.rs` — startup/shutdown, config reload
- `search.rs`, `visible_items.rs`, `selection.rs` — filtering and list navigation
- `favorites.rs`, `expansion_state.rs` — persistent UI state
- `command.rs`, `wallix_state.rs`, `tunnel_state.rs`, `scp_state.rs` — mode-specific state

### Configuration

[config.rs](src/config.rs) (~80KB) is the largest file. It parses YAML inventory files, resolves `!include` directives, applies variable interpolation, and resolves the final connection mode (direct / single-hop / multi-hop jump / Wallix) from potentially inherited settings across group → environment → server.

### UI layer

[ui/mod.rs](src/ui/mod.rs) is the main Ratatui draw function. [ui/overlays.rs](src/ui/overlays.rs) and [ui/panels.rs](src/ui/panels.rs) handle modal dialogs and list panels. [ui/theme.rs](src/ui/theme.rs) wires the catppuccin crate into Ratatui styles.

### SSH operations

- [ssh/client.rs](src/ssh/client.rs) — builds the `ssh` command, forkpty on Unix, Wallix auth flow
- [ssh/sftp.rs](src/ssh/sftp.rs) — SCP transfers using ssh2
- [ssh/tunnel.rs](src/ssh/tunnel.rs) — SSH tunnel lifecycle
- [wallix/](src/wallix/) — Wallix bastion auth menu parsing and target selection
- [hooks.rs](src/hooks.rs) — pre-connect / post-disconnect shell hooks

### Supporting modules

- [import.rs](src/import.rs) — parses `~/.ssh/config` into susshi YAML
- [export/](src/export/) — Ansible inventory export
- [probe.rs](src/probe.rs) — diagnostics (disk, SSH keys)
- [i18n.rs](src/i18n.rs) — localization
- [state.rs](src/state.rs) — persistent state (favorites, tunnel overrides) written to disk

## Development environment

- Shell: **fish** (not bash) — use fish syntax in any shell snippets
- Workflow: **TDD** — write the test first, then the implementation

## Key constraints

**Cross-platform:** Unix-specific code must be gated with `#[cfg(unix)]`. The `nix` crate (pty/process/term features) is only in `[target.'cfg(unix)'.dependencies]`. New platform-specific code must compile on Windows/macOS.

**No `unwrap()` in production code.** Use `anyhow` / `thiserror` for error handling.

**Release process is fully automated via release-plz** — never manually edit `Cargo.toml` version, `CHANGELOG.md`, or push version tags. The CI pipeline (release-plz.yml → release.yml → aur-publish.yml) handles versioning, crates.io publish, binary/DEB/RPM packaging, and AUR PKGBUILD updates.

**Never push directly to master.** All changes must go through a PR.

**Conventional commits are required:** `feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`, `perf:` — these drive automatic changelog generation and version bumping.
