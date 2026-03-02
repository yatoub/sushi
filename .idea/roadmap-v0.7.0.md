# Roadmap v0.7.0

Fonctionnalité principale : **configuration multi-fichiers avec `includes`** (issue #3) — possibilité de répartir la configuration par équipe/périmètre dans des fichiers YAML distincts, fusionnés au démarrage et présentés dans la TUI sous un niveau hiérarchique dédié (namespace).

---

## Objectif

Un utilisateur multi-équipes peut maintenant maintenir un fichier YAML par équipe (`~/.susshi_ces.yml`, `~/.susshi_crt.yml`, etc.) et les référencer depuis un fichier principal via la clé `includes`. La TUI affiche les serveurs regroupés par namespace (= label de l'include), sans modifier l'arborescence interne de chaque fichier.

```
📦 CES
  📂 PRJ1
    🌩️ Production
       🖥️ web-01
       🖥️ db-01
📦 CRT
  📂 PRJ2
  📂 PRJ3
     🖥️ standalone
```

Les serveurs du fichier principal (sans namespace) continuent d'apparaître directement à la racine, comme avant.

---

## Syntaxe YAML

```yaml
# ~/.susshi.yml — fichier principal
includes:
  - label: "CES"
    path: "~/.susshi_ces.yml"
  - label: "CRT"
    path: "~/.susshi_crt.yml"

defaults:
  user: "admin"
  ssh_key: "~/.ssh/id_ed25519"

groups:
  - name: "Local"          # groupes du fichier principal — sans namespace
    servers:
      - name: "dev-vm"
        host: "192.168.56.10"
```

- `label` : texte affiché comme en-tête de namespace dans la TUI.
- `path` : chemin absolu ou `~`-expandé. Les chemins relatifs sont résolus par rapport au répertoire du fichier principal.
- Les fichiers inclus sont des YAML susshi standards (même format, même héritage `defaults → group → env → server`).
- Les `defaults` d'un fichier inclus sont **locaux** : ils ne fusionnent pas avec ceux du fichier principal.
- **Les includes dans un fichier inclus sont ignorés** (pas de nesting, v0.7 seulement) — un avertissement non-fatal est affiché.

---

## Architecture technique

### `src/config.rs`

#### Nouveaux types

```rust
/// Entrée dans la section `includes` du fichier principal.
#[derive(Debug, Deserialize, Clone)]
pub struct IncludeEntry {
    pub label: String,
    pub path:  String,
}

/// Nœud de namespace dans l'arbre de configuration — résultat du chargement
/// d'un fichier inclus, dont les groupes deviennent des enfants.
#[derive(Debug, Clone)]
pub struct NamespaceEntry {
    pub label:   String,
    pub entries: Vec<ConfigEntry>,  // ConfigEntry::Group ou ConfigEntry::Server
}
```

#### Modifications de `Config`

```rust
pub struct Config {
    pub defaults: Option<Defaults>,
    pub groups:   Vec<ConfigEntry>,
    pub includes: Option<Vec<IncludeEntry>>,   // nouveau — ignoré dans les sous-fichiers
}
```

#### Nouveau variant `ConfigEntry`

```rust
pub enum ConfigEntry {
    Server(Server),
    Group(Group),
    Namespace(NamespaceEntry),   // nouveau variant
}
```

#### `Config::load_merged()`

Nouvelle méthode publique qui remplace l'appel à `Config::load()` dans `main.rs` :

```rust
pub fn load_merged<P: AsRef<Path>>(
    path: P,
    loading_stack: &mut HashSet<PathBuf>,  // protection anti-circulaire
) -> Result<(Self, Vec<IncludeWarning>), ConfigError>
```

1. Charge le fichier principal avec `Config::load()`.
2. Pour chaque `IncludeEntry` :
   a. Résout le chemin (`shellexpand::tilde` + `canonicalize`).
   b. Détecte les cycles via `loading_stack` → `IncludeWarning::Circular`.
   c. Tente `Config::load(sub_path)` ; en cas d'échec → `IncludeWarning::LoadError`.
   d. Si des sous-includes sont présents dans le fichier inclus → `IncludeWarning::NestedIgnored`.
   e. Enveloppe les entrées dans un `ConfigEntry::Namespace { label, entries: sub.groups }`.
   f. Ajoute le namespace à la fin de `main.groups`.
3. Appelle `sort()` sur l'ensemble.
4. Retourne `(Config, Vec<IncludeWarning>)`.

```rust
pub enum IncludeWarning {
    LoadError   { label: String, path: String, error: String },
    Circular    { label: String, path: String },
    NestedIgnored { label: String },
}
```

#### `Config::resolve()`

La signature change pour propager le `namespace` :

```rust
// Interne — passe le namespace courant aux résolveurs de groupe
fn resolve_with_namespace(namespace: &str, ...) -> Result<Vec<ResolvedServer>, ConfigError>
```

#### `ResolvedServer`

Ajout du champ :

```rust
pub struct ResolvedServer {
    // …champs existants…
    pub namespace: String,   // vide si fichier principal, sinon label de l'include
}
```

---

### `src/app.rs`

#### `ConfigItem`

```rust
pub enum ConfigItem {
    Namespace(String),                // nouveau
    Group(String),
    Environment(String, String),
    Server(Box<ResolvedServer>),
}
```

#### `build_visible_items()`

Itération sur le nouvel arbre :

```
pour chaque ConfigEntry de config.groups :
  ├─ ConfigEntry::Namespace(ns) →
  │    push ConfigItem::Namespace(ns.label)
  │    si NS expandé ou recherche active :
  │      pour chaque entry in ns.entries :
  │        (même logique groupe/env/serveur qu'avant)
  ├─ ConfigEntry::Group(g) → logique actuelle
  └─ ConfigEntry::Server(s) → logique actuelle
```

#### `toggle_expansion()`

Gère la nouvelle clé `"NS:{label}"` pour les items namespace (même pattern que `"Group:{name}"`).

#### `matches_search()`

Inchangée — les serveurs d'un namespace sont filtrés avec la même logique nom/hôte.

---

### `src/ui/mod.rs`

#### Rendu `ConfigItem::Namespace`

```rust
ConfigItem::Namespace(label) => {
    let id = format!("NS:{}", label);
    let icon = if expanded { "📦" } else { "📫" };
    Line::from(vec![Span::styled(
        format!("{} {}", icon, label),
        Style::default().fg(theme.namespace_header).add_modifier(Modifier::BOLD),
    )])
}
```

#### Indentation

| Niveau | Contexte | Indent |
|--------|----------|--------|
| Namespace | — | `` |
| Group | sous namespace | `  ` |
| Group | racine | `` |
| Environment | sous namespace + group | `    ` |
| Environment | sous group racine | `  ` |
| Server | sous namespace + group | `    ` ou `      ` |
| Server | sous group racine | `  ` ou `    ` |

`ConfigItem::Server` porte déjà `server.namespace` — l'indentation peut être calculée depuis `server.group_name`, `server.env_name` et `server.namespace`.

#### Panneau détails — namespace

Pour `ConfigItem::Namespace(label)`, le panneau détails affiche :
- Nom du namespace (label)
- Nombre de serveurs chargés depuis ce namespace
- Chemin du fichier source (nouveau champ `source_path: String` dans `NamespaceEntry`)

#### Warnings d'includes

Si des `IncludeWarning` sont collectés au chargement, `App::new()` les stocke dans un `Vec<IncludeWarning>` et affiche à la fermeture du splash initial (ou dans la barre de statut) une notification non-bloquante.
Alternative plus simple : les empiler dans `app.app_mode = AppMode::Error(...)` au démarrage, fermé par Entrée/Esc.

---

### `src/ui/theme.rs`

Ajout d'une couleur dédiée :

```rust
pub struct Theme {
    // …existant…
    pub namespace_header: Color,
}
```

Valeur par défaut suggérée : `mauve` (Catppuccin) pour distinguer visuellement namespace > group.

---

### `src/state.rs`

Les clés de persistance de l'expansion :

| Type | Clé |
|------|-----|
| Namespace | `NS:{label}` |
| Group (racine) | `Group:{name}` |
| Group (sous NS) | `NS:{label}:Group:{name}` |
| Environment | `NS:{label}:Env:{group}:{env}` ou `Env:{group}:{env}` |

---

### `src/i18n.rs`

Nouvelles clés dans `Strings` :

```rust
pub namespace_header_hint: &'static str, // "📦 {label} — {} serveurs" / "📦 {label} — {} servers"
pub include_warn_load:     &'static str, // "Impossible de charger '{}' : {}" / "Failed to load '{}': {}"
pub include_warn_circular: &'static str, // "'{}' crée une dépendance circulaire" / "'{}' creates a circular dependency"
pub include_warn_nested:   &'static str, // "Les includes imbriqués dans '{}' sont ignorés" / …
```

---

### `src/main.rs`

```rust
// Avant
let config = Config::load(&config_path)?;

// Après
let (config, warnings) = Config::load_merged(&config_path, &mut HashSet::new())?;
// warnings transmis à App::new()
```

`App::new()` accepte un nouveau paramètre `warnings: Vec<IncludeWarning>` et les stocke pour affichage.

---

## Ordre d'implémentation

1. **`src/config.rs`** — `IncludeEntry`, `NamespaceEntry`, `ConfigEntry::Namespace`, `Config.includes`, `ResolvedServer.namespace`, `IncludeWarning`, `Config::load_merged()`, adaptation de `resolve()`.

2. **`src/app.rs`** — `ConfigItem::Namespace`, `build_visible_items()`, `toggle_expansion()`, stockage des warnings.

3. **`src/ui/theme.rs`** — couleur `namespace_header`.

4. **`src/ui/mod.rs`** — rendu namespace, indentation corrigée, panneau détails namespace, affichage warnings.

5. **`src/i18n.rs`** — nouvelles clés FR/EN.

6. **`src/state.rs`** — nouvelles clés de persistance.

7. **`src/main.rs`** — `load_merged()`, transmission des warnings.

8. **Tests** — voir section dédiée.

---

## Tests

### `config::tests`

| Test | Description |
|------|-------------|
| `test_includes_basic` | Deux fichiers inclus → `ConfigEntry::Namespace` dans `groups`, `ResolvedServer.namespace` correctement renseigné |
| `test_includes_defaults_isolation` | Les `defaults` d'un fichier inclus n'affectent pas les serveurs du fichier principal |
| `test_includes_missing_file` | Fichier absent → `IncludeWarning::LoadError`, autres includes chargés normalement |
| `test_includes_circular` | Include circulaire → `IncludeWarning::Circular`, pas de boucle infinie |
| `test_includes_nested_ignored` | Include dans un sous-fichier → `IncludeWarning::NestedIgnored` |
| `test_includes_relative_path` | Chemin relatif résolu depuis le répertoire du fichier principal |
| `test_namespace_sort` | Namespaces triés alphabétiquement, serveurs au sein de chaque namespace triés indépendamment |

### `app::tests`

| Test | Description |
|------|-------------|
| `test_namespace_visibility` | Namespace fermé → seul l'en-tête visible |
| `test_namespace_expansion` | Namespace ouvert → groupes enfants visibles |
| `test_search_crosses_namespaces` | La recherche traverse tous les namespaces |

---

## Non inclus dans v0.7.0

- **Includes imbriqués** (includes dans un sous-fichier) — prévu v0.8.
- **Auto-discovery `~/.susshi_*.yml`** — alternative non retenue pour v0.7 (préférence pour la déclaration explicite).
- **Multi `--config` en CLI** — le flag `--config` reste mono-fichier ; les includes sont la voie officielle.
- **Reload à chaud** — rechargement sans redémarrer le TUI.
- **Fusion des `defaults`** — les defaults d'un sous-fichier restent locaux à ce fichier.
