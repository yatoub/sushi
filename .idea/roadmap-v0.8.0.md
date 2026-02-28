# Roadmap v0.8.0

Fonctionnalités principales :
- **Reliquats v0.7.0** : includes imbriqués + fusion des `defaults` inter-fichiers
- **Historique des connexions** — date/heure de dernière connexion par serveur
- **Rechargement à chaud** — recharger la config sans relancer le process (`r`)
- **Favoris** — marquer/épingler des serveurs, vue filtrée dédiée
- **Commande ad-hoc** — exécuter une commande SSH arbitraire depuis la TUI (`x`)
- **Validation YAML stricte** — rapport lisible des champs inconnus/mal typés

---

## 1. Reliquats v0.7.0

### 1.1 Includes imbriqués

La v0.7.0 ignorait silencieusement les `includes` déclarés dans un sous-fichier et émettait un `IncludeWarning::NestedIgnored`.
En v0.8.0, les includes imbriqués sont résolus **récursivement** (la protection anti-circulaire via `loading_stack` est déjà en place).

**Comportement :**
- Un sous-fichier peut lui-même déclarer `includes`.
- Les namespaces générés sont **plats** dans l'arbre TUI (pas de namespaces imbriqués) : les serveurs du sous-sous-fichier apparaissent sous le namespace de niveau 1 qui les a tirés, avec leur label enrichi `"CES / Équipe A"` (séparateur `/`).
- Le `IncludeWarning::NestedIgnored` disparaît ; le variant est retiré.

**Changements :**
- `Config::load_merged()` : supprimer le guard `NestedIgnored`; appel récursif avec `loading_stack` transmis.
- `NamespaceEntry` : ajouter `source_depth: u8` (0 = principal, 1 = direct, 2 = imbriqué…).
- TUI : le label affiché peut être `"CES / Équipe A"` ou simplement `"Équipe A"` avec indentation.

### 1.2 Fusion des `defaults` inter-fichiers

Actuellement les `defaults` d'un sous-fichier s'appliquent **uniquement** à ses propres serveurs.
En v0.8.0, le fichier principal peut déclarer une clé optionnelle :

```yaml
includes:
  - label: "CES"
    path: "~/.sushi_ces.yml"
    merge_defaults: true   # nouveau, défaut : false
```

Quand `merge_defaults: true`, les `defaults` du fichier principal sont **fusionnés** comme couche de base pour les serveurs du sous-fichier (avec la même priorité que les defaults du sous-fichier, mais moins prioritaires que les surcharges groupe/env/serveur).

**Changements :**
- `IncludeEntry` : ajouter `merge_defaults: Option<bool>`.
- `Config::load_merged()` : si actif, passer les `Defaults` du principal à `resolve_entries()` comme base sous les defaults du sous-fichier.

---

## 2. Historique des connexions

Un utilisateur multi-serveurs dispose d'une vue instantanée du "dernier vu" de chaque machine.

### Comportement utilisateur

