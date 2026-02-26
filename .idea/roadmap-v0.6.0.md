# Roadmap v0.6.0

Fonctionnalité principale : **internationalisation (i18n) de l'interface TUI** — français et anglais, sélectionné automatiquement selon la locale système au lancement.

---

## Objectif

Tous les textes affichés dans la TUI (libellés, titres, barres de statut, messages d'erreur, hints) sont extraits dans un module `src/i18n.rs`. La langue est détectée une seule fois au démarrage via les variables d'environnement `LANG` / `LC_ALL` / `LC_MESSAGES`. Aucune dépendance externe : implémentation 100 % en Rust standard.

---

## Architecture technique

### `src/i18n.rs` — module central

```rust
pub enum Lang { Fr, En }

pub struct Strings {
    // ── Fenêtre erreur ────────────────────────────────────────
    pub error_title:          &'static str,  // " ⚠  Erreur " / " ⚠  Error "
    pub error_dismiss:        &'static str,  // "Appuyez sur Entrée ou Esc pour fermer" / "Press Enter or Esc to close"

    // ── Onglets connexion ─────────────────────────────────────
    pub tab_title:            &'static str,  // " Mode de Connexion (Tab to switch) " / " Connection Mode (Tab to switch) "

    // ── Toggle verbose ────────────────────────────────────────
    pub verbose_title:        &'static str,  // " Options (v to toggle) "
    pub verbose_label:        &'static str,  // "Verbose (-v)"

    // ── Barre de recherche ────────────────────────────────────
    pub search_title_idle:    &'static str,  // " Search (press /) " / " Recherche (/) "
    pub search_placeholder:   &'static str,  // "(search by name or host, ESC to cancel)" / "(nom ou hôte, Échap pour annuler)"
    // titres dynamiques → format!(lang.search_title_active, total)
    pub search_title_active:  &'static str,  // " 🔍 Search by name/host ({} servers) " / " 🔍 Recherche nom/hôte ({} serveurs) "
    pub search_no_results:    &'static str,  // " 🔍 No results for '{}' " / " 🔍 Aucun résultat pour '{}' "
    pub search_all_match:     &'static str,  // " 🔍 All {} servers match " / " 🔍 {} serveurs correspondent "
    pub search_partial:       &'static str,  // " 🔍 {} / {} servers " / " 🔍 {} / {} serveurs "
    pub search_result_all:    &'static str,  // " ✓ Showing all {} servers " / " ✓ {} serveurs affichés "
    pub search_result_partial:&'static str,  // " ✓ {} / {} servers match '{}' " / " ✓ {} / {} correspondent à '{}' "

    // ── Panneaux principaux ───────────────────────────────────
    pub panel_servers:        &'static str,  // " Servers " / " Serveurs "
    pub panel_details:        &'static str,  // " Details " / " Détails "
    pub details_placeholder:  &'static str,  // "Select a server to view details." / "Sélectionnez un serveur."

    // ── Libellés du panneau détails ───────────────────────────
    pub label_name:           &'static str,  // "Name:   "
    pub label_host:           &'static str,  // "Host:   " / "Hôte:   "
    pub label_port:           &'static str,  // "Port:   "
    pub label_user:           &'static str,  // "User:   " / "Util.:  "
    pub label_mode:           &'static str,  // "Mode:   "
    pub label_key:            &'static str,  // "Key:    " / "Clé:    "
    pub label_jump:           &'static str,  // "Jump:   " / "Rebond: "
    pub label_bastion:        &'static str,  // "Bastion:"
    pub label_options:        &'static str,  // "Options:"

    // ── Bloc diagnostic (System) ──────────────────────────────
    pub probe_section:        &'static str,  // "─── System ─────────────────────────"
    pub probe_hint:           &'static str,  // "  d — diagnostiquer" / "  d — probe"
    pub probe_running:        &'static str,  // "Diagnostic en cours…" / "Running probe…"
    pub probe_kernel:         &'static str,  // "Kernel   "
    pub probe_cpu:            &'static str,  // "CPU      "
    pub probe_load:           &'static str,  // "Load     "
    pub probe_ram:            &'static str,  // "RAM"
    pub probe_disk:           &'static str,  // "Disk /"
    pub probe_bastion_error:  &'static str,  // "Diagnostic non disponible en mode Bastion" / "Probe unavailable in Bastion mode"

    // ── Barre de statut ───────────────────────────────────────
    pub status_normal:        &'static str,  // "Navigate: ↑/↓ | Expand: Space/Enter | Search: / | Mode: Tab/1-3 | Verbose: v | y: Copy cmd | d: Probe | q: Quit"
    pub status_searching:     &'static str,  // "Search Mode: Type to filter | ESC: Cancel | Ctrl+U: Clear | Enter: Apply" / "Recherche : Tapez pour filtrer…"
    pub status_search_active: &'static str,  // "Navigate: ↑/↓ | Clear: ESC | New search: / | …"

    // ── Messages de statut (format strings) ──────────────────
    pub copied:               &'static str,  // "Copied: {}" / "Copié : {}"
    pub clipboard_error:      &'static str,  // "Clipboard error: {}" / "Erreur presse-papiers : {}"
    pub clipboard_unavailable:&'static str,  // "Clipboard unavailable" / "Presse-papiers indisponible"
    pub ssh_error:            &'static str,  // "SSH error: {}" / "Erreur SSH : {}"
}

/// Détecte la langue depuis `LC_ALL` → `LC_MESSAGES` → `LANG`.
/// Retourne `Lang::Fr` si la valeur commence par `"fr"`, `Lang::En` sinon.
pub fn detect_lang() -> Lang { /* … */ }

pub fn get_strings(lang: Lang) -> &'static Strings { /* … */ }
```

### Intégration dans `App`

```rust
pub struct App {
    // …
    pub lang: &'static Strings,
}
```

Initialisé dans `App::new()` :
```rust
lang: sushi::i18n::get_strings(sushi::i18n::detect_lang()),
```

Tous les textes hardcodés dans `ui/mod.rs`, `main.rs` et `probe.rs` sont remplacés par `app.lang.xxx` ou `format!(app.lang.xxx, valeur)`.

---

## Inventaire complet des textes à extraire

| Fichier | Ligne | Clé `Strings` |
|---------|-------|---------------|
| `ui/mod.rs` | 73 | `error_title` |
| `ui/mod.rs` | 95 | `error_dismiss` |
| `ui/mod.rs` | 120 | `tab_title` |
| `ui/mod.rs` | 150 | `verbose_title` |
| `ui/mod.rs` | 140 | `verbose_label` |
| `ui/mod.rs` | 163 | `search_placeholder` |
| `ui/mod.rs` | 165–177 | `search_title_active`, `search_no_results`, `search_all_match`, `search_partial` |
| `ui/mod.rs` | 184–190 | `search_result_all`, `search_result_partial` |
| `ui/mod.rs` | 200 | `search_title_idle` |
| `ui/mod.rs` | 314 | `panel_servers` |
| `ui/mod.rs` | 332 | `panel_details` |
| `ui/mod.rs` | 344 | `details_placeholder` |
| `ui/mod.rs` | 351–449 | `label_name`, `label_host`, `label_port`, `label_user`, `label_mode`, `label_key`, `label_jump`, `label_bastion`, `label_options` |
| `ui/mod.rs` | 455 | `probe_section` |
| `ui/mod.rs` | 458 | `probe_hint` |
| `ui/mod.rs` | 464–468 | `probe_running` |
| `ui/mod.rs` | 489–509 | `probe_kernel`, `probe_cpu`, `probe_load`, `probe_ram`, `probe_disk` |
| `ui/mod.rs` | 562–565 | `status_searching`, `status_search_active`, `status_normal` |
| `main.rs` | 377 | `copied` |
| `main.rs` | 380 | `clipboard_error` |
| `main.rs` | 383 | `clipboard_unavailable` |
| `main.rs` | 387 | `ssh_error` |
| `main.rs` | 403 | `probe_bastion_error` |
| `probe.rs` | 119 | `probe_bastion_error` |

---

## Ordre d'implémentation

1. **`src/i18n.rs`** — struct `Strings`, enum `Lang`, `detect_lang()`, deux statics `STRINGS_EN` / `STRINGS_FR`, `get_strings()`. Tests unitaires :
   - `detect_lang` avec `LANG=fr_FR.UTF-8` → `Lang::Fr`
   - `detect_lang` avec `LANG=en_US.UTF-8` → `Lang::En`
   - `detect_lang` sans variable définie → `Lang::En`

2. **`src/lib.rs`** — `pub mod i18n;`

3. **`src/app.rs`** — ajout du champ `pub lang: &'static Strings`, initialisation dans `App::new()`.

4. **`src/ui/mod.rs`** — remplacement de tous les littéraux par `app.lang.xxx`. Les format strings dynamiques (compteurs de serveurs, etc.) restent des `format!(app.lang.xxx, n)`.

5. **`src/main.rs`** — `set_status_message(format!(app.lang.copied, cmd))`, etc.

6. **`src/probe.rs`** — les messages d'erreur internes (non affichés en TUI) restent en anglais ; seuls `probe_bastion_error` et le message d'échec SSH sont externalisées.

7. **`src/handlers/mod.rs`** — les titres `["Direct [1]", "Rebond [2]", "Bastion [3]"]` passent dans `Strings`.

---

## Tests

- `i18n::tests` : détection locale (3 cas), valeurs Fr ≠ valeurs En (smoke test sur 3 clés).
- Pas de test de rendu TUI (non prévu à ce stade).

---

---

## Fonctionnalité secondaire : points de montage configurables dans le probe

### Objectif

En plus du point de montage fixe `/`, le probe peut lire l'utilisation de **filesystems supplémentaires** définis dans la configuration YAML. Si un filesystem déclaré est absent du serveur (occupation non retournée par `df`), un message explicite est affiché dans les résultats.

### Configuration YAML

Nouveau champ optionnel `probe_filesystems` au niveau `defaults`, groupe, environnement ou serveur :

```yaml
defaults:
  probe_filesystems:
    - /data
    - /var/log

groups:
  - name: "Storage"
    probe_filesystems:
      - /mnt/nas
      - /backup
    servers:
      - name: "nas01"
        host: "nas01.internal"
        probe_filesystems:   # surcharge locale (remplace héritage)
          - /mnt/data
```

L'héritage suit la même cascade que les autres champs : `defaults → groupe → environnement → serveur`.

### Architecture technique

**`src/config.rs`** — ajout du champ dans les structs existantes :

```rust
// Dans Defaults, Group, Environment, Server
pub probe_filesystems: Option<Vec<String>>,

// Dans ResolvedServer
pub probe_filesystems: Vec<String>,  // vide si non configuré
```

`resolve_server()` fusionne le champ en cascade (même logique que `ssh_options`).

**`src/probe.rs`** — extension de la commande distante :

La commande SSH exécutée passe de `df -h /` à une commande composite qui interroge `/` plus tous les points de montage configurés :

```rust
// Commande générée si probe_filesystems = ["/data", "/var/log"]
"df -h / /data /var/log 2>/dev/null; echo '---FS-DONE---'"
```

Le parsing du retour identifie chaque ligne par son point de montage.  
Si un filesystem demandé est absent de la sortie `df`, la struct résultat contient une entrée dédiée :

```rust
pub struct FsEntry {
    pub mountpoint: String,
    pub usage:      Option<FsUsage>,  // None → absent du serveur
}

pub struct FsUsage {
    pub size:    String,
    pub used:    String,
    pub avail:   String,
    pub percent: String,
}
```

**`src/ui/mod.rs`** — affichage dans le panneau détails :

- `/` est toujours affiché en premier (comportement actuel inchangé).
- Les filesystems additionnels suivent, un par ligne, avec le même format `Disk /data   10G / 2G (20%)`.
- Si `usage` est `None` : ligne affichée en jaune avec le libellé `⚠ /data — not mounted` (`probe_fs_absent` dans `Strings`).

**`src/i18n.rs`** — deux nouvelles clés dans `Strings` :

```rust
pub probe_disk_extra:  &'static str,  // "Disk {}"  (format string, mountpoint)
pub probe_fs_absent:   &'static str,  // "⚠ {} — not mounted" / "⚠ {} — non monté"
```

### Inventaire des textes supplémentaires

| Fichier | Clé `Strings` |
|---------|---------------|
| `ui/mod.rs` | `probe_disk_extra` |
| `ui/mod.rs` | `probe_fs_absent` |

### Ordre d'implémentation (ajout à la séquence existante)

Insérer entre les étapes 6 et 7 :

**6b.** `src/config.rs` — `probe_filesystems: Option<Vec<String>>` dans `Defaults`, `Group`, `Environment`, `Server` et `ResolvedServer` ; cascade dans `resolve_server()`.

**6c.** `src/probe.rs` — commande `df` étendue, parsing multi-filesystem, struct `FsEntry` / `FsUsage`.

**6d.** `src/ui/mod.rs` — rendu des entrées supplémentaires avec indicateur jaune si absent.

**6e.** `src/i18n.rs` — ajout de `probe_disk_extra` et `probe_fs_absent` dans `STRINGS_EN` et `STRINGS_FR`.

### Tests

- `config::tests` : `probe_filesystems` hérité correctement depuis `defaults` ; surcharge au niveau serveur.
- `probe::tests` : parsing `df` avec un filesystem absent → `FsEntry { usage: None }`.

---

## Non inclus dans cette version

- Autres langues (espagnol, allemand…) — l'architecture `Strings` est extensible mais seules Fr/En sont livrées.
- Traduction des messages `eprintln!` en CLI (hors TUI, en anglais uniquement).
- Traduction des chaînes dans les fichiers de config YAML ou les exemples.
