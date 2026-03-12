# Roadmap v0.13.0

> Version cible: 0.13.0  
> Theme: Wallix real menu automation (ID auto-selection from config)

---

## Objectif principal

Automatiser la selection de cible dans le menu interactif Wallix a partir de la configuration,
pour ne plus demander a l'utilisateur final de taper manuellement un ID (ex: `0`).

Comportement vise:
- Lire la config YAML pour construire l'identite Wallix attendue (user, account, host, protocol)
- se connecter au bastion qui affiche son menu interactif
- Parser le menu et selectionner automatiquement la bonne ligne via le matching group Wallix
- En cas d'ambiguite ou de mismatch, message clair + strategie de fallback controlee

---

## Epic 1 - Auto-selection du menu Wallix (MVP)

### 1.1 Schema de configuration Wallix (simplifie)

Etendre la section wallix avec les champs essentiels:
- host: string (bastion Wallix, ex: "ssh.in.phm.education.gouv.fr")
- user: string (compte utilisateur bastion, ex: "pcollin")
- account: string (optionnel, defaut: "default")
- protocol: string (optionnel, defaut: "SSH")
- auto_select: bool (defaut: true)
- fail_if_menu_match_error: bool (defaut: true)
- selection_timeout_secs: u64 (defaut: 8)

Au niveau serveur/groupe/env, ajouter un champ simple:
- wallix_group: string (groupe/autorisation Wallix, ex: "PP-ONDE_ces3s-admins")

Conserver la retrocompatibilite:
- Si template est defini, il garde la priorite (mode expert).

**Exemple de configuration YAML simplifie:**
```yaml
defaults:
  wallix:
    host: "ssh.in.phm.education.gouv.fr"
    user: "pcollin"
    account: "default"
    protocol: "SSH"
    auto_select: true
    fail_if_menu_match_error: true

groups:
  - name: "ONDE-BD"
    mode: wallix
    servers:
      - name: "pp-ond-admins-ces3s"
        host: "PP-ONDE-BD"
        wallix_group: "PP-ONDE_ces3s-admins"      # groupe spécifique
      - name: "pp-ond-admins-crtech"
        host: "PP-ONDE-BD"
        wallix_group: "PP-ONDE_crtech-admins"    # autre groupe, même serveur
```

### 1.2 Construction de la chaine SSH attendue

Depuis la config YAML, construire simplement:
- expected_target = `{user}@{account}@{host}:{protocol}`
  Exemple: `pcollin@default@PP-ONDE-BD:SSH`
- expected_group = `{wallix_group}` (la valeur lue directement du yaml)
  Exemple: `PP-ONDE_ces3s-admins`

Resolution des champs (héritage standard: server → env → group → defaults → fallback):
1. user: server.user ou defaults.wallix.user → "root"
2. account: defaults.wallix.account → "default"
3. host: server.host (obligatoire)
4. protocol: defaults.wallix.protocol → "SSH"
5. wallix_group: server.wallix_group (optionnel, peut venir du groupe ou env aussi)

**Cas d'usage courant:** Meme serveur cible (PP-ONDE-BD), plusieurs groupes Wallix:
- expected_target identique: `pcollin@default@PP-ONDE-BD:SSH`
- expected_group differe: `PP-ONDE_ces3s-admins` vs `PP-ONDE_crtech-admins`
- susshi filtre le menu pour trouver la ligne avec (target, group) unique et envoie son ID

### 1.3 Lecture et parse du menu Wallix

Executer la session SSH Wallix dans un PTY dedie (pas en `exec` direct) pour pouvoir:
- lire les lignes du menu (`ID`, `Cible`, `Autorisation`)
- detecter le prompt de selection (`> `)
- parser les entrees et construire une table de candidats

Parcourir la table affichee (exemple reel):
```
| ID | Cible (page 1/1)               | Autorisation
|----|--------------------------------|-----------------------
|  0 | pcollin@default@PP-ONDE-BD:SSH | PP-ONDE_ces3s-admins
|  1 | pcollin@default@PP-ONDE-BD:SSH | PP-ONDE_crtech-admins
Tapez h pour l'aide, ctrl-D pour quitter
 > _
```

Algo de selection (TDD-friendly):
1. Parser chaque ligne du tableau pour extraire (ID, Cible, Autorisation)
2. Filter par `Cible == expected_target` (ex: "pcollin@default@PP-ONDE-BD:SSH")
3. Dans ce sous-ensemble, filter par `Autorisation == expected_group` (ex: "PP-ONDE_ces3s-admins")
4. Si 1 match unique: retourner l'ID
5. Sinon: erreur avec liste des alternatives

**Cas geres:**
- 1 match exact (cible + groupe): envoyer l'ID ✓
- 0 matches: erreur actionnable avec liste des groupes disponibles pour cette cible
- N > 1 matches: erreur ambiguite (ne devrait pas survenir avec une config correcte)
- Cible absente: erreur actionnable (mauvaise config ou droits Wallix insuffisants)

### 1.4 Execution SSH en mode Wallix

Dans la construction des args SSH:
- utiliser le builder Wallix pour -l
- garder bastion host/port comme destination finale
- ne pas changer les modes direct/jump

### 1.5 Strategie de fallback et messages utilisateur

Quand auto_select=true:

**Config incomplets (avant connexion):**
- Erreur: "wallix.host not configured" → impossible de se connecter au bastion
- Erreur: "wallix.user not configured" → fallback root (acceptable)
- Warning: "wallix_group not specified" → tout groupe sera accepté (moins sûr)

