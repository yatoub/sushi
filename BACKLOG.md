# susshi — Backlog d'améliorations

Repo : https://github.com/yatoub/susshi  
Stack : Rust, Ratatui, YAML, SSH2, Catppuccin  

---

## 🔴 Priorité haute

- Ajouter `cargo audit` au pipeline CI (sécurité des dépendances)
- Migrer `serde_yaml 0.9` (deprecated) vers `serde-yaml-ng` ou `figment`
- Configurer l'auto-merge Dependabot
- Corriger la doc : le README référence v0.15 mais la dernière release est v0.14.0
- Intégrer des badges dans le README pour refleter la qualité de code
- Fixer les options release Cargo : `codegen-units = 1`, `incremental = false` pour les binaires distribués
- Ajouter un `SECURITY.md` au projet

## 🟠 Priorité moyenne

- Ajouter `cargo clippy -- -D warnings` et `cargo fmt --check` comme jobs CI bloquants
- Créer des templates GitHub pour les issues et les PRs
- Publier le package sur crates.io (`cargo install susshi`)
- Étoffer CONTRIBUTING.md : setup dev, lancement des tests, conventions de commits
- Ajouter la mesure de couverture de tests (cargo-tarpaulin ou cargo-llvm-cov, seuil ~70%)

## 🟡 Priorité normale

- Découper en workspace multi-crates : `susshi-config`, `susshi-tui`, `susshi-ssh`, `susshi-cli`
- Ajouter support SSH agent forwarding (`agent_forwarding: true` dans la config)
- Clarifier le support Windows : stubs no-op trompeurs, documenter ou implémenter
- Fuzzing sur le parsing YAML de config (`cargo-fuzz`)
- Benchmarks sur les chemins critiques (parsing, filtrage) avec `criterion`

## 🟢 Long terme / nouvelles fonctionnalités

### Connectivité
- Reconnexion automatique avec backoff en mode `keep_open`
- Multiplexage ControlMaster avec affichage des sessions actives
- Gestion des password
  si vous n'avez pas de clé ssh enregistré sur le bastion phm la connexion est impossible
  pour l'instant je gère mal (quand c'est le cas) le fallback password
  je n'ai pas test non plus une clé ssh avec password
  et les messages de clé (known_host) cassent la TUI
  il faut implémenter un prompt password en cas de besoin de la connexion ssh


### Inventaire & intégration
- Exécution en masse sur un groupe (`susshi exec --group prod "uptime"`)
- Includes depuis une URL HTTPS (inventaire d'équipe partagé)
- `--list --json` pour pipe vers `jq` / `fzf`
- Export vers d'autres outils en plus d'Ansible, exporter vers Terraform inventory et Nmap target lists.

### Sécurité
- Intégration SSH agent (détection automatique des identités)
- Audit log local des connexions (timestamp, durée, code de sortie)

### Packaging
- Ajouter dans les paquets linux une manpage et un exemple de la configuration dans `/usr/share/doc/susshi`