# Roadmap v0.11.0

> Version cible : **0.11.0**
> Thème : Config avancée, productivité & robustesse technique

---

## Objectifs

Cette version ne touche pas à la couche SSH de connexion principale. Elle consolide la dette
technique accumulée (SCP, tests) et enrichit la couche configuration pour que susshi soit
crédible dans des environnements entreprise avec des inventaires de serveurs importants.

Deux axes :

1. **Robustesse technique** — remplacement du spawn `scp` par `libssh2` native, tests
   d'intégration manquants sur les fonctions critiques.
2. **Config & productivité** — templating, tags, import, validation, hooks, ControlMaster,
   export Ansible.

---

## 1. Remplacement de `scp` par `libssh2` (SFTP natif)

### Contexte

L'implémentation actuelle dans `ssh/scp.rs` spawn un processus `scp` système et parse son
stderr pour extraire la progression. Ce design est fragile :
- `scp` est déprécié sur OpenSSH ≥ 9.0 (remplacé par SFTP en interne).
- Le parsing du pourcentage dépend du format de sortie de l'outil système.
- Aucun contrôle fin sur les erreurs réseau.

`libssh2-sys` est **déjà dans les dépendances**. La crate `ssh2` (wrapper safe) peut être
ajoutée sans impact sur le binaire final.

### Objectif

Remplacer `build_scp_args` + spawn système par un transfert SFTP pur via `ssh2::Sftp`,
dans un thread dédié qui émet des `ScpEvent` via `mpsc` (interface inchangée côté `App`).

### Structures Rust

```
ssh/scp.rs  → pub fn transfer_sftp(
                  server: &ResolvedServer,
                  mode: ConnectionMode,
                  direction: &ScpDirection,
                  local: &Path,
                  remote: &str,
                  tx: mpsc::Sender<ScpEvent>,
              ) -> Result<()>
              // Établit une session ssh2::Session sur local TCP, authentifie via
              // agent ou clé, ouvre un canal SFTP, transfère par chunks de 64 KiB
              // en émettant ScpEvent::Progress(pct) à chaque chunk.
```

- L'authentification suit la même priorité que le client système :
  agent SSH (`$SSH_AUTH_SOCK`) → clé explicite (`server.ssh_key`) → clé par défaut.
- Le mode Jump est géré en ouvrant d'abord une connexion TCP directe vers le jump host
  puis en créant un channel `direct-tcpip` vers la cible finale.
- Le mode Wallix reste désactivé (inchangé).
- `scp_child_pid: Option<u32>` dans `App` devient obsolète et peut être retiré.

### Avantages attendus

