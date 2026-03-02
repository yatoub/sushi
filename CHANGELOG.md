# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

---

## [0.9.1] — 2026-03-02

### Changed

- **Rename `ConnectionMode::Bastion` → `ConnectionMode::Wallix`**: the connection mode is now called `wallix` in config files (YAML value). Backward compatibility is preserved via a serde alias — existing configs using `mode: "bastion"` continue to work.

### Removed

- **Example scripts** (`examples/check_mode_inheritance.rs`, `examples/test_search.rs`): removed stale development helpers that required a personal `~/.susshi.yml` to run and were not reproducible as automated tests.

---

## [0.9.0] — 2026-03-02

### Added

- **Keep-open mode** (`keep_open`): set `defaults.keep_open: true` in your config to automatically reopen the TUI after a connection closes, allowing you to quickly switch to another server without relaunching susshi. Defaults to `false` (historical behaviour: the process exits after connecting via `exec`). Closes #9.

---

## [0.8.6] — 2026-03-02

### Fixed

- **CI**: remove invalid `needs: []` in `aur-publish-bin` job that caused a workflow lint error.

---

## [0.8.5] — 2026-03-02

### Fixed

- **OS detection in diag**: use `PRETTY_NAME` from `/etc/os-release` instead of assembling `NAME` + `VERSION_ID` with awk. The previous awk expression (`print n(v?...)`) was treated as a function call on some implementations (RHEL/gawk), causing the field to always return `unknown`.

---

## [0.8.4] — 2026-03-02

### Added

