# Roadmap post-v0.3.0

Liste des améliorations à implémenter une par une.

---

## Robustesse & qualité du code

- [x] **1. `ConnectionMode` en enum**
  - Fichiers : `src/app.rs`, `src/main.rs`, `src/ssh/client.rs`
  - Remplacer `connection_mode: usize` (valeurs magiques 0/1/2) par un enum `ConnectionMode { Direct, Jump, Bastion }`.
  - Élimine les risques de valeur hors-borne et rend le `match` exhaustif.

- [x] **2. `mode: Option<String>` → enum serde dans la config**
  - Fichier : `src/config.rs`
  - Créer `enum ConnectionModeCfg { Direct, Jump, Bastion }` avec `#[serde(rename_all = "lowercase")]`.
  - Une faute de frappe dans le YAML sera rejetée à la désérialisation au lieu de tomber silencieusement en mode `"direct"`.

- [ ] **3. Mise en cache de `get_visible_items()`**
  - Fichier : `src/app.rs`
  - Ajouter un champ `cached_items: Vec<ConfigItem>` et un flag `dirty: bool`.
  - Recalculer uniquement quand la config, la recherche ou l'état d'expansion change.

- [ ] **4. Remonter l'erreur de `config.resolve()` dans `App::new()`**
  - Fichier : `src/app.rs`
  - Changer la signature en `App::new(config: Config) -> Result<Self, ConfigError>` pour ne plus avaler les erreurs silencieusement avec `unwrap_or_default()`.

- [ ] **5. Supprimer ou exploiter la dépendance `portable-pty`**
  - Fichier : `Cargo.toml`
  - Si non utilisée : la retirer.
  - Si prévue : implémenter l'ouverture du terminal intégré dans la TUI (session SSH inline).

---

## Fonctionnalités

- [x] **6. Argument CLI `--config <path>`**
  - Fichier : `src/main.rs`
  - Ajouter `clap` en dépendance et exposer `--config`, `--version`, `--help`.
  - Permet de gérer plusieurs profils / contextes (perso, pro, client…).
  - exposer `--direct`, `--rebond` et `--bastion` suivi d'un host pour initier une connexion directe sans passer par l'ui  

- [ ] **7. Rendre `-F /dev/null` optionnel**
  - Fichier : `src/ssh/client.rs`
  - Ajouter un champ `use_system_ssh_config: bool` dans `Defaults` (défaut `false`).
  - Quand `true`, ne pas passer `-F /dev/null` afin de respecter `~/.ssh/config` (ControlMaster, aliases, clés…).

- [ ] **8. Raccourci `Ctrl+U` pour vider la recherche**
  - Fichier : `src/main.rs`
  - En mode recherche, capturer `KeyCode::Char('u')` + `KeyModifiers::CONTROL` pour effacer `search_query` en entier.

- [ ] **9. Copier la commande SSH dans le presse-papiers**
  - Fichiers : `src/main.rs`, `src/ssh/client.rs`
  - Raccourci `y` sur un serveur sélectionné.
  - Extraire la construction de la commande SSH en une fonction `build_command()` (sans `exec`), puis écrire dans le presse-papiers (crate `arboard` ou appel `xclip`/`pbcopy`).

- [ ] **10. Multi-sauts SSH (ProxyJump chaîné)**
  - Fichiers : `src/config.rs`, `src/ssh/client.rs`
  - Changer `rebond: Option<JumpConfig>` en `rebond: Option<Vec<JumpConfig>>`.
  - Construire la chaîne `-J user1@host1,user2@host2` dans `client.rs`.

- [ ] **11. Persistance de l'état d'expansion**
  - Fichier : `src/app.rs`, nouveau module `src/state.rs`
  - Sauvegarder `expanded_items: HashSet<String>` dans `~/.sushi_state.json` (serde_json) à la fermeture, restaurer au démarrage.

---

## UX / Interface

- [ ] **12. Écran d'erreur in-TUI**
  - Fichier : `src/main.rs`, `src/ui/mod.rs`
  - Ajouter un variant `AppState::Error(String)` et afficher un panneau d'erreur centré dans la TUI au lieu de quitter brutalement avec `eprintln!`.

- [ ] **13. Afficher le port effectif dans le panneau détail**
  - Fichier : `src/ui/mod.rs` (`draw_details`)
  - Vérifier que le port résolu (issu de `ResolvedServer.port`) est bien rendu visible, en particulier quand il diffère de 22.

- [ ] **14. Thème configurable (variantes Catppuccin)**
  - Fichiers : `src/config.rs`, `src/ui/theme.rs`
  - Ajouter `theme: Option<String>` dans `Defaults` (`"latte"`, `"frappe"`, `"macchiato"`, `"mocha"`).
  - Charger la palette correspondante depuis la crate `catppuccin` déjà présente.

---

## Tests & maintenabilité

- [ ] **15. Tests unitaires pour `ssh/client.rs`**
  - Fichier : `src/ssh/client.rs`
  - Extraire la construction de la commande dans `build_ssh_args(server, mode, verbose) -> Vec<String>`.
  - Écrire des tests couvrant les 3 modes et les cas d'erreur (jump_host vide, bastion_host vide).

- [ ] **16. Ajouter un `CHANGELOG.md`**
  - Fichier : `CHANGELOG.md` (racine du projet)
  - Format [Keep a Changelog](https://keepachangelog.com/fr/1.0.0/) avec les entrées `[0.3.0]`, `[0.2.0]`, etc.
  - Documenter les changements futurs dans `[Unreleased]` au fil des PR.

---

## Ordre d'implémentation suggéré

1. → **2** (enum config, sécurité de base)
2. → **1** (enum ConnectionMode, refactor propre)
3. → **6** (CLI args, clap)
4. → **7** (respect de ~/.ssh/config)
5. → **15** (tests client SSH)
6. → **4** (propagation d'erreurs)
7. → **9** (copie presse-papiers)
8. → **8** (Ctrl+U)
9. → **3** (cache visible items)
10. → **11** (persistance expansion)
11. → **12** (écran erreur TUI)
12. → **14** (thèmes)
13. → **10** (multi-jump)
14. → **5** (portable-pty / terminal intégré)
15. → **13** (port dans détails)
16. → **16** (CHANGELOG)
