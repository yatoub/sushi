🛠 La Stack Technique Confirmée
- Langage : Rust (Edition 2021)
- TUI : ratatui + crossterm (pour le support souris/clavier)
- SSH : ssh2-rs (binding libssh2) pour la gestion native
- Config : serde + serde_yaml
- Thème : Catppuccin (Mocha par défaut)
- Terminal : portable-pty (pour relayer le canal SSH vers le terminal local)

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

groups:
  - name: "Projet Alpha"
    user: "dev_user"  # Surcharge le global pour tout le groupe
    environments:
      - name: "Production"
        ssh_key: "~/.ssh/prod_key" # Surcharge pour cet environnement uniquement
        servers:
          - name: "web-01"
            host: "10.0.0.1"
            # Résultat : user="dev_user", key="prod_key"
          - name: "db-01"
            host: "10.0.0.2"
            user: "postgres" # Surcharge spécifique au serveur
            # Résultat : user="postgres", key="prod_key"
```