# Roadmap v0.5.0

Reprend les items non implémentés de la v0.4.0 et ajoute une nouvelle fonctionnalité de diagnostic.

---

## Reports depuis v0.4.0

- [ ] **1. Supprimer ou exploiter `portable-pty`**
  - Fichier : `Cargo.toml`
  - La dépendance `portable-pty = "0.9.0"` est présente mais inutilisée.
  - **Option A (simple)** : la retirer pour alléger le binaire.
  - **Option B (ambitieuse)** : l'utiliser pour implémenter une session SSH inline dans un panneau de la TUI (PTY géré par susshi, sans quitter l'interface).

- [ ] **2. Multi-sauts SSH (ProxyJump chaîné)**
  - Fichiers : `src/config.rs`, `src/ssh/client.rs`, `examples/full_config.yaml`
  - Changer `rebond: Option<JumpConfig>` → `rebond: Option<Vec<JumpConfig>>`.
  - Construire la chaîne `-J user1@host1[:port],user2@host2[:port]` dans `build_ssh_args()`.
  - Mettre à jour les tests unitaires dans `ssh/client.rs`.
  - Documenter dans `examples/full_config.yaml`.

---

## Nouvelle fonctionnalité

- [ ] **3. Diagnostic rapide de connexion (`d`)**

  ### Objectif
  Appuyer sur `d` sur un serveur sélectionné lance un diagnostic non-bloquant via SSH
  et affiche un résumé système (kernel, CPU, RAM, disque) dans un bloc dédié de la TUI.

  ### Commande SSH envoyée
  Une seule invocation SSH avec un one-liner bash :
  ```bash
  ssh [args] "uname -r; \
    awk '/^model name/{sub(/.*: /,\"\"); print; exit}' /proc/cpuinfo; \
    uptime | awk -F'load average:' '{print $2}' | xargs; \
    free -b | awk '/^Mem/{printf \"%.0f %.0f\n\", $3/$2*100, $2}'; \
    df -B1 / | awk 'NR==2{printf \"%.0f %.0f\n\", $3/$2*100, $2}'"
  ```
  Retourne 5 lignes : kernel, CPU model, load avg (1/5/15m), RAM (% used, total bytes), Disk / (% used, total bytes).

  ### Architecture technique
  - `build_ssh_args()` est réutilisée telle quelle — seul `connect()` fait un `exec`.
  - Nouvelle fonction `fn probe(server: &ResolvedServer, mode: ConnectionMode) -> Result<ProbeResult>` dans `src/ssh/client.rs` :
    - Construit les mêmes args que `build_ssh_args()` + le one-liner en argument final.
    - Lance via `std::process::Command::new("ssh").args(...).output()` (capture stdout, non-exec).
    - Timeout via `std::process::Child::wait_timeout` ou thread + channel.
  - `ProbeResult` struct (dans `src/ssh/client.rs` ou `src/probe.rs`) :
    ```rust
    pub struct ProbeResult {
        pub kernel: String,       // "6.1.0-28-amd64"
        pub cpu_model: String,    // "Intel Xeon E5-2670"
        pub load: String,         // "0.42, 0.38, 0.31"
        pub ram_pct: u8,          // 67
        pub ram_total_gb: f32,    // 15.6
        pub disk_pct: u8,         // 23
        pub disk_total_gb: f32,   // 465.0
    }
    ```
  - Le résultat est stocké dans `App` :
    ```rust
    pub probe_result: Option<ProbeResult>,   // None = pas encore lancé
    pub probe_state: ProbeState,             // Idle | Running | Done | Error(String)
    ```
  - Le probe tourne dans un thread dédié ; le résultat est envoyé via `std::sync::mpsc::channel` et lu dans la boucle d'événements TUI (poll avec timeout 250 ms).

  ### Affichage dans la TUI
  Le panneau détail (droite) affiche un nouveau bloc **System** quand `probe_state == Done` :

  ```
  ┌─ System ─────────────────────────┐
  │ Kernel   6.1.0-28-amd64          │
  │ CPU      Intel Xeon E5-2670      │
  │ Load     0.42 / 0.38 / 0.31      │
  │ RAM      ████████░░░░  67%  16GB │
  │ Disk /   ████░░░░░░░░  23% 465GB │
  └──────────────────────────────────┘
  ```

  Barres de progression ratatui (`Gauge` ou `LineGauge`) colorées :
  - < 60 % → green
  - 60–85 % → yellow
  - > 85 % → red

  Pendant l'exécution (`ProbeState::Running`) : spinner animé (`⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏`) à la place du bloc.

  ### Raccourci et UX
  | Touche | Action |
  |--------|--------|
  | `d` | Lance le diagnostic sur le serveur sélectionné |
  | `d` (à nouveau) | Relance / rafraîchit |
  | Changement de sélection | Réinitialise `probe_state` → `Idle` |

  ### Tests
  - Mock du one-liner bash (output statique) pour tester `ProbeResult::parse(raw: &str)`.
  - Test du parsing de chaque ligne (kernel, load, ram, disk).

---

## Ordre d'implémentation suggéré

1. **Item 1 — Option A** : retirer `portable-pty` (5 min, risque zéro).
2. **Item 2** : multi-sauts SSH (config + args + tests).
3. **Item 3** : diagnostic — dans cet ordre :
   a. `ProbeResult` + parsing + tests unitaires
   b. `probe()` avec `Command::output()` + thread + channel
   c. Intégration dans `App` (`probe_state`, receiver)
   d. Affichage dans `draw_details()` (spinner + bloc System)
   e. Raccourci `d` dans `main.rs`
