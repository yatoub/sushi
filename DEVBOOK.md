# DEVBOOK - Sushi SSH Connection Manager

## Méthodologie
Ce projet suit une approche TDD (Test Driven Development). Pour chaque fonctionnalité :
1. Écriture des tests
2. Implémentation du code minimal pour faire passer les tests
3. Refactoring du code

## Structure du Projet
- `cmd/` : Point d'entrée de l'application
- `internal/` : Code interne de l'application
- `pkg/` : Packages réutilisables
- `tests/` : Tests d'intégration

## Étapes de Développement

### 1. Configuration de Base [ ]
- [ ] Mise en place de la structure du projet Go
- [ ] Configuration de l'environnement de test
- [ ] Création du fichier go.mod
- [ ] Configuration des linters et outils de qualité de code

### 2. Gestion de la Configuration YAML [ ]
- [ ] Tests de parsing du fichier YAML
- [ ] Structure de données pour les serveurs et groupes
- [ ] Validation de la configuration
- [ ] Gestion des erreurs de configuration

### 3. Interface CLI Interactive [ ]
- [ ] Tests du menu interactif
- [ ] Implémentation de la navigation dans l'arborescence
- [ ] Gestion des entrées utilisateur
- [ ] Affichage formaté des groupes et serveurs

### 4. Gestion des Connexions SSH [ ]
#### 4.1 Mode Rebond SSH [ ]
- [ ] Tests de configuration SSH avec rebond
- [ ] Implémentation de la connexion via ProxyJump
- [ ] Gestion des clés SSH
- [ ] Validation des paramètres de connexion

#### 4.2 Mode Bastion [ ]
- [ ] Tests de configuration SSH avec bastion
- [ ] Implémentation de la connexion via bastion
- [ ] Gestion du ForwardAgent
- [ ] Validation des paramètres de connexion

### 5. Gestion des Erreurs et Logging [ ]
- [ ] Tests des différents scénarios d'erreur
- [ ] Implémentation du système de logging
- [ ] Messages d'erreur utilisateur
- [ ] Journalisation des connexions

### 6. Tests d'Intégration [ ]
- [ ] Tests avec des serveurs de test
- [ ] Validation des deux modes de connexion
- [ ] Tests de performances
- [ ] Tests de charge

### 7. Documentation [ ]
- [ ] Documentation du code
- [ ] Documentation utilisateur
- [ ] Exemples de configuration
- [ ] Guide de contribution

### 8. Finalisation et Distribution [ ]
- [ ] Tests de packaging
- [ ] Scripts de build
- [ ] Configuration des releases
- [ ] Documentation du processus de release

## Format de Configuration YAML

Structure proposée pour le fichier de configuration :

```yaml
defaults:
  user: defaultuser
  port: 22
  key_file: ~/.ssh/id_rsa

groups:
  Groupe1:
    defaults:
      user: user1
      proxy_jump: jumphost.example.com
    servers:
      - name: Serveur1
        host: serveur1.example.com
      - name: Serveur2
        host: serveur2.example.com

  Groupe2:
    defaults:
      user: user2
      bastion_host: bastion.example.com
    servers:
      - name: Serveur3
        host: serveur3.example.com
```

## État d'Avancement
🟢 Terminé | 🟡 En cours | 🔴 Non commencé

État actuel : 🔴 Projet non commencé
