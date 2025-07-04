# Cahier des charges Sushi

## Sushi
Gestionnaire de connexion ssh en golang
## Objectif
Créer un outil de gestion des connexions ssh en golang, permettant de se connecter à des serveurs distants via un rebond ssh ou un bastion, avec une interface utilisateur interactive.
## Fonctionnalités
- Menu de sélection interactif du serveur auquel se connecter
- Liste des serveurs dans un fichier de configuration yaml structuré pour avoir le minimum de saisie en doublon (on ajoutera uniquement le nom du serveur et le groupe auquel il appartient sans saisir plusieurs fois l'utilisateur, le port, etc.)
- Arborescence des connexions dans le menu cli
```
Groupe1
    Serveur1
    Serveur2
Groupe2
    Serveur3
    Serveur4
Groupe3
    Serveur5
```
- Deux modes de connexion:
  - Connexion via un rebond ssh et clef publique (exemple de configuration ssh)
      ```
    Host *.phm.education.gouv.fr 
    ProxyJump outils-crt.in.ac-dijon.fr
    User pcollin
    Port 22
    PubkeyAcceptedKeyTypes +ssh-rsa
    IdentityFile ~/.ssh/id_rsa
    ```
  - Connexion via un bastion (exemple de configuration ssh)
    - Connexion via un bastion
    ```
    Host *.phm.education.gouv.fr
    Hostname ssh.in.phm.education.gouv.fr
    User pcollin@%n:SSH:pcollin
    ForwardAgent yes
    ```