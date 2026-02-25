# Sushi — Instructions pour l'agent de développement

> Toute la documentation (README, CHANGELOG, commentaires publics) est en **anglais**.
> Les instructions de développement et les commentaires privés peuvent être en **français** à usage interne.
> Les commits doivent être en anglais, mais les branches peuvent être nommées en français (ex: `feature/écran-erreur-tui`).
> Les commits doivent respecter la convention [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/).
> On est en TDD — écris les tests avant d'implémenter la fonctionnalité.
> Les PR doivent être rédigées en anglais, mais les titres peuvent être en français (ex: "Écran d'erreur in-TUI").
> Ce fichier d'instructions est en français à usage interne.
> On utilise le tutoiement pour une communication directe et informelle avec l'agent de développement.
> Les instructions sont organisées en sections : Stack technique, Structure du projet, Schéma de configuration YAML, Patterns architecturaux, Modes de connexion SSH, Raccourcis clavier, Tests, Fonctionnalités implémentées et restantes.

---

## 🛠 Stack Technique

| Composant | Crate / outil | Version |
|-----------|--------------|---------|
| Langage | Rust | Edition **2024** |
| TUI | `ratatui` + `crossterm` | 0.30.0 / 0.29.0 |
| SSH | `exec()` système — pas de PTY interne | — |
| Config | `serde` + `serde_yaml` | — |
| Thème | `catppuccin` (4 flavors configurables) | 2.6.0 |
| CLI | `clap` (derive) | 4 |
| Presse-papiers | `arboard` | 3 |
| Persistance | `serde_json` | 1 |
| Expansion `~` | `shellexpand` | 3 |
| Erreurs | `anyhow` + `thiserror` | — |

> `portable-pty = "0.9.0"` est présent dans `Cargo.toml` mais **non utilisé**.
> À retirer ou exploiter dans une prochaine itération (item 5).

---

## 📁 Structure du Projet

```
sushi/
├── src/
│   ├── lib.rs           # Exports publics des modules
│   ├── main.rs          # Point d'entrée : CLI clap, boucle événements, handover SSH
│   ├── app.rs           # État de l'application (App, AppMode, ConfigItem, cache dirty)
│   ├── config.rs        # Parsing YAML, ConnectionMode, ThemeVariant, ResolvedServer
│   ├── state.rs         # Persistance ~/.sushi_state.json (groupes développés)
│   ├── ui/
│   │   ├── mod.rs       # Rendu ratatui complet (draw, draw_details, draw_error_overlay)
│   │   ├── theme.rs     # Statics Catppuccin (LATTE/FRAPPÉ/MACCHIATO/MOCHA), get_theme()
│   │   └── widgets/     # Widgets personnalisés (réservé extension future)
│   ├── ssh/
│   │   ├── mod.rs
│   │   └── client.rs    # build_ssh_args() (pure, testable) + connect() (exec)
│   └── handlers/
│       └── mod.rs       # handle_mouse_event(), get_layout(), is_in_rect()
├── examples/
│   └── full_config.yaml # Référence complète de toutes les clés YAML
├── tests/
│   ├── parse_full_config.rs
│   └── fixtures/        # YAML de fixtures pour les tests d'intégration
├── CHANGELOG.md         # Keep a Changelog, anglais
└── README.md
```

---

## 📝 Schéma de Configuration YAML

Profondeur 1 à 3 : `defaults` → groupe → environnement → serveur.
Référence complète : [`examples/full_config.yaml`](../examples/full_config.yaml)

### Héritage (du moins prioritaire au plus prioritaire)
```
defaults → groupe → environnement → serveur
```
Chaque niveau surcharge : `user`, `port`, `ssh_key`, `ssh_options`,
`default_mode`, `jump_host` (chaîne pré-formatée `user@host:port,…`), `bastion_host`, `bastion_user`, `bastion_template`.

`rebond` est désormais une **liste** de `JumpConfig` (`Option<Vec<JumpConfig>>`). Le niveau enfant remplace entièrement le niveau parent. `resolve_server` construit la chaîne `-J` complète.

### `use_system_ssh_config` (dans `defaults`)
- `false` (défaut) : passe `-F /dev/null` à SSH → ignore `~/.ssh/config`.
- `true` : n'ajoute pas `-F /dev/null` → ControlMaster, aliases et identités `~/.ssh/config` sont honorés.

---

## 🏗 Patterns Architecturaux