- Barre de progression précise et fiable (taille du fichier connue dès l'ouverture).
- Plus de dépendance sur l'utilitaire `scp` installé sur la machine.
- Gestion propre des erreurs réseau (codes SFTP standard).
- Compatibilité OpenSSH 9.x et serveurs SFTP-only (SFTP subsystem).

---

## 2. Tests d'intégration manquants

### Contexte

`tests/parse_full_config.rs` ne teste que la désérialisation YAML. Les fonctions de
construction d'arguments SSH et le cycle de vie des tunnels ne sont couverts par aucun test.

### Nouveaux tests

#### `tests/ssh_args.rs`

| Test | Scénario |
|------|----------|
| `direct_minimal` | Serveur sans clé, sans options → args minimaux |
| `direct_with_key` | Clé SSH explicite → `-i <path>` présent, tilde expandé |
| `direct_with_port` | Port non-standard → `-p <port>` présent |
| `direct_with_options` | `ssh_options` avec `-o` et option brute (préfixe `-`) |
| `jump_host` | Mode Jump → `-J <host>` présent, destination correcte |
| `jump_no_host` | Mode Jump sans `jump_host` configuré → `Err` |
| `wallix_template` | Mode Wallix → `-l user@target` construit depuis le template |
| `wallix_no_host` | Mode Wallix sans `bastion_host` configuré → `Err` |
| `destination_is_last` | Invariant : la destination est toujours le dernier argument |
| `system_ssh_config` | `use_system_ssh_config: true` → pas de `-F /dev/null` |

#### `tests/scp_args.rs` (ou `tests/sftp.rs` selon l'impl finale)

| Test | Scénario |
|------|----------|
| `upload_direct` | Upload direct → src/dst dans le bon ordre |
| `download_direct` | Download direct → src/dst inversés |
| `jump_host_forwarded` | Mode Jump → channel SFTP via jump correctement construit |
| `wallix_disabled` | Mode Wallix → `Err` retourné |

#### `tests/tunnel_handle.rs`

| Test | Scénario |
|------|----------|
| `new_is_idle` | Un `TunnelHandle::new()` a le statut `Idle` |
| `is_running_false_when_idle` | `is_running()` retourne `false` avant démarrage |
| `poll_returns_false_without_child` | `poll()` sans child retourne `false` sans paniquer |
| `build_tunnel_args_structure` | Arguments `-L port:host:port -N` présents dans les args |

### Infrastructure de test

Les tests d'args sont **purement unitaires** — aucun serveur SSH réel requis. Les types
`ResolvedServer` sont construits directement avec des `Default::default()` surchargés.

---

## 3. Import depuis `~/.ssh/config`

### Fonctionnalité

```
susshi --import-ssh-config [--output <path>] [--dry-run]
```

Lit `~/.ssh/config` (et les `Include` récursifs), groupe les entrées `Host` non génériques
et génère un bloc YAML compatible susshi vers `stdout` ou `--output`.

### Mapping

| Directive `ssh_config` | Champ susshi YAML |
|------------------------|-------------------|
| `Host` | `name` + `host` (si `HostName` absent → `Host` utilisé comme hôte) |
| `HostName` | `host` |
| `User` | `user` |
| `Port` | `port` |
| `IdentityFile` | `ssh_key` |
| `ProxyJump` | `jump_host` + `connection_mode: jump` |
| `ServerAliveInterval` | `ssh_options: ["ServerAliveInterval=<n>"]` |

- Les entrées `Host *` (wildcard) sont ignorées (converties en commentaire YAML).
- Les entrées avec `ProxyCommand` non-`ProxyJump` génèrent un avertissement.
- `--dry-run` affiche le YAML sans écrire de fichier.

### Structures Rust

```
// Nouveau module — ne dépend pas de la TUI.
src/import.rs  → pub fn import_ssh_config(path: &Path) -> Result<Vec<ServerConfig>>
                 pub fn import_to_yaml(servers: &[ServerConfig]) -> String
```

Le module utilise la crate `ssh2-config` (parse `~/.ssh/config`) ou un parseur
maison léger si la crate ajoute trop de poids.

---

## 4. Templating / variables dans la config YAML

### Problème

Pour une flotte homogène (ex: 20 workers avec le même jump host et la même clé), copier
chaque entrée serveur est source d'erreurs et rend les mises à jour fastidieuses.

### Syntaxe proposée

```yaml
_vars:
  jump: "bastion.prod.example.com"
  key: "~/.ssh/prod_ed25519"

groups:
  - name: workers
    environments:
      - name: prod
        defaults:
          jump_host: "{{ jump }}"
          ssh_key: "{{ key }}"
          connection_mode: jump
        servers:
          - name: "worker-{{ index }}"
            host: "10.0.1.{{ index }}"
          # index est résolu à la compilation de la liste
```

Variables scalaires uniquement en v0.11.0. Pas de boucles (`for`).

### Résolution

- Section `_vars` au niveau du fichier YAML (principal et includes).
- Interpolation `{{ var }}` sur tous les champs scalaires (`host`, `user`, `ssh_key`, etc.).
- Les includes ont leur propre scope `_vars` qui ne dépasse pas le fichier.
- Warning non-bloquant si une variable est référencée mais non définie.

### Structures Rust

```
config.rs  → struct VarMap(HashMap<String, String>)
             fn interpolate(s: &str, vars: &VarMap) -> String
             // Remplace {{ key }} par vars[key] ; laisse {{ key }} intact si absent.
```

---

## 5. Tags et filtres avancés

### Fonctionnalité

Nouvelle clé optionnelle `tags` sur les serveurs et les groupes :

```yaml
servers:
  - name: prod-api
    host: 10.0.0.1
    tags: [prod, eu-west, api, k8s]
```

### Recherche étendue

La barre de recherche (touche `/`) supporte les tags avec le préfixe `#` :

| Saisie | Comportement |
|--------|--------------|
| `prod` | Recherche textuelle sur `name` + `host` (comportement actuel) |
| `#prod` | Filtre sur le tag `prod` uniquement |
| `#prod #k8s` | Filtre AND : serveurs ayant **les deux** tags |
| `api #prod` | Texte `api` dans le nom **ET** tag `prod` |

### Filtres sauvegardés (v0.11.0 scope minimal)

- Un seul filtre nommé par fichier de config (suffit pour le cas d'usage courant).
- Défini dans la section `defaults` :

```yaml
defaults:
  default_filter: "#prod"
```

- Ce filtre est actif au démarrage et effaçable avec `Esc`.

### Structures Rust

```
config.rs   → ResolvedServer += tags: Vec<String>
app.rs      → App::matches_search() étendue pour parser les tokens #tag
              fn parse_search_tokens(q: &str) -> (Vec<String>, Vec<String>)
              //                                    text tokens  tag tokens
```

---

## 6. Validation de la config à la sauvegarde / rechargement

### Fonctionnalité

Commande CLI :

```
susshi --validate [<config_path>]
```

Valide la config et retourne exit code 0 (OK) ou 1 (erreurs). Destinée à être intégrée
dans un hook de pre-commit ou une CI.

### Checks effectués

| Check | Gravité |
|-------|---------|
| Syntaxe YAML | Erreur bloquante |
| Champs obligatoires (`host`, `user` résolus) | Erreur bloquante |
| Valeurs de port hors 1–65535 | Erreur bloquante |
| Champs inconnus (structs `#[deny_unknown_fields]`) | Warning |
| Jump host référencé mais vide | Warning |
| Template Wallix avec `{target_host}` absent | Warning |
| Tunnels avec `local_port` en double sur un même serveur | Warning |
| Variables `{{ var }}` non définies | Warning |

### Rechargement TUI (touche `R`)

Le rechargement existant acquiert désormais un hash SHA-256 du fichier avant et après.
Si le hash a changé **et** que la validation passe, la config est rechargée silencieusement.
Si la validation échoue, un `AppMode::Error` non-bloquant liste les nouvelles erreurs.

### Structures Rust

```
// Nouveau module (ou extension de config.rs)
src/validate.rs  → pub struct ValidationReport {
                       pub errors: Vec<ValidationError>,
                       pub warnings: Vec<ValidationWarning>,
                   }
                   pub fn validate_config(config: &Config) -> ValidationReport
```

---

## 7. Hooks `pre_connect` / `post_disconnect`

### Fonctionnalité

Deux hooks optionnels dans la config, activables globalement ou par serveur :

```yaml
defaults:
  pre_connect_hook: "~/.config/susshi/hooks/pre_connect.sh"
  post_disconnect_hook: "~/.config/susshi/hooks/post_disconnect.sh"

servers:
  - name: prod-restricted
    host: 10.0.0.50
    pre_connect_hook: "~/.config/susshi/hooks/notify_cmdb.sh"
```

### Comportement

- Le hook reçoit les variables d'environnement :
  `SUSSHI_SERVER`, `SUSSHI_HOST`, `SUSSHI_USER`, `SUSSHI_PORT`, `SUSSHI_MODE`.
- `pre_connect_hook` est exécuté **avant** le `connect()` (ou `exec`). Si le hook retourne
  un code non-zéro, la connexion est annulée avec un message dans la barre de statut.
- `post_disconnect_hook` est exécuté après le retour de `connect()`.
- Timeout configurable (défaut : 5 s) pour éviter le blocage sur un hook lent :

```yaml
defaults:
  hook_timeout_secs: 5
```

### Structures Rust

```
config.rs      → ResolvedServer += pre_connect_hook: Option<String>
                                   post_disconnect_hook: Option<String>
                                   hook_timeout_secs: u64
src/hooks.rs   → pub fn run_hook(path: &str, server: &ResolvedServer) -> Result<()>
                 // std::process::Command::new(path)
                 //   .envs([("SUSSHI_SERVER", ...), ...])
                 //   .timeout(Duration::from_secs(server.hook_timeout_secs))
                 //   .status()
```

---

## 8. Support `ControlMaster` / multiplexage SSH

### Fonctionnalité

Injection automatique des options `ControlMaster` et `ControlPath` dans les arguments SSH,
configurable globalement ou par serveur :

```yaml
defaults:
  control_master: true          # active le multiplexage
  control_path: "~/.ssh/ctl/%h_%p_%r"  # path par défaut
  control_persist: "10m"        # durée de vie du master après déconnexion
```

### Comportement

Quand `control_master: true`, `build_ssh_args()` injecte **avant la destination** :

```
-o ControlMaster=auto
-o ControlPath=<expanded_path>
-o ControlPersist=<persist>
```

- Le tilde dans `control_path` est expandé via `shellexpand::tilde`.
- Le répertoire parent de `ControlPath` est créé automatiquement si absent.
- Le mode Wallix **ne prend pas en charge** le multiplexage (désactivé silencieusement).

### Avantages

- Les connexions répétées vers le même hôte réutilisent le master TCP → quasi-instantanées.
- Le tunnel overlay bénéficie aussi du multiplexage si le master est déjà ouvert.

### Structures Rust

```
config.rs  → Defaults += control_master: Option<bool>
                          control_path: Option<String>
                          control_persist: Option<String>
             ResolvedServer += control_master: bool
                                control_path: String       // "" si désactivé
                                control_persist: String    // "10m" par défaut
ssh/client.rs → build_ssh_args() : injection des options ControlMaster si activé
```

---

## 9. Export inventaire Ansible

### Fonctionnalité

```
susshi --export ansible [--output <path>] [--filter <query>]
```

Génère un fichier d'inventaire Ansible INI ou YAML depuis la config susshi.

### Format de sortie (YAML Ansible)

```yaml
all:
  children:
    prod:
      hosts:
        prod-api:
          ansible_host: 10.0.0.1
          ansible_user: admin
          ansible_port: 22
          ansible_ssh_private_key_file: ~/.ssh/prod_ed25519
        prod-db:
          ansible_host: 10.0.0.10
          ansible_user: admin
          ansible_port: 22
    staging:
      hosts:
        ...
```

- Les **groupes** susshi deviennent des groupes Ansible (`children`).
- Les **environnements** deviennent des sous-groupes.
- Les **namespaces** (includes) deviennent un groupe de haut niveau.
- `--filter` accepte la même syntaxe que la recherche TUI (texte + `#tag`).
- `--output` écrit dans un fichier ; sans option → stdout.

### Structures Rust

```
// Nouveau module CLI-only
src/export/ansible.rs  → pub fn to_ansible_yaml(servers: &[ResolvedServer]) -> String
                          // Utilise serde_yaml pour produire le YAML Ansible.
```

---

## Ordre d'implémentation suggéré

1. **Tests d'intégration** (§2) — TDD first, définit les contrats des modules existants.
2. **SFTP natif** (§1) — supporte les tests unitaires écrits à l'étape 1.
3. **Validation** (§6) — base pour les étapes suivantes qui étendent le schéma YAML.
4. **Templating** (§4) — étend `config.rs`, tests dans le module.
5. **Tags** (§5) — étend `ResolvedServer`, étend `matches_search()`.
6. **Import `~/.ssh/config`** (§3) — module isolé, CLI uniquement.
7. **ControlMaster** (§8) — modification de `build_ssh_args()`, couvert par les tests §2.
8. **Hooks** (§7) — nouveau module `src/hooks.rs`, intégration dans `app.rs`.
9. **Export Ansible** (§9) — module isolé, CLI uniquement.
10. **CHANGELOG & doc** — mise à jour `CHANGELOG.md`, `README.md`, `full_config.yaml`.
11. **Release** — `cargo fmt --all -- --check`, `cargo clippy -- -D warnings`, `cargo test`, PR → merge.

---

## Ce qui ne change pas

- Le schéma YAML de base reste rétrocompatible (toutes les nouvelles clés sont optionnelles).
- Le mode Wallix reste pleinement supporté pour la connexion directe.
- L'interface `ScpEvent` / `mpsc` côté `App` reste inchangée (seul le backend change en §1).
- Aucun breaking change sur `build_ssh_args()` (les callers existants ne sont pas modifiés,
  uniquement les args produits peuvent contenir les nouvelles options ControlMaster).

---

## Versioning

| Version  | Contenu                                                              |
|----------|----------------------------------------------------------------------|
| 0.10.x   | Tunnels SSH + SCP (patch fixes)                                      |
| **0.11.0** | Config avancée, productivité & robustesse technique (cette roadmap) |
| 1.0.0    | Stabilisation schéma YAML, documentation finale                      |