- Panneau détail : nouvelle ligne `Dernière connexion : il y a 3 jours (25/02/2026 14:22)`.
- Arbre TUI : aucun changement visuel (pas d'icône pour ne pas surcharger).
- Tri optionnel : touche `H` pour basculer le tri de l'arbre entre alphabétique (défaut) et "récemment utilisé".

### Stockage

Extension de `~/.sushi_state.json` :

```json
{
  "expanded_items": ["Group:foo"],
  "last_seen": {
    "Group:Infra:NS::web-01": "2026-02-25T14:22:00Z",
    "NS:CES:Group:PRJ1:Env:Prod:db-01": "2026-02-20T08:11:00Z"
  },
  "sort_by_recent": false
}
```

Clé = `[NS:{ns}:]Group:{g}[:Env:{e}]:Server:{name}` (cohérent avec les clés `expanded_items`).

### Changements

**`src/state.rs`**
```rust
pub struct AppState {
    pub expanded_items:  HashSet<String>,
    pub last_seen:       HashMap<String, DateTime<Utc>>,  // clé server → timestamp ISO8601
    pub sort_by_recent:  bool,
}
```

Dépendance à ajouter : `chrono` (feature `serde`).

**`src/app.rs`**
- `App::connect()` (ou la branche `AppResult::Connect` dans `main.rs`) : enregistre `Utc::now()` dans `app_state.last_seen` sous la clé du serveur connecté.
- `get_visible_items()` : si `sort_by_recent`, trier les serveurs par `last_seen` décroissant.
- Nouvelle méthode `App::last_seen_for(server: &ResolvedServer) -> Option<DateTime<Utc>>`.

**`src/ui/mod.rs`** — `draw_details()`
```
Dernière connexion : il y a 3 jours  (25/02/2026 14:22)
```
Si jamais connecté : `Dernière connexion : —`

**`src/i18n.rs`**
```rust
pub last_seen_never:   &'static str,  // "Aucune"  / "Never"
pub last_seen_ago:     &'static str,  // "il y a {}"  / "{} ago"
pub last_seen_label:   &'static str,  // "Dernière connexion"  / "Last connection"
pub sort_recent_hint:  &'static str,  // "[H] Tri récent"  / "[H] Recent sort"
```

---

## 3. Rechargement à chaud (`r`)

Recharger tous les fichiers YAML (principal + includes) sans quitter la TUI.

### Comportement utilisateur

- Touche `r` hors mode recherche : déclenche le rechargement.
- L'arbre se met à jour ; l'état d'expansion est **préservé** pour les nœuds encore présents.
- le serveur sélectionné est retrouvé par son identifiant (`namespace:group:env:name`) si encore présent, sinon sélection remise à 0.
- Si une erreur fatale survient (YAML invalide) : overlay d'erreur non-bloquant, ancienne config conservée en mémoire.
- Barre de statut : `Config rechargée (N serveurs)` pendant 2 s.

### Changements

**`src/app.rs`**
```rust
pub fn reload(&mut self, config_path: &Path) -> Result<(), ConfigError> {
    let (new_config, warnings) = Config::load_merged(config_path, &mut HashSet::new())?;
    let sel_key = self.selected_server_key(); // namespace:group:env:name
    self.config = new_config;
    self.warnings = warnings;
    self.invalidate_cache();
    self.restore_selection(sel_key);
    self.set_status_message(i18n::fmt(lang.config_reloaded, &[&server_count.to_string()]));
    Ok(())
}
```

**`src/app.rs` — `App`**
- Ajouter `config_path: PathBuf` dans la struct (transmis depuis `main.rs`).

**`src/handlers/mod.rs`**
```rust
KeyCode::Char('r') if !app.is_searching => {
    if let Err(e) = app.reload(&app.config_path.clone()) {
        app.set_error(e.to_string());
    }
}
```

**`src/i18n.rs`**
```rust
pub config_reloaded:  &'static str,  // "Config rechargée ({} serveurs)"  / "Config reloaded ({} servers)"
pub reload_hint:      &'static str,  // "[r] Recharger"  / "[r] Reload"
```

---

## 4. Favoris (`f` / `F`)

Épingler des serveurs fréquemment utilisés pour y accéder sans navigation.

### Comportement utilisateur

- `f` : bascule le favori du serveur sélectionné (toggle). Icône `⭐` dans l'arbre à droite du nom.
- `F` : bascule la vue "Favoris uniquement" — l'arbre n'affiche que les serveurs marqués (avec leur hiérarchie complète : namespace / groupe / env conservés).
- En mode favoris, un bandeau `⭐ Favoris` remplace le titre normal de la zone liste.
- Les favoris sont persistés dans `~/.sushi_state.json`.

### Stockage

```json
{
  "expanded_items": [...],
  "last_seen": {...},
  "favorites": ["NS:CES:Group:PRJ1:Env:Prod:web-01", "Group:Infra:proxmox"],
  "sort_by_recent": false
}
```

Même format de clé que `last_seen`.

### Changements

**`src/state.rs`**
```rust
pub struct AppState {
    pub expanded_items: HashSet<String>,
    pub last_seen:      HashMap<String, chrono::DateTime<chrono::Utc>>,
    pub favorites:      HashSet<String>,
    pub sort_by_recent: bool,
}
```

**`src/app.rs`**
- `App` : ajouter `favorites_only: bool`.
- `App::toggle_favorite()` : insert/remove dans `app_state.favorites` + `save_state()`.
- `App::toggle_favorites_view()` : bascule `favorites_only`.
- `get_visible_items()` : si `favorites_only`, filtrer pour ne garder que les `ConfigItem::Server` dont la clé est dans `favorites`.
- `is_favorite(server: &ResolvedServer) -> bool` : helper.

**`src/ui/mod.rs`**
- `draw_tree()` : afficher `⭐` à droite du nom pour les favoris (même hors vue favorites).
- `draw_tree()` : titre de la liste = `⭐ Favoris` si `favorites_only`.

**`src/handlers/mod.rs`**
```rust
KeyCode::Char('f') => app.toggle_favorite(),
KeyCode::Char('F') => app.toggle_favorites_view(),
```

**`src/i18n.rs`**
```rust
pub favorites_title:   &'static str,  // "⭐ Favoris"  / "⭐ Favorites"
pub favorite_added:    &'static str,  // "⭐ Ajouté aux favoris"  / "⭐ Added to favorites"
pub favorite_removed:  &'static str,  // "Favori retiré"  / "Removed from favorites"
pub fav_hint:          &'static str,  // "[f] Favori  [F] Vue favoris"  / "[f] Favorite  [F] Fav. view"
```

---

## 5. Commande ad-hoc (`x`)

Exécuter une commande shell arbitraire sur le serveur sélectionné, résultat affiché dans la zone détail.

### Comportement utilisateur

- `x` : ouvre un mini-prompt dans la barre de statut (`Commande : █`).
- `Entrée` : lance la commande via SSH (même mécanisme que le probe, avec `build_ssh_args()`).
- La zone détail bascule en mode "sortie commande" : titre + sortie brute (scrollable).
- `Esc` : ferme le prompt / efface la sortie et revient au panneau standard.
- Si la commande échoue (exit code ≠ 0), la sortie stderr est affichée en rouge.
- Exécution **asynchrone** (thread séparé → `mpsc`) pour ne pas bloquer la TUI.

### Types

```rust
pub enum CmdState {
    Idle,
    Prompting(String),           // saisie en cours
    Running(String),             // cmd lancée, en attente
    Done { cmd: String, output: String, exit_ok: bool },
    Error(String),
}
```

### Changements

**`src/app.rs`**
- `App` : ajouter `cmd_state: CmdState` et `cmd_rx: Option<Receiver<(String, bool)>>`.
- `App::start_cmd(cmd: String)` : lance un thread SSH non-interactif (pas de PTY, `ssh -o RequestTTY=no … cmd`), envoie résultat via channel.
- `App::poll_cmd()` : appelé dans la boucle principale comme `poll_probe()`.

**`src/probe.rs`** — factoriser `build_ssh_args()` en helper partagé (ou l'exposer depuis `src/ssh/client.rs`).

**`src/ui/mod.rs`**
- `draw_details()` : si `cmd_state != Idle`, affiche la sortie à la place du panneau habituel.
- `draw_status_bar()` : si `CmdState::Prompting`, affiche `Commande : <input>█`.

**`src/handlers/mod.rs`**
```rust
KeyCode::Char('x') if selected_server => {
    app.cmd_state = CmdState::Prompting(String::new());
}
// … dans le bloc Prompting :
KeyCode::Enter  => app.start_cmd(std::mem::take(&mut prompt)),
KeyCode::Esc    => app.cmd_state = CmdState::Idle,
KeyCode::Char(c) => prompt.push(c),
KeyCode::Backspace => { prompt.pop(); }
```

**`src/i18n.rs`**
```rust
pub cmd_prompt:    &'static str,  // "Commande : "  / "Command: "
pub cmd_running:   &'static str,  // "Exécution…"  / "Running…"
pub cmd_hint:      &'static str,  // "[x] Commande ad-hoc"  / "[x] Ad-hoc command"
pub cmd_exit_ok:   &'static str,  // "Terminé (exit 0)"  / "Done (exit 0)"
pub cmd_exit_err:  &'static str,  // "Erreur (exit {})"  / "Error (exit {})"
```

---

## 6. Validation YAML stricte

Détecter et rapporter les champs inconnus ou mal typés sans bloquer le démarrage.

### Comportement utilisateur

- Au chargement (démarrage ou rechargement), si des problèmes sont détectés, un overlay s'affiche :
  ```
  ⚠  Problèmes de configuration
  ──────────────────────────────
  ~/.sushi.yml ligne 14 : champ inconnu « usre » (serveur web-01)
  ~/.sushi_ces.yml ligne 7 : « port » doit être un entier (valeur : "vingt-deux")
  ──────────────────────────────
  [Entrée / Esc] Continuer quand même
  ```
- Les serveurs concernés sont chargés avec les valeurs par défaut pour les champs invalides.
- L'overlay utilise le même composant que les warnings d'includes existants.

### Implémentation

Serde avec `#[serde(deny_unknown_fields)]` est **trop brutal** (blocage total). On utilise à la place :

1. **Double désérialisation** : d'abord via `serde_json::Value` (ou `serde_yaml::Value`) pour capturer le document brut, puis désérialisation typée normalement.
2. Comparaison des clés présentes avec la liste des champs connus → produit des `ValidationWarning`.
3. Pour les erreurs de type : capturer les erreurs de `serde_yaml` et les reformater en indiquant le chemin YAML (fichier + contexte).

```rust
pub enum ValidationWarning {
    UnknownField { file: String, path: String, field: String },
    TypeMismatch { file: String, path: String, field: String, got: String },
}
```

**`src/config.rs`**
- `Config::load()` : ajouter une passe de validation via `Value` avant la désérialisation typée.
- Retourne `(Config, Vec<ValidationWarning>)` au lieu de `Config`.
- `Config::load_merged()` agrège les `ValidationWarning` de tous les fichiers chargés.

**`src/app.rs`** — `App::warnings` : `Vec<AnyWarning>` où `AnyWarning = IncludeWarning | ValidationWarning`.

**`src/ui/mod.rs`** — overlay de warnings étendu pour afficher les `ValidationWarning`.

---

## Ordre d'implémentation recommandé

| # | Tâche | Dépend de |
|---|-------|-----------|
| 1 | Reliquat : includes imbriqués | — |
| 2 | Reliquat : `merge_defaults` | — |
| 3 | `chrono` dans `Cargo.toml` | — |
| 4 | `AppState` : `last_seen`, `favorites`, `sort_by_recent` | 3 |
| 5 | `App` : `config_path`, `favorites_only`, `cmd_state` | — |
| 6 | `App::reload()` | 5 |
| 7 | `App::toggle_favorite()` + `favorites_only` dans `get_visible_items()` | 4, 5 |
| 8 | Historique : enregistrement + affichage dans détail | 4 |
| 9 | Tri par récence (`H`) | 8 |
| 10 | Commande ad-hoc `x` | 5 |
| 11 | Validation YAML | — |
| 12 | `i18n.rs` : toutes les nouvelles clés | au fur et à mesure |
| 13 | Tests | après chaque groupe |
| 14 | Documentation (README, full_config.yaml) | fin |

---

## Tests

| Fichier | Test | Description |
|---------|------|-------------|
| `config.rs` | `test_nested_includes` | Sous-fichier avec includes → résolution récursive, protection circulaire |
| `config.rs` | `test_merge_defaults_flag` | `merge_defaults: true` appliquer les defaults du principal dans le sous-fichier |
| `config.rs` | `test_validation_unknown_field` | Champ inconnu → `ValidationWarning::UnknownField` |
| `config.rs` | `test_validation_type_mismatch` | Mauvais type → `ValidationWarning::TypeMismatch` |
| `app.rs` | `test_reload_preserves_expansion` | `App::reload()` conserve l'état d'expansion |
| `app.rs` | `test_reload_error_keeps_config` | Config invalide en reload → ancienne config inchangée |
| `app.rs` | `test_favorite_toggle` | `toggle_favorite()` insère/supprime dans `AppState.favorites` |
| `app.rs` | `test_favorites_view_filter` | Mode favoris → seuls les serveurs favoris visibles |
| `app.rs` | `test_last_seen_recorded` | Connexion enregistre un timestamp dans `last_seen` |
| `app.rs` | `test_sort_by_recent` | `sort_by_recent = true` → tri des serveurs par `last_seen` décroissant |

---

## Non inclus dans v0.8.0

- **Auto-discovery `~/.sushi_*.yml`** — déclaration explicite via `includes` reste la norme.
- **Multi `--config` en CLI** — hors scope.
- **Tunnel de port** (`-L`) — prévu v0.9.
- **Export `~/.ssh/config`** — prévu v0.9.
- **Tags YAML + filtre** — prévu v0.9.