**A la connexion (lors du parse menu):**
- Si menu ne s'affiche pas avant timeout: 
  - Si fail_if_menu_match_error=true: abort avec timeout
  - Si fail_if_menu_match_error=false: continuer (laisser l'utilisateur choisir)
  
- Si wallix_group est spécifié et aucun match (cible existe, mais groupe ne correspond pas):
  - Message: "Expected: pcollin@default@PP-ONDE-BD:SSH [group: PP-ONDE_ces3s-admins]"
  - Message: "Found for this target: [PP-ONDE_crtech-admins (ID: 1), ...]"
  - Si fail_if_menu_match_error=true: abort
  - Si fail_if_menu_match_error=false: afficher le menu et laisser l'utilisateur choisir
  
- Si cible absente du menu:
  - Message: "No entry found for target pcollin@default@PP-ONDE-BD:SSH"
  - Message: "Check server config (host, user, account) or user's Wallix policy"
  - Abort (config issue)
  
- Si wallix_group non spécifié et plusieurs groupes pour la cible:
  - Warning: "Multiple groups available for this target, auto-selecting first one"
  - Ou: afficher le menu et laisser l'utilisateur choisir

---

## Epic 2 - Validation et UX config

### 2.1 Validation YAML

Ajouter des warnings de schema pour les nouvelles cles wallix:
- host, user, account, protocol
- auto_select, fail_if_menu_match_error, selection_timeout_secs
- wallix_group (au niveau server/env/group)

Messages de validation au chargement de la config:
- "wallix.host missing" → critical, impossible de se connecter
- "server.host missing in wallix mode" → critical, cible à se connecter manquante
- "wallix_group not specified" → warning, tout groupe sera accepté
- Au diagnostic/connexion: afficher le mapping esperé vs trouvé

### 2.2 Doc utilisateur

Mettre a jour:
- README: section Wallix avec exemple reel de menu et auto-selection
- examples/full_config.yaml: exemple wallix avec authorization et timeout
- CHANGELOG: nouvelle feature et compatibilite

---

## Epic 3 - Diagnostic compatible mode Wallix

### 3.1 Objectif

Rendre la touche de diagnostic utilisable en mode Wallix sans casser l'UX actuelle.

Comportement vise:
- Le diagnostic ne retourne plus "indisponible" en mode Wallix.
- Le diagnostic execute un sous-ensemble adapte (reachable + handshake + auth path) sans tunnel.
- Le resultat indique clairement ce qui est teste en mode Wallix vs direct/jump.

### 3.2 Strategie technique

Ajouter un profil de diagnostic par mode de connexion:
- direct/jump: comportement existant (systeme + fichiersystems + tunnel checks eventuels)
- wallix: checks compatibles bastion only

Checks Wallix minimum:
- resolution de la cible Wallix (template/build final)
- reachability TCP du bastion (host/port)
- verification de la capacite a parser un menu type Wallix (sur fixture de sortie)
- classification des erreurs: config, reseau, auth, policy/authorization

### 3.3 UX et restitution

Dans l'ecran de diagnostic:
- afficher un badge "Wallix profile"
- afficher la liste des checks sautes explicitement (ex: filesystems, tunnels)
- messages actionnables pour corriger la config wallix (wallix_group/account/host)
- afficher le resume de matching: target attendu, groupe attendu, nb candidats trouves

---

## Epic 4 - Tests (TDD)

### 4.1 Tests unitaires config

Dans config:
- merge_bastion conserve les nouveaux champs et priorites
- resolve_server herite correctement account/protocol/wallix_group
- template custom reste prioritaire

### 4.2 Tests unitaires ssh args

Dans ssh/client:
- wallix_builds_expected_target_identity
- wallix_fallback_to_default_account
- wallix_missing_required_fields_returns_error
- wallix_custom_template_still_supported

### 4.3 Tests unitaires parser/selection menu

Dans un module dedie Wallix:
- parse_wallix_menu_extracts_id_target_group
- select_id_by_target_and_group_returns_unique_id
- select_id_returns_error_when_no_match
- select_id_returns_error_when_ambiguous
- parse_handles_french_banner_and_non_menu_lines

### 4.4 Tests unitaires diagnostic

Dans probe/diagnostic:
- wallix_profile_selected_when_mode_wallix
- wallix_skips_unsupported_checks_with_explicit_reason
- wallix_bastion_reachability_failure_is_classified_network
- wallix_auth_failure_is_classified_auth_or_policy
- wallix_menu_match_error_is_classified_configuration_or_policy

### 4.5 Tests integration

Dans tests/ssh_args.rs:
- scenarii Wallix avec config complete
- scenarii incomplets avec fail_if_menu_match_error true/false

Dans tests/wallix_menu.rs:
- fixture du prompt Wallix reel (avec table ID/Cible/Autorisation)
- auto-selection envoie l'ID attendu (ex: 0)
- verification du passage au shell apres "Account successfully checked out"

Dans tests de diagnostic:
- scenario Wallix happy-path (sans menu interactif)
- scenario Wallix with missing authorization (message actionnable)

### 4.6 Tests UI TUI Wallix

Dans tests/wallix_ui.rs:
- wallix_ui_shows_when_auto_select_false
- wallix_ui_shows_when_no_match_and_fail_if_menu_error_false
- wallix_ui_filters_by_target_name
- wallix_ui_filters_by_group_name
- wallix_ui_returns_selected_id_on_confirm
- wallix_ui_cancels_and_aborts_on_escape
- wallix_ui_displays_availability_count_after_filter

Dans app.rs (test integration TUI):
- scenario: user triggers wallix connection, gets UI selector, selects group 0, handoff shell

---

## Epic 5 - Observabilite minimale

Ajouter logs de debug (verbose):
- mode wallix auto-select active/desactive
- champs resolves (sans infos sensibles)
- raison d'un fallback ou d'un abort
- decision de profil diagnostic (direct/jump/wallix) et checks ignores
- details du matching menu: IDs candidats et ID retenu

---

## Epic 6 - UI TUI pour selection manuelle du groupe Wallix

### 6.1 Objectif

Quand auto_select=false OU fail_if_menu_match_error=false et pas de match unique,
afficher une interface TUI interactive pour que l'utilisateur choisisse le groupe Wallix.

Comportement vise:
- A la place du fallback "afficher le menu brut Wallix", presenter une liste interactive TUI
- L'utilisateur peut filtrer/chercher la cible et le groupe voulus
- Confirmation avant d'envoyer l'ID au bastion
- Plus facile a utiliser qu'une table brute

### 6.2 Design UI

Ajouter un ecran modal TUI "Select Wallix Target":
```
┌─ Select Wallix Target ──────────────────────────────────┐
│ Filter: [________________]                              │
│                                                          │
│ ⚡ Available targets:                                   │
│  [✓] pcollin@default@PP-ONDE-BD:SSH                     │
│      → PP-ONDE_ces3s-admins          [Confirm] [Cancel] │
│      → PP-ONDE_crtech-admins                          │
│  [ ] autre_user@default@AUTRE:SSH                       │
│                                                    ↓     │
│ [Arrows to navigate, Enter to select, Esc to cancel]   │
└─────────────────────────────────────────────────────────┘
```

Elements:
- Barre de search/filter rapide sur cible ou groupe
- Liste hierarchique: cibles → groupes sous chaque cible
- Navigation clavier: fleches, Enter pour confirmer, Esc pour annuler
- Affichage du nombre d'options restantes apres filtre

### 6.3 Integration

Declenchement:
- Si auto_select=true ET pas de match unique → abort (actuel)
- Si auto_select=false → toujours afficher la TUI
- Si fail_if_menu_match_error=false ET pas de match unique → afficher la TUI avant d'abort

Confirmation:
- L'utilisateur selectionne une ligne
- Afficher un brief: "Connexion via PP-ONDE_ces3s-admins? [Y/n]"
- Envoyer l'ID au bastion

### 6.4 Cas d'usage

- Utilisateur oublie sa wallix_group dans la config → TUI lui propose les options dispo
- Utilisateur veut switcher entre groupes pour tester → desactive auto_select, choisit via TUI
- Configuration incomplete (wallix_group manquant) + fail_if_menu_match_error=false → TUI guide l'utilisateur

---

## Epic 5 (renomme) - Observabilite minimale

Ajouter logs de debug (verbose):
- mode wallix auto-select active/desactive
- champs resolves (sans infos sensibles)
- raison d'un fallback ou d'un abort
- decision de profil diagnostic (direct/jump/wallix) et checks ignores
- details du matching menu: IDs candidats et ID retenu

---

## Critere d'acceptation v0.13.0

- Une connexion Wallix peut selectionner automatiquement la bonne entree de menu via la config.
- Aucune saisie manuelle de l'ID n'est necessaire quand target+groupe matchent une ligne unique.
- Une UI TUI interactive permet a l'utilisateur de choisir le groupe Wallix en cas de besoin (auto_select=false ou pas de match unique).
- Le diagnostic (`d`) est executable en mode Wallix avec un resultat exploitable.
- Les config existantes restent fonctionnelles sans modification obligatoire.
- Les tests existants restent verts + nouveaux tests Wallix passent.
- cargo fmt, cargo clippy -- -D warnings, cargo test passent.

---

## Hors scope (v0.13.x+)

- Gestion multi-groupes avec priorites/filtres complexes (a la Ansible inventory).
- Probe disque/IO distant complet en mode Wallix si la politique bastion l'interdit.

---

---

## Exemples concrets du flux complet

### Cas 1: Single authorization per user (cas nominal)

**Config YAML:**
```yaml
defaults:
  wallix:
    host: "ssh.in.phm.education.gouv.fr"
    user: "pcollin"

groups:
  - name: "ONDE-BD"
    mode: wallix
    servers:
      - name: "pp-ond-ces3s"
        host: "PP-ONDE-BD"
        wallix_group: "PP-ONDE_ces3s-admins"
```

**Menu Wallix reel:**
```
| ID | Cible                          | Autorisation
|----|--------------------------------|-----------------------
|  0 | pcollin@default@PP-ONDE-BD:SSH | PP-ONDE_ces3s-admins
|  1 | pcollin@default@PP-ONDE-BD:SSH | PP-ONDE_crtech-admins
Tapez h pour l'aide, ctrl-D pour quitter
 > _
```

**Flux susshi:**
1. Build expected_target: `pcollin@default@PP-ONDE-BD:SSH`
2. Build expected_group: `PP-ONDE_ces3s-admins`
3. Parse menu → candidats = [{ID: 0, Cible: "pcollin@default@PP-ONDE-BD:SSH", Autorisation: "PP-ONDE_ces3s-admins"}, {ID: 1, ...}]
4. Filter par cible → [candidat 0, candidat 1]
5. Filter par groupe → [candidat 0]
6. Unique match: envoyer "0\n"
7. Attend "Account successfully checked out" + handoff shell

### Cas 2: Multiple authorizations, cherche l'autre

**Config YAML (variante):**
```yaml
groups:
  - name: "ONDE-BD"
    mode: wallix
    servers:
      - name: "pp-ond-crtech"
        host: "PP-ONDE-BD"
        wallix_group: "PP-ONDE_crtech-admins"  # <-- autre groupe
```

**Menu Wallix identique**, susshi:
1. expected_target: `pcollin@default@PP-ONDE-BD:SSH` (identique)
2. expected_group: `PP-ONDE_crtech-admins` (different!)
3. Filter par cible → [candidat 0, candidat 1]
4. Filter par groupe → [candidat 1]
5. Unique match: envoyer "1\n"

### Cas 3: Authorization mismatch (pas de match)

**Config YAML:**
```yaml
servers:
  - name: "pp-ond-autre"
    host: "PP-ONDE-BD"
    wallix_group: "PP-ONDE_autre-admins"  # n'existe pas dans le menu
```

**Flux susshi:**
1. Parse menu comme cas 1/2
2. Filter par cible → [candidat 0, candidat 1]
3. Filter par groupe → [] (aucun match!)
4. Erreur actionnable:
   ```
   [ERROR] Wallix menu selection failed
   Expected: pcollin@default@PP-ONDE-BD:SSH [group: PP-ONDE_autre-admins]
   Available groups for this target:
   - PP-ONDE_ces3s-admins (ID: 0)
   - PP-ONDE_crtech-admins (ID: 1)
   Hint: update 'wallix_group' in your config.
   ```
5. Si fail_if_menu_match_error=true: abort
6. Si fail_if_menu_match_error=false: afficher le menu et laisser l'utilisateur choisir

### Cas 4: Target absent du menu (droits bastion insuffisants)

**Config YAML:**
```yaml
servers:
  - name: "restricted-server"
    host: "RESTRICTED-BD"  # n'existe pas dans les droits Wallix de l'utilisateur
    wallix_group: "PP-ONDE_ces3s-admins"
```

**Menu Wallix (sans la cible cherchee):**
```
| ID | Cible                          | Autorisation
|----|--------------------------------|-----------------------
|  0 | pcollin@default@PP-ONDE-BD:SSH | PP-ONDE_ces3s-admins
|  1 | pcollin@default@PP-ONDE-BD:SSH | PP-ONDE_crtech-admins
Tapez h pour l'aide, ctrl-D pour quitter
 > _
```

**Flux susshi:**
1. Parse menu
2. Filter par cible → [] (aucune ligne avec "pcollin@default@RESTRICTED-BD:SSH")
3. Erreur actionnable:
   ```
   [ERROR] Wallix: target not found in menu
   Expected: pcollin@default@RESTRICTED-BD:SSH
   Available targets:
   - pcollin@default@PP-ONDE-BD:SSH
   Possible causes:
   - Server hostname (host:) configured incorrectly
   - User has no access to this target in Wallix policy
   ```
4. Abort (ca va fallback si fail_if_menu_match_error=false, sinon abort immediately)

---

## Plan de livraison propose

- Lot A (MVP): schema wallix + parser menu + algo de selection ID + tests unitaires parser
- Lot B: integration SSH PTY + fallback + tests integration wallix_menu
- Lot C: UI TUI selection Wallix + tests UI TUI + integration avec fallback
- Lot D: diagnostic Wallix profile + tests diagnostic
- Lot E: validation + docs + exemple complet + polish logs