- **Collapse all** (`C`): press `C` to collapse all expanded groups, namespaces and environments at once and jump back to the top of the list (closes #7).
- **CPU core count in diag** (`d`): the diagnostic panel now shows the number of logical CPU cores (`nproc`) below the CPU model (closes #8).
- **OS name in diag** (`d`): the diagnostic panel now shows the OS name and version from `/etc/os-release` (e.g. `Debian GNU/Linux 12`) below the kernel version (closes #10).

---

## [0.8.3] — 2026-03-02

### Fixed

- **AUR package**: fix b2sum mismatch caused by re-tagging v0.8.2 after updating `Cargo.lock`.

---

## [0.8.2] — 2026-03-02

### Fixed

- **AUR package**: fix broken install caused by `source` URL still pointing to the old `sushi` repository name instead of `susshi`. The extracted directory was named `sushi-x.y.z` while the PKGBUILD expected `susshi-x.y.z`.
- **AUR automation**: add GitHub Actions workflow to automatically publish updated PKGBUILD to AUR on every tagged release.

---

## [0.8.1] — 2026-03-02

### Fixed

- **Include inheritance**: servers in included files now automatically inherit the main file's `defaults` (user, ssh port, jump host, wallix, probe filesystems, etc.). Previously, inheritance only worked when `merge_defaults: true` was explicitly set on the include entry. The sub-file's own `defaults` still take precedence field-by-field.
- **Test isolation**: `test_namespace_visibility_collapsed` and `test_namespace_expansion` now reset `expanded_items` after `App::new` to avoid interference from a real `~/.susshi_state.json` on the developer machine.
- **i18n test race condition**: `with_env()` now correctly uses a `Mutex` (as its comment always claimed) to prevent concurrent environment variable mutation across parallel tests.

### Added

- **Historique des connexions** (`last_seen`): l'horodatage de la dernière connexion à chaque serveur est persisté dans `~/.susshi_state.json`. Il s'affiche dans le panneau de détails (ex. : "il y a 2 h").
- **Rechargement à chaud** (touche `r`): recharge la configuration depuis le disque sans quitter l'application. Un message temporaire confirme le succès ou l'erreur.
- **Favoris** (touche `f`) : bascule le statut favori du serveur sélectionné. Les favoris apparaissent avec une icône ⭐ dans l'arbre.
- **Vue favoris** (touche `F`) : basculer entre l'affichage de tous les serveurs et les favoris seuls.
- **Tri par récent** (touche `H`) : mode liste plate triée par dernière connexion (le plus récent en premier).
- **Commande ad-hoc** (touche `x`) : saisir une commande SSH non-interactive à exécuter sur le serveur sélectionné ; le résultat s'affiche dans le panneau de détails (20 premières lignes, code de sortie coloré).
- **Validation YAML stricte** : les champs inconnus dans les fichiers de config déclenchent des avertissements `ValidationWarning` non-bloquants (collectés à chaque `load_merged`).
- **Includes récursifs** (`merge_defaults`) : les sous-fichiers inclus peuvent eux-mêmes avoir des `includes`; le flag `merge_defaults: true` sur une entrée d'include fusionne les `defaults` du parent dans le sous-fichier.

### Changed

- `Config::load_merged` retourne maintenant un triplet `(Config, Vec<IncludeWarning>, Vec<ValidationWarning>)`.
- `App::new` accepte deux arguments supplémentaires : `config_path: PathBuf` et `validation_warnings: Vec<ValidationWarning>`.
- `IncludeWarning::NestedIgnored` supprimé — les includes imbriqués sont désormais traités récursivement.

---

## [0.7.1] — 2026-02-28

### Fixed

- **Panic au démarrage** : les chaînes littérales `conflicts_with_all` de clap référençaient les anciens noms de champs `"rebond"` et `"bastion"` après leur renommage en `"jump"` et `"wallix"`.

---

## [0.7.0] — 2026-02-28

### Added

- **`includes` / namespaces**: new top-level `includes` key in the main config file. Each entry has a `label` (displayed as a collapsible `📦` namespace in the TUI) and a `path` (absolute or `~`-expanded; relative paths are resolved from the main file's directory). Included files use the same YAML schema and their `defaults` apply only to their own servers.
- **Circular-dependency & nested-include detection**: startup warnings are emitted as non-blocking overlays when a cycle or unsupported nested include is detected.
- **`jump` key** (config): replaces the former `rebond` key for expressing SSH jump-host chains. A **list** of `{ host, user }` entries, even for a single hop.
- **`wallix` key** (config): replaces the former `bastion` key for Wallix/PAM bastion configuration.

### Changed

- Config field `rebond` renamed to `jump` at all hierarchy levels (`defaults`, group, environment, server). Old configs using `rebond:` must be migrated.
- Config field `bastion` renamed to `wallix` at all hierarchy levels. Old configs using `bastion:` must be migrated.
- Namespace entries rendered as top-level collapsible nodes (`📦`) in the server tree; their groups/environments/servers are indented beneath them as usual.

### Quality

- 55 tests (unit + integration) — all passing.

---

## [0.6.0] — 2026-02-28

### Added

- **Internationalisation (i18n)**: all TUI strings (labels, titles, status bar, error messages, hints) are extracted into `src/i18n.rs`. Language is auto-detected at startup from `LC_ALL` → `LC_MESSAGES` → `LANG`. French (`fr*`) and English (default) are supported with no external dependencies.
- **`probe_filesystems`**: new optional list key at every config level (`defaults`, group, environment, server). Extra mount points are probed during the quick diagnostic (`d`) and rendered as color-coded progress bars in the detail pane. If a path is not mounted on the target, a yellow `⚠ /path — not mounted` line is shown instead. Inheritance is **additive**: each level appends its paths to those of the parent (no deduplication across levels).
- **i18n `fmt()` helper**: zero-dependency template substitution (`{}` placeholders) for dynamic status messages.

### Changed

- Bastion-mode tab label renamed to **Wallix** (reflects WAB/PAM bastion type).

### Fixed

- **Double-click connection mode override**: `App::select()` no longer resets `connection_mode` when the click targets the already-selected server, preserving manual Tab/1-3 overrides through to the connection.

### Quality

- 45 unit tests (8 new i18n tests: locale detection × 4, fmt × 3, FR≠EN smoke), 0 failures.
- `cargo fmt` + `cargo clippy -D warnings` clean.

---

## [0.5.0] — 2026-02-25

### Added

- **Quick diagnostic (`d`)**: pressing `d` on a selected server launches a non-blocking SSH probe in a dedicated thread. The detail pane then displays a **System** block: kernel, CPU model, load average, and color-coded RAM/Disk progress bars (green < 60%, yellow 60–85%, red > 85%). An animated spinner is shown while waiting. Pressing `d` again re-runs the probe; switching server resets it.

### Changed

- **Multi-hop SSH** ⚠️ **Breaking**: `rebond` is now a **list** of `JumpConfig` entries (`- host: … / user: …`), enabling ProxyJump chains (`-J user1@h1,user2@h2`). Existing configs using the map syntax must be converted. The `jump_user` field has been removed from `ResolvedServer`.

### Fixed

- SSH argument ordering: `-i` and `ssh_options` are now placed before the destination, ensuring `user@host` is always the last argument (critical fix for the quick diagnostic).

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
- **Expansion state persistence** in `~/.susshi_state.json` (serde_json). Expanded groups/environments are restored on next startup.
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

[0.6.0]: https://github.com/yatoub/susshi/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/yatoub/susshi/compare/v0.4.1...v0.5.0
[0.4.1]: https://github.com/yatoub/susshi/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/yatoub/susshi/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/yatoub/susshi/compare/v0.1.1...v0.3.0
[0.2.0]: https://github.com/yatoub/susshi/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/yatoub/susshi/releases/tag/v0.1.0
