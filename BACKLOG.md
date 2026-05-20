# susshi — Backlog d'améliorations

Repo : https://github.com/yatoub/susshi  
Stack : Rust, Ratatui, YAML, SSH2, Catppuccin  

---

## 🔵 Améliorations du code

### Performance

- **`Arc<ResolvedServer>` dans le cache** — `cached_items` clone l'intégralité des `ResolvedServer` à chaque recalcul de la liste visible. Passer à `Arc<ResolvedServer>` éliminerait ces clones et réduirait la pression mémoire sur les gros inventaires. (`src/app/visible_items.rs`, `src/config.rs`)
- **`extend_tags()` / `extend_filesystems()` en O(n²)** — Ces deux fonctions utilisent `Vec::contains()` en boucle ; remplacer par un `HashSet` interne pour la déduplication. (`src/config.rs` ~l. 875 et 891)
- **Clones inutiles dans `resolve_server()`** — Plusieurs `.clone().unwrap_or_default()` sur `Defaults` peuvent être remplacés par `.as_ref().unwrap_or(&defaults)`. (`src/config.rs` ~l. 437, 445, 506)
- **Double parse YAML** — `validate_yaml()` re-désérialise la structure YAML après que serde l'a déjà fait. Fusionner la validation dans un visiteur serde ou la faire en une seule passe. (`src/config.rs` ~l. 988)

### Architecture / lisibilité

- **Découper `resolve_server()`** — 149 lignes, 18 paramètres. Extraire la résolution des defaults, des hooks et de la config Wallix dans des sous-fonctions nommées. (`src/config.rs` ~l. 1206)
- **Découper `load_merged()`** — ~170 lignes gérant includes récursifs, fetch HTTPS et aplatissement de namespaces. Découper en étapes nommées. (`src/config.rs` ~l. 480)
- **Unifier les helpers de merge** — `merge_bastion()`, `merge_jump()`, `extend_filesystems()` suivent le même pattern ; un helper générique éviterait la répétition. (`src/config.rs`)
- **Supprimer `OverviewState.scroll`** — Champ mort marqué `#[allow(dead_code)]`, jamais lu ni écrit. (`src/app/core_state.rs` ~l. 336)
- **Extraire l'état Wallix de `App`** — `App` contient ~40 champs dont plusieurs propres au cycle de vie Wallix (`wallix_pending_connection`, `wallix_pending_auth`, cache de sélection). Extraire dans un `WallixSession` struct dédié. (`src/app/core_state.rs`)

### Robustesse / erreurs

- **`unwrap()` sur `proxy_jump` dans `import.rs`** — Ligne ~245 : `.unwrap()` sur un `Option`; remplacer par `.as_deref()` ou `map()`. (`src/import.rs`)
- **Contexte perdu lors d'un échec de hook** — `hooks.rs` ligne ~40 : l'erreur de `spawn()` ne précise pas si c'est le hook pre ou post qui a échoué. Ajouter le contexte dans le message d'erreur. (`src/hooks.rs`)
- **Warning sans chemin résolu** — `import.rs` lignes ~67-71 : le warning "fichier non trouvé" n'affiche pas le chemin canonique qui a été tenté. (`src/import.rs`)

### Couverture de tests

- **`build_ssh_args()`** — Pas de tests unitaires vérifiant les arguments SSH générés selon le mode (direct, jump, Wallix). (`src/ssh/client.rs`)
- **Parsing et sélection du menu Wallix** — Logique de parsing du menu non couverte unitairement. (`src/wallix/mod.rs`)
- **Mutations d'état dans `app.rs`** — Les transitions de `AppMode` et les mutations de l'état central ne sont pas testées directement. (`src/app/`)

---

## 🟡 Nouvelles fonctionnalités

### UX TUI

- **Recherche floue (fuzzy search)** — Actuellement sous-chaîne exacte + tags. Ajouter un scoring fuzzy (type `fzf` / Levenshtein) pour `appm` → `app-mysql`. Impact UX fort, effort moyen.
- **Persistance de l'historique des commandes ad-hoc** — L'historique de la commande `x` est perdu à chaque fermeture. Le sauvegarder dans le fichier d'état (`~/.cache/susshi/`).
- **Fallback clipboard** — Si `arboard` échoue (environnement sans display, Wayland sans `wl-clipboard`, etc.), afficher la valeur dans une overlay plutôt que de silencieusement ne rien faire.
- **Champ `notes` par serveur** — Permettre une description libre par serveur dans le YAML, affichée dans le panneau de détail et dans l'overlay. Nécessite une évolution du schéma de config.
- **Bookmarks de vues filtrées** — Sauvegarder des filtres nommés (ex. `#prod user:deployer`) pour les rappeler en une touche, sans retaper la recherche.
- **Toggle thème à la volée** — Changer de variante Catppuccin (Latte ↔ Mocha) sans modifier le fichier de config et recharger.
- **Expand / collapse tous les groupes** — Raccourci clavier pour plier ou déplier l'intégralité de l'arbre d'un coup (actuellement `C` replie uniquement le groupe courant).
- **Selection chaine** — Pouvoir séléctionner une chaine de caractère de la TUI à la souris

### SSH / Sécurité

- **Support des certificats SSH** — Ajouter un champ `ssh_cert` dans le schéma de config et passer `-i <cert>` à la commande SSH. Actuellement seul `ssh_key` est supporté.
- **Support de `ProxyCommand` à l'import** — L'import de `~/.ssh/config` ignore silencieusement les entrées `ProxyCommand`. Les convertir en `jump_host` ou les conserver dans `ssh_options`.
- **Override du socket SSH Agent par serveur** — Permettre `ssh_agent_sock: /run/user/1000/gnupg/S.gpg-agent.ssh` dans la config pour router différents serveurs par des agents différents.
- **Rate limiting pour `--exec-group` et probe de groupe** — Ajouter une option `--concurrency` / `max_parallel_connections` pour éviter de saturer le réseau sur les grands groupes.
- **Logs d'audit de connexion** — Enregistrer localement chaque connexion (timestamp, serveur, mode, utilisateur) dans un fichier de log rotatif pour traçabilité et débogage.

### Wallix

- **Auto-détection de locale** — Le parsing du menu Wallix utilise des chaînes codées en dur en anglais et français. Détecter automatiquement la langue ou permettre de configurer les patterns dans la config.
- **Patterns de prompts personnalisables** — Exposer `wallix.menu_patterns` dans la config pour les installations Wallix non standard.
- **Réduction du délai de probe** — Éviter le probe systématique du menu (~5 s) lorsque `wallix.direct: true` est déjà défini ; le flag indique que l'auto-sélection réussira.

### Intégrations et exports

- **Export CSV** — Ajouter un format CSV à `--list` pour intégration tableur ou scripts simples.
- **Export `~/.ssh/config`** — Générer des blocs `Host` compatibles OpenSSH depuis l'inventaire susshi.
- **Import Ansible inventory** — Parser un inventory Ansible YAML existant pour l'importer en config susshi.
- **Import AWS SSM** — Récupérer la liste des instances SSM Session Manager via AWS CLI / SDK et générer la config susshi correspondante.

### SCP / transferts

- **SCP multi-fichiers / récursif** — La session SCP actuelle transfère un seul fichier à la fois. Permettre la sélection multiple et le transfert de répertoires.


### Packaging

- Rédiger une manpage pour les packages linux
- Ajouter la doc et l'exemple de configuration dans `/usr/share/doc/susshi` via les paquets linux
