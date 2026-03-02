# susshi — Instructions pour l'agent de développement

Ce fichier est en français, à usage interne. Communiquer avec l'utilisateur en utilisant le tutoiement.

---

## Projet

- **Nom du binaire** : `susshi`
- **Langage** : Rust (édition 2024), TUI via `ratatui` + `crossterm`, thème Catppuccin
- **Dépôt** : https://github.com/yatoub/susshi
- **Roadmaps** : dans `.idea/` (usage interne uniquement)

---

## Langues

| Contexte | Langue |
|---|---|
| Code source, doc publique (README, CHANGELOG, docstrings) | **Anglais** |
| Commits, PR, issues, noms de branches | **Anglais** |
| Commentaires privés dans le code | Français autorisé |
| Instructions (ce fichier), discussions techniques | **Français** |

---

## Commits

- Respecter [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) **obligatoirement**.
- Types courants : `feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`, `perf:`
- Le corps du commit peut être en français.
- Ne jamais écrire le message de commit en multiligne dans le shell : utiliser un fichier temporaire avec `git commit -F /tmp/msg.txt`.

---

## Développement

- **TDD** : écrire les tests avant l'implémentation.
- `cargo fmt` avant tout commit.
- `cargo clippy -- -D warnings` doit passer sans erreur.
- `cargo test` doit passer (tous les tests, y compris les tests d'intégration dans `tests/`).
- Aucun `unwrap()` dans le code de production sans justification explicite.
- Le code Unix-spécifique (`libc`, `nix`, `CommandExt::exec/pre_exec`) doit être gaté derrière `#[cfg(unix)]`. `nix` est dans `[target.'cfg(unix)'.dependencies]` dans `Cargo.toml`.

---

## Chaîne CI/CD

La chaîne de release est **entièrement automatisée** via release-plz.

### ⚠️ Interdictions absolues
- Ne **jamais** modifier le champ `version` dans `Cargo.toml` manuellement dans le cadre d'une release.
- Ne **jamais** pousser un tag `v*.*.*` manuellement — cela court-circuiterait release-plz.
- Ne **jamais** éditer le `CHANGELOG.md` pour les versions futures — release-plz le génère depuis les commits.

### Flux automatique

```
commit feat:/fix: → push master
  ├── ci.yml          : fmt + clippy + tests
  └── release-plz.yml : ouvre/met à jour une PR "chore: release vX.Y.Z"
                        (bump Cargo.toml + CHANGELOG auto)

merge de la PR release-plz
  └── release-plz.yml : crée le tag vX.Y.Z + GitHub Release

push tag v*.*.*
  └── release.yml     : build Linux x86_64 / macOS Intel / macOS ARM / Windows x86_64
                        → binaires attachés à la GitHub Release
```

### Workflows

| Fichier | Déclencheur | Rôle |
|---|---|---|
| `ci.yml` | push `master` + PR | fmt, clippy, tests |
| `release-plz.yml` | push `master` | PR de release + tag + GitHub Release |
| `release.yml` | push tag `v*.*.*` | build multiplateforme + upload binaires |
| `aur-publish.yml` | push tag `v*.*.*` | mise à jour du PKGBUILD AUR |
