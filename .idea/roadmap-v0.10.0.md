# Roadmap v0.10.0

> Version cible : **0.10.0**
> Thème : Tunnels SSH & Transfert de fichiers SCP

---

## Objectifs

Deux nouvelles fonctionnalités majeures, toutes deux **désactivées en mode Wallix** (un bastion
Wallix contrôle et journalise les sessions — il ne propage ni le forwarding de port arbitraire ni
le protocole SCP/SFTP).

---

## 1. Tunnels SSH

### Config YAML

Nouvelle clé `tunnels` disponible à tous les niveaux (`defaults`, `groups`, `servers`).
La résolution suit la même logique d'héritage que `ssh_options`.

```yaml
defaults:
  user: admin

servers:
  - name: prod-db
    host: 10.0.0.10
    tunnels:
      - local_port: 5432
        remote_host: 127.0.0.1
        remote_port: 5432
        label: "PostgreSQL"
      - local_port: 6379
        remote_host: 127.0.0.1
        remote_port: 6379
        label: "Redis"
```

Champs par tunnel :

| Champ           | Type     | Obligatoire | Description                              |
|-----------------|----------|-------------|------------------------------------------|
| `local_port`    | `u16`    | oui         | Port local écouté (`localhost:X`)        |
| `remote_host`   | `String` | oui         | Hôte côté distant                        |
| `remote_port`   | `u16`    | oui         | Port côté distant                        |
| `label`         | `String` | non         | Nom affiché dans l'UI                    |

### Structures Rust

```
config.rs   → struct TunnelConfig { local_port, remote_host, remote_port, label }
              ResolvedServer += tunnels: Vec<TunnelConfig>

app.rs      → enum TunnelStatus { Idle, Active(Child), Error(String) }
              struct TunnelHandle { config: TunnelConfig, status: TunnelStatus }

              /// Champ actif dans le formulaire d'édition/création.
              enum TunnelFormField { Label, LocalPort, RemoteHost, RemotePort }

              /// État du formulaire tunnel (édition ou création).
              struct TunnelForm {
                  label:       String,
                  local_port:  String,   // saisie libre puis validée en u16
                  remote_host: String,
                  remote_port: String,
                  focus:       TunnelFormField,
                  /// Some(idx) = édition du tunnel à cet index ; None = création
                  editing_index: Option<usize>,
                  /// Erreur de validation par champ (None si valide)
                  errors: HashMap<TunnelFormField, String>,
              }

              enum TunnelOverlayState {
                  /// Liste des tunnels (vue principale de l'overlay)
                  List { selected: usize },
                  /// Formulaire d'édition ou de création
                  Form(TunnelForm),
              }

              App += tunnels: HashMap<ServerKey, Vec<TunnelHandle>>
                     tunnel_rx: Option<mpsc::Receiver<TunnelEvent>>
                     tunnel_overlay: Option<TunnelOverlayState>  // Some = overlay ouvert
```

### Backend

- Construit les args SSH comme `build_ssh_args` mais ajoute `-L local:remote_host:remote_port -N`.
- Lance le subprocess via `Command::spawn()` (pas `exec()`) pour rester non-bloquant.
- Un thread de surveillance `mpsc` notifie l'`App` si le subprocess se termine inopinément.
- Plusieurs tunnels peuvent être actifs simultanément (tous sur le même serveur ou sur des
  serveurs différents).

### UI/UX

**Touche `T`** sur un serveur sélectionné :

- En mode **Wallix** → message dans la barre de statut : `"Tunnels non disponibles en mode Wallix"` (pas d'overlay).
- Sinon → **overlay flottant centré** listant les tunnels configurés pour ce serveur :

```
┌─ Tunnels — prod-db ──────────────────────────────────┐
│  ✔  PostgreSQL   localhost:5432 → 127.0.0.1:5432     │
│  ✖  Redis        localhost:6379 → 127.0.0.1:6379     │
│  +  (nouveau tunnel)                                 │
│                                                      │
│  ↑↓ naviguer   Enter démarrer/arrêter                │
│  e éditer      a ajouter   Del supprimer             │
│  q/Esc fermer                                        │
└──────────────────────────────────────────────────────┘
```

- `Enter` démarre ou arrête le tunnel sélectionné.
- `e` sur un tunnel → **formulaire d'édition** (voir ci-dessous).
- `a` → **formulaire de création** (champs vides).
- `Del` sur un tunnel actif → arrêt puis suppression ; sur un tunnel inactif → suppression directe
  avec confirmation courte dans la barre de statut (`"Tunnel supprimé"`).  
  Un tunnel issu du YAML est marqué ~~barré~~ et caché jusqu'au prochain rechargement — il ne
  peut pas être réellement supprimé du fichier depuis la TUI (voir *Persistance* ci-dessous).

**Formulaire d'édition / création** (overlay centré) :

```
┌─ Modifier le tunnel — prod-db ─────────────────────────┐
│  Label        : [PostgreSQL                          ] │
│  Port local   : [5432                                ] │
│  Hôte distant : [127.0.0.1                           ] │
│  Port distant : [5432                                ] │
│                                                        │
│  Tab changer de champ   Enter valider   Esc annuler    │
└────────────────────────────────────────────────────────┘
```

- Validation à la soumission : ports entiers 1–65535, `remote_host` non vide.
- En cas d'erreur : le champ invalide est surligné en rouge, focus maintenu.
- Un **badge** dans le panneau de détails (1/3 droit) indique le nombre de tunnels actifs :
  `Tunnels actifs : 2`.
- Les tunnels actifs survivent à la navigation dans la liste (ils tournent en arrière-plan).
- À la fermeture de l'application, tous les subprocessus sont `kill()`és proprement.

### Persistance des modifications TUI

**Principe** : le fichier YAML n'est jamais modifié depuis la TUI (préserver les commentaires,
la mise en forme et la source de vérité de l'équipe). Les modifications sont stockées en
**couche d'overrides** dans `~/.susshi_state.json`.

**Stratégie de fusion** (à la résolution au démarrage) :

```
tunnels effectifs = tunnels YAML  ←  overrides state.json
```

| Action TUI                  | Effet dans `state.json`                                      |
|-----------------------------|--------------------------------------------------------------|
| Édition d'un tunnel YAML    | Override stocké avec `server_key` + index d'origine          |
| Ajout d'un tunnel           | Nouveau tunnel marqué `source: "user"` dans les overrides    |
| Suppression d'un tunnel YAML| Tunnel marqué `hidden: true` dans les overrides              |
| Suppression d'un tunnel user| Entrée retirée des overrides                                 |

**Structures Rust supplémentaires** :

```
state.rs  → struct TunnelOverride {
                server_key: String,
                /// None = tunnel ajouté par l'utilisateur (pas d'origine YAML)
                yaml_index: Option<usize>,
                config: TunnelConfig,
                hidden: bool,   // true = supprimé depuis la TUI
            }
            AppState += tunnel_overrides: Vec<TunnelOverride>
```

Ce mécanisme garantit qu'un `reload` de la config (touche `R`) réapplique les overrides
en conservant les modifications utilisateur, sauf si le YAML lui-même a changé en sens
contraire (l'override gagne — l'utilisateur est averti dans la barre de statut).

---

## 2. Transfert de fichiers SCP

### Contrainte Wallix

Désactivé en mode Wallix, comme les tunnels. Message dans la barre de statut si tenté.

### Backend

- Réutilise `build_ssh_args` pour extraire la clé, le port, le jump host (`-J`).
- Construit les args `scp` : `scp -i key -P port [-J jump] src dst`.
- Subprocess non-bloquant avec capture de stdout/stderr.
- Parse le pourcentage de progression depuis la sortie de `scp -v`.

### Structures Rust

```
app.rs  → enum ScpDirection { Upload, Download }
          enum ScpState {
              Idle,
              SelectingDirection,
              FillingForm { direction: ScpDirection, local: String, remote: String, focus: FormField },
              Running { direction: ScpDirection, progress: u8, label: String },
              Done { label: String },
              Error(String),
          }
          App += scp_state: ScpState
                 scp_rx: Option<mpsc::Receiver<ScpEvent>>
```

### UI/UX

**Touche `s`** sur un serveur sélectionné :

**Étape 1 — Choix de direction** (overlay centré petit) :

```
┌─ Transfert SCP ──────────┐
│  ↑  Upload  (local → srv)│
│  ↓  Download (srv → local)│
│  Esc annuler             │
└──────────────────────────┘
```

**Étape 2 — Formulaire** (overlay centré) :

```
┌─ SCP Upload — prod-db ─────────────────────┐
│  Local  : ~/exports/dump.sql               │
│  Distant: admin@10.0.0.10:/home/admin/     │
│                                            │
│  Tab changer de champ  Enter confirmer     │
│  Esc annuler                               │
└────────────────────────────────────────────┘
```

- Le champ **Distant** est pré-rempli avec `user@host:~`.
- **Tab** bascule le focus entre les deux champs.
- Complétion `~` expandée via `shellexpand::tilde` à la soumission.

**Étape 3 — Progression** (dans le panneau de détails, pas d'overlay bloquant) :

```
SCP Upload en cours...
dump.sql → prod-db:/home/admin/
[███████░░░░░░░░░░░░░] 38%
```

**Fin** : message dans la barre de statut `"SCP terminé ✔"` ou `"SCP échoué : <erreur>"`.

---

## Ordre d'implémentation suggéré

1. **Config** — `TunnelConfig`, héritage/résolution dans `ResolvedServer`, parsing YAML, tests.
2. **State overrides** — `TunnelOverride` dans `AppState`, fusion config+overrides à la résolution.
3. **Backend tunnels** — `TunnelHandle`, spawn/kill, thread de surveillance, `TunnelEvent`.
4. **UI tunnels — liste** — overlay `T`, `Enter` start/stop, badge dans le panneau de détails, gestion du mode Wallix, i18n des nouveaux libellés.
5. **UI tunnels — formulaire** — `TunnelFormState`, `e` édition, `a` création, `Del` suppression, validation inline, i18n des nouveaux libellés.
6. **Backend SCP** — `ScpState`, construction des args, parsing de la progression, `ScpEvent`.
7. **UI SCP** — overlay `s` en deux étapes, barre de progression dans le panneau de détails.
8. **Nettoyage** — `Drop` sur `App` pour tuer les subprocessus actifs, i18n des nouveaux libellés.
9. **Tests** — unit tests sur la construction des args (tunnels + scp), fusion overrides, test d'intégration parse config.
10. **CHANGELOG & doc** — mise à jour `CHANGELOG.md`, `README.md`, `full_config.yaml`.
11. **Tests et release** - cargo test, cargo fmt, cargo clippy et tag avec changelog

---

## Ce qui ne change pas

- Le mécanisme de connexion SSH principal (`exec`) reste inchangé.
- Le mode Wallix reste pleinement fonctionnel pour la connexion directe.
- Aucun breaking change sur le schéma YAML existant (nouvelles clés optionnelles uniquement).

---

## Versioning

| Version  | Contenu                                          |
|----------|--------------------------------------------------|
| 0.10.0   | Tunnels SSH + SCP (cette roadmap)                |
| 0.11.0   | À définir (retours d'usage sur le SCP/tunnels)   |
| 1.0.0    | Stabilisation schéma YAML, documentation finale  |

La **1.0.0** sera le signal que le schéma de configuration YAML est stable et ne subira plus
de breaking changes sans bump de version majeure.