### `ConnectionMode` enum
Remplace les chaînes magiques et l'index `usize`. Sérialisé/désérialisé en YAML (`lowercase`).
Expose `index()`, `from_index()`, `next()`. Tab UI sélectionne Direct / Jump / Bastion.

### `AppMode` enum
`Normal` ou `Error(String)`. Quand `Error(msg)` est actif, un overlay rouge centré est affiché.
Enter / Esc / q ferment l'overlay et reviennent à `Normal`.

### Cache `items_dirty`
`get_visible_items()` retourne `&[ConfigItem]` depuis un cache.
Le flag `items_dirty` est levé uniquement quand la config, la recherche ou l'état d'expansion change.

### `ResolvedServer`
Struct plate issue de la résolution de l'héritage YAML. Tous les champs sont fusionnés.
C'est ce type qui est passé à `build_ssh_args()` et affiché dans le panneau détail.

### Handover SSH
`connect()` dans `ssh/client.rs` appelle `Command::new("ssh").args(…).exec()` (trait `CommandExt`).
Le processus sushi est **remplacé** par ssh — il n'y a pas de retour au menu après `exit` ou `Ctrl+D`.

---

## 🖥 Modes de Connexion SSH

| Mode | Commande générée |
|------|-----------------|
| **Direct** | `ssh [-F /dev/null] [-v] [-p PORT] [-i KEY] [-o OPT] user@host` |
| **Jump** | `ssh [-F /dev/null] [-v] -J user1@jump1,user2@jump2 [-i KEY] user@host` |
| **Bastion** | `ssh [-F /dev/null] [-v] -l "<bastion_template>" [-p PORT] bastion_host` |

Le template bastion supporte : `{target_user}`, `{target_host}`, `{bastion_user}`, `%n`.

---

## ⌨️ Raccourcis Clavier (mode Normal)

| Touche | Action |
|--------|--------|
| `↑` / `↓` | Naviguer dans la liste |
| `Enter` | Connexion SSH (mode actif) |
| `Space` | Développer / replier un groupe |
| `/` | Entrer en mode recherche |
| `Ctrl+U` | Vider la recherche (mode recherche) |
| `Esc` | Quitter la recherche / fermer l'overlay erreur |
| `Tab` | Passer au mode de connexion suivant |
| `1` / `2` / `3` | Sélectionner Direct / Jump / Bastion |
| `v` | Activer/désactiver le mode verbeux SSH |
| `y` | Copier la commande SSH dans le presse-papiers |
| `q` / `Ctrl+C` | Quitter |

---

## 🧪 Tests

- **22 tests** au total, 0 échec — `cargo test`
- `tests/parse_full_config.rs` : intégration, parsing YAML complet.
- `src/ssh/client.rs` : 15 tests unitaires `build_ssh_args()` (3 modes × normaux + erreurs + edge cases).
- `tests/fixtures/` : fichiers YAML de référence.

---

## ✅ Fonctionnalités Implémentées (v0.4.0)

- [x] `ConnectionMode` enum typé (remplace les chaînes)
- [x] CLI `clap` : `--config`, `--direct`, `--rebond`, `--bastion`, `--user`, `--port`, `--key`, `--verbose`
- [x] `use_system_ssh_config` dans `defaults`
- [x] Copie commande SSH dans le presse-papiers (`y`) via `arboard`
- [x] `Ctrl+U` pour vider la recherche
- [x] Persistance de l'état d'expansion (`~/.sushi_state.json`)
- [x] Écran d'erreur in-TUI (`AppMode::Error`)
- [x] Thème Catppuccin configurable (`latte` / `frappe` / `macchiato` / `mocha`)
- [x] Panneau détail enrichi : port (jaune si ≠ 22), mode, jump host, bastion host
- [x] `examples/full_config.yaml` documenté
- [x] `build_ssh_args()` extraite comme fonction pure testable
- [x] Cache `items_dirty` pour `get_visible_items()`
- [x] `App::new()` retourne `Result<Self, ConfigError>`

## 🔲 Fonctionnalités Restantes

- [ ] **Item 5** : Retirer `portable-pty` de `Cargo.toml` (dépendance inutilisée) ou l'exploiter pour une gestion PTY interne.
- [ ] **Item 10** : Multi-sauts SSH — `rebond: Option<Vec<JumpConfig>>` dans la config, construire `-J user1@h1,user2@h2` dans `build_ssh_args()`.
