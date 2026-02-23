🛠 La Stack Technique Confirmée
- Langage : Rust (Edition 2021)
- TUI : ratatui + crossterm (pour le support souris/clavier)
- SSH : Handover via `exec` (commande système `ssh`) pour remplacer le processus courant
- Config : serde + serde_yaml
- Thème : Crate `catppuccin` (Flavor Mocha)
- Terminal : Pas de gestion PTY interne, le handover laisse `ssh` gérer le terminal

📁 Structure du Projet (Workspace)
L'agent devra suivre cette organisation pour séparer les responsabilités :

```Plaintext
sushi/
├── src/
│   ├── main.rs          # Point d'entrée, gestion du panic hook
│   ├── app.rs           # Logique de l'état de l'application (Tabs, Filtres)
│   ├── config.rs        # Parsing YAML et mapping des serveurs
│   ├── ui/              # Composants Ratatui (Tabs, Tree, SearchBar)
│   │   ├── mod.rs
│   │   ├── theme.rs     # Définition des couleurs Catppuccin
│   │   └── widgets/     # Widgets personnalisés (Arborescence)
│   ├── ssh/             # Logique de connexion et gestion du PTY
│   └── handlers/        # Gestion des événements (Clavier/Souris)
└── tests/               # Tests d'intégration (TDD)
```
📝 Schéma de Configuration YAML
Voici le modèle de données que l'agent doit valider (gestion de la profondeur 1 à 3) :

```YAML
# sushi.yaml
defaults:
  user: "default_user"
  ssh_key: "~/.ssh/id_rsa"
  # Surcharge globale possible pour le mode Rebond ou Bastion
  jump_host: "gateway.corp"
  jump_user: "gatekeeper"
  bastion_host: "bastion.corp"
  bastion_user: "root"
  bastion_template: "{target_user}@%n:SSH:{bastion_user}"

groups:
  - name: "Projet Alpha"
    user: "dev_user"  # Surcharge le global pour tout le groupe
    environments:
      - name: "Production"
        ssh_key: "~/.ssh/prod_key" # Surcharge pour cet environnement uniquement
        servers:
          - name: "web-01"
            host: "10.0.0.1"
            # jump_host hérité
          - name: "db-01"
            host: "10.0.0.2"
            user: "postgres" # Surcharge spécifique au serveur
            # Résultat : user="postgres", key="prod_key"

```

## Nouvelles Fonctionnalités Requises

1. **Modes de Connexion (Tabs)**
   - 3 Onglets sélectionnables (Tab ou Clic Souris) : Direct, Rebond (ProxyJump), Bastion.
   - **Direct** : `ssh -p <port> <user>@<host>`
   - **Rebond** : `ssh -J <jump_user>@<jump_host> <user>@<host>`
   - **Bastion** : `ssh -l "<target_user>@<target_host>:SSH:<bastion_user>" <bastion_host>`

2. **Comportement Handover**
   - Sushi doit restaurer le terminal et invoquer `exec()` pour remplacer son processus par `ssh`.
   - `Ctrl+D` ou `exit` depuis la session SSH ferme complètement l'application (pas de retour au menu).
   - Ignorer impérativement `~/.ssh/config` (`-F /dev/null`).

3. **Navigation Souris**
   - Clic simple : Sélectionner un serveur.
   - Double-clic : Lancer la connexion.
   - Clic sur les onglets : Changer de mode.

4. **Fichier de configuration YAML**
   - validation stricte du schéma (profondeur 1 à 3).
   - support des variables d'environnement (ex: `~/.ssh/id_rsa`).
   - gestion des héritages et surcharges (global → groupe → environnement → serveur).
   - gestion des options ssh globales et spécifiques pour chaque mode (ex: `jump_host`, `bastion_template`).
   - utilise les fichiers YAML situés dans tests/fixtures/ pour valider le comportement du parser.
   - le fichier exemple `examples/full_config.yaml` doit être utilisé comme référence pour les tests de parsing et de validation.
