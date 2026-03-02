//! Internationalisation (i18n) — textes de l'interface TUI.
//!
//! Deux langues supportées : Français (`Fr`) et Anglais (`En`).
//! La langue est détectée une seule fois au démarrage via les variables
//! d'environnement `LC_ALL` → `LC_MESSAGES` → `LANG`.
//!
//! ## Gabarits de format
//!
//! Les champs dont la valeur contient `{}` sont des gabarits substituables.
//! Utiliser [`fmt`] ou [`str::replacen`] pour les instancier.

// ─── Types ────────────────────────────────────────────────────────────────────

/// Langue de l'interface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Lang {
    Fr,
    En,
}

/// Ensemble des textes de l'interface TUI.
///
/// Les champs contenant `{}` sont des gabarits à instancier via [`fmt`].
pub struct Strings {
    // ── Fenêtre erreur ──────────────────────────────────────────────────────
    pub error_title: &'static str,
    pub error_dismiss: &'static str,

    // ── Onglets connexion ───────────────────────────────────────────────────
    pub tab_title: &'static str,
    /// Libellé de l'onglet Direct (affiché dans le widget Tabs et pour la détection clic).
    pub tab_direct: &'static str,
    /// Libellé de l'onglet Jump/Rebond.
    pub tab_jump: &'static str,
    /// Libellé de l'onglet Wallix.
    pub tab_wallix: &'static str,

    // ── Toggle verbose ──────────────────────────────────────────────────────
    pub verbose_title: &'static str,
    pub verbose_label: &'static str,

    // ── Barre de recherche ──────────────────────────────────────────────────
    /// Texte affiché dans la barre inactive et vide.
    pub search_idle_hint: &'static str,
    /// Titre du bloc quand la barre est inactive.
    pub search_title_idle: &'static str,
    /// Texte indicatif dans la barre active avant la saisie.
    pub search_placeholder: &'static str,
    /// Titre du bloc actif — `{}` = nombre total de serveurs.
    pub search_title_active: &'static str,
    /// Titre quand aucun résultat — `{}` = terme recherché.
    pub search_no_results: &'static str,
    /// Titre quand tous les serveurs correspondent — `{}` = nombre.
    pub search_all_match: &'static str,
    /// Titre résultats partiels (actif) — `{}` = trouvés, `{}` = total.
    pub search_partial: &'static str,
    /// Titre résultats (non actif) tous affichés — `{}` = nombre.
    pub search_result_all: &'static str,
    /// Titre résultats partiels (non actif) — `{}` = trouvés, `{}` = total, `{}` = terme.
    pub search_result_partial: &'static str,

    // ── Panneaux principaux ─────────────────────────────────────────────────
    pub panel_servers: &'static str,
    pub panel_details: &'static str,
    pub details_placeholder: &'static str,

    // ── Libellés du panneau détails ─────────────────────────────────────────
    pub label_name: &'static str,
    pub label_host: &'static str,
    pub label_port: &'static str,
    pub label_user: &'static str,
    pub label_mode: &'static str,
    pub label_key: &'static str,
    pub label_jump: &'static str,
    pub label_wallix: &'static str,
    pub label_options: &'static str,

    // ── Bloc diagnostic (System) ────────────────────────────────────────────
    pub probe_section: &'static str,
    pub probe_hint: &'static str,
    pub probe_running: &'static str,
    pub probe_kernel: &'static str,
    pub probe_cpu: &'static str,
    /// Nombre de cœurs logiques.
    pub probe_cpu_cores: &'static str,
    /// Nom et version de l'OS.
    pub probe_os: &'static str,
    pub probe_load: &'static str,
    pub probe_ram: &'static str,
    pub probe_disk: &'static str,
    /// Erreur quand le diagnostic est demandé en mode Wallix.
    pub probe_wallix_error: &'static str,
    /// Libellé d'un filesystem supplémentaire présent — `{}` = point de montage.
    pub probe_disk_extra: &'static str,
    /// Ligne complète pour un filesystem absent — `{}` = point de montage.
    pub probe_fs_absent: &'static str,

    // ── Barre de statut ─────────────────────────────────────────────────────
    pub status_normal: &'static str,
    pub status_searching: &'static str,
    pub status_search_active: &'static str,

    // ── Avertissements includes ─────────────────────────────────────────────
    /// `{}` = label, `{}` = chemin, `{}` = erreur
    pub include_warn_load: &'static str,
    /// `{}` = label, `{}` = chemin
    pub include_warn_circular: &'static str,
    /// `{}` = label
    pub include_warn_nested: &'static str,

    // ── Messages de statut (gabarits) ───────────────────────────────────────
    /// `{}` = commande SSH copiée.
    pub copied: &'static str,
    /// `{}` = description de l'erreur.
    pub clipboard_error: &'static str,
    pub clipboard_unavailable: &'static str,
    /// `{}` = description de l'erreur SSH.
    pub ssh_error: &'static str,

    // ── Historique des connexions ────────────────────────────────────────────
    pub last_seen_label: &'static str,
    pub last_seen_never: &'static str,
    /// `{}` = durée formatée (ex. "3 jours")
    pub last_seen_ago: &'static str,
    pub last_seen_just_now: &'static str,

    // ── Rechargement à chaud ─────────────────────────────────────────────────
    /// `{}` = nombre de serveurs
    pub config_reloaded: &'static str,
    pub config_reload_error: &'static str,

    // ── Favoris ──────────────────────────────────────────────────────────────
    pub favorites_title: &'static str,
    pub favorite_added: &'static str,
    pub favorite_removed: &'static str,

    // ── Sort par récence ─────────────────────────────────────────────────────
    pub sort_recent_on: &'static str,
    pub sort_recent_off: &'static str,

    // ── Commande ad-hoc ──────────────────────────────────────────────────────
    pub cmd_prompt: &'static str,
    pub cmd_running: &'static str,
    /// `{}` = exit code
    pub cmd_exit_err: &'static str,

    // ── Validation YAML ──────────────────────────────────────────────────────
    pub validation_title: &'static str,
    /// `{}` = fichier, `{}` = contexte, `{}` = champ
    pub validation_unknown_field: &'static str,

    // ── Tunnels SSH ──────────────────────────────────────────────────────────
    pub tunnel_wallix_unavailable: &'static str,
    /// `{}` = index
    pub tunnel_not_found: &'static str,
    /// `{}` = label, `{}` = port
    pub tunnel_already_active: &'static str,
    /// `{}` = label, `{}` = port
    pub tunnel_started: &'static str,
    /// `{}` = erreur
    pub tunnel_error: &'static str,
    /// `{}` = label, `{}` = port
    pub tunnel_stopped: &'static str,
    /// `{}` = label, `{}` = port, `{}` = raison
    pub tunnel_died: &'static str,
    pub tunnel_deleted: &'static str,
    pub tunnel_updated: &'static str,
    pub tunnel_added: &'static str,
    pub tunnel_overlay_new: &'static str,
    pub tunnel_overlay_hints1: &'static str,
    pub tunnel_overlay_hints2: &'static str,
    /// `{}` = serveur
    pub tunnel_form_edit_title: &'static str,
    /// `{}` = serveur
    pub tunnel_form_new_title: &'static str,
    pub tunnel_form_field_label: &'static str,
    pub tunnel_form_field_local_port: &'static str,
    pub tunnel_form_field_remote_host: &'static str,
    pub tunnel_form_field_remote_port: &'static str,
    pub tunnel_form_hint: &'static str,
    pub tunnel_form_local_port_invalid: &'static str,
    pub tunnel_form_remote_host_empty: &'static str,
    pub tunnel_form_remote_port_invalid: &'static str,
    pub tunnel_badge_label: &'static str,
    /// `{}` = n_actif, `{}` = suffixe pluriel, `{}` = n_cfg, `{}` = suffixe pluriel
    pub tunnel_badge_active: &'static str,
    /// `{}` = n_cfg, `{}` = suffixe pluriel
    pub tunnel_badge_none: &'static str,

    // ── SCP ──────────────────────────────────────────────────────────────────
    pub scp_wallix_unavailable: &'static str,
    pub scp_done_ok: &'static str,
    pub scp_done_err: &'static str,
    /// `{}` = erreur
    pub scp_failed: &'static str,
    pub scp_form_local_required: &'static str,
    pub scp_form_remote_required: &'static str,
    /// `{}` = serveur
    pub scp_direction_title: &'static str,
    pub scp_direction_upload: &'static str,
    pub scp_direction_download: &'static str,
    pub scp_direction_hint: &'static str,
    pub scp_form_field_local: &'static str,
    pub scp_form_field_remote: &'static str,
    pub scp_form_hint: &'static str,
    pub scp_result_title: &'static str,
    /// `{}` = direction.label()
    pub scp_result_success: &'static str,
    /// `{}` = direction.label()
    pub scp_result_errors: &'static str,
    /// `{}` = erreur
    pub scp_result_fail: &'static str,
    pub scp_result_hint: &'static str,
    /// `{}` = direction.label()
    pub scp_in_progress: &'static str,
}

// ─── Français ─────────────────────────────────────────────────────────────────

pub static STRINGS_FR: Strings = Strings {
    error_title: " ⚠  Erreur ",
    error_dismiss: "Appuyez sur Entrée ou Esc pour fermer",

    tab_title: " Mode de Connexion (Tab pour changer) ",
    tab_direct: "Direct [1]",
    tab_jump: "Rebond [2]",
    tab_wallix: "Wallix [3]",

    verbose_title: " Options (v pour basculer) ",
    verbose_label: "Verbose (-v)",

    search_idle_hint: "Appuyez sur / pour rechercher…",
    search_title_idle: " Recherche (/) ",
    search_placeholder: "(nom ou hôte, Échap pour annuler)",
    search_title_active: " 🔍 Recherche nom/hôte ({} serveurs) ",
    search_no_results: " 🔍 Aucun résultat pour '{}' ",
    search_all_match: " 🔍 {} serveurs correspondent ",
    search_partial: " 🔍 {} / {} serveurs ",
    search_result_all: " ✓ {} serveurs affichés ",
    search_result_partial: " ✓ {} / {} correspondent à '{}' ",

    panel_servers: " Serveurs ",
    panel_details: " Détails ",
    details_placeholder: "Sélectionnez un serveur pour voir les détails.",

    label_name: "Nom:    ",
    label_host: "Hôte:   ",
    label_port: "Port:   ",
    label_user: "Util.:  ",
    label_mode: "Mode:   ",
    label_key: "Clé:    ",
    label_jump: "Rebond: ",
    label_wallix: "Wallix:",
    label_options: "Options:",

    probe_section: "─── Système ─────────────────────",
    probe_hint: "  d — diagnostiquer",
    probe_running: "Diagnostic en cours…",
    probe_kernel: "Kernel   ",
    probe_cpu: "CPU      ",
    probe_cpu_cores: "Cœurs    ",
    probe_os: "OS       ",
    probe_load: "Charge   ",
    probe_ram: "RAM",
    probe_disk: "Disk /",
    probe_wallix_error: "Diagnostic non disponible en mode Wallix",
    probe_disk_extra: "Disk {}",
    probe_fs_absent: "⚠  {} — non monté",

    status_normal: "Navigation : ↑/↓ | Ouvrir : Espace/Entrée | Recherche : / | Mode : Tab/1-3 | v : Verbose | y : Copier | d : Probe | f : Favori | F : Vue favs | r : Recharger | x : Cmd | H : Tri | C : Replier tout | q : Quitter",
    status_searching: "Recherche : Tapez pour filtrer… | Échap : Annuler | Ctrl+U : Effacer | Entrée : Valider",
    status_search_active: "Navigation : ↑/↓ | Effacer : Échap | Nouvelle recherche : / | Verbose : v | Entrée : Connecter | q : Quitter",

    include_warn_load: "Impossible de charger '{}' ({}) : {}",
    include_warn_circular: "Dépendance circulaire ignorée : '{}' ({})",
    include_warn_nested: "Les includes imbriqués dans '{}' sont ignorés (v0.7)",

    copied: "Copié : {}",
    clipboard_error: "Erreur presse-papiers : {}",
    clipboard_unavailable: "Presse-papiers indisponible",
    ssh_error: "Erreur SSH : {}",

    last_seen_label: "Dern. conn.: ",
    last_seen_never: "—",
    last_seen_ago: "il y a {}",
    last_seen_just_now: "à l'instant",

    config_reloaded: "Config rechargée ({} serveurs)",
    config_reload_error: "Erreur rechargement config",

    favorites_title: " ⭐ Favoris ",
    favorite_added: "⭐ Ajouté aux favoris",
    favorite_removed: "Favori retiré",

    sort_recent_on: "Tri : récent  [H]",
    sort_recent_off: "Tri : alpha   [H]",

    cmd_prompt: "Commande : ",
    cmd_running: "Exécution…",
    cmd_exit_err: "Erreur (exit {})",

    validation_title: " ⚠  Avertissements de configuration ",
    validation_unknown_field: "{} ({}): champ inconnu « {} »",

    tunnel_wallix_unavailable: "Tunnels SSH non disponibles en mode Wallix",
    tunnel_not_found: "Tunnel #{} introuvable pour ce serveur",
    tunnel_already_active: "Tunnel « {} » déjà actif (port {})",
    tunnel_started: "Tunnel « {} » démarré sur le port {}",
    tunnel_error: "Erreur tunnel : {}",
    tunnel_stopped: "Tunnel « {} » (port {}) arrêté",
    tunnel_died: "Tunnel « {} » (port {}) s'est arrêté : {}",
    tunnel_deleted: "Tunnel supprimé",
    tunnel_updated: "Tunnel mis à jour",
    tunnel_added: "Tunnel ajouté",
    tunnel_overlay_new: "+ (nouveau tunnel)",
    tunnel_overlay_hints1: "  ↑↓ naviguer   Enter démarrer/arrêter   Del supprimer",
    tunnel_overlay_hints2: "  e éditer      a ajouter                q/Esc fermer",
    tunnel_form_edit_title: " Modifier le tunnel — {} ",
    tunnel_form_new_title: " Nouveau tunnel — {} ",
    tunnel_form_field_label: "  Label        : ",
    tunnel_form_field_local_port: "  Port local   : ",
    tunnel_form_field_remote_host: "  Hôte distant : ",
    tunnel_form_field_remote_port: "  Port distant : ",
    tunnel_form_hint: "  Tab champ suivant   Enter valider   Esc annuler",
    tunnel_form_local_port_invalid: "Port local invalide (entier 1\u{2013}65535 attendu)",
    tunnel_form_remote_host_empty: "Hôte distant obligatoire",
    tunnel_form_remote_port_invalid: "Port distant invalide (entier 1\u{2013}65535 attendu)",
    tunnel_badge_label: "Tunnels : ",
    tunnel_badge_active: "  {} actif{} / {} configuré{}",
    tunnel_badge_none: "  {} configuré{}, aucun actif",

    scp_wallix_unavailable: "SCP non disponible en mode Wallix",
    scp_done_ok: "SCP terminé ✔",
    scp_done_err: "SCP terminé avec des erreurs ✗",
    scp_failed: "SCP échoué : {}",
    scp_form_local_required: "Le chemin local est obligatoire",
    scp_form_remote_required: "Le chemin distant est obligatoire",
    scp_direction_title: " Transfert SCP — {} ",
    scp_direction_upload: "(local → serveur)",
    scp_direction_download: "(serveur → local)",
    scp_direction_hint: "  Esc annuler",
    scp_form_field_local: "  Local   : ",
    scp_form_field_remote: "  Distant : ",
    scp_form_hint: "  Tab changer de champ   Enter confirmer   Esc annuler",
    scp_result_title: " Résultat SCP ",
    scp_result_success: "SCP {} terminé avec succès",
    scp_result_errors: "SCP {} terminé avec des erreurs",
    scp_result_fail: "Erreur SCP : {}",
    scp_result_hint: "  Enter / Esc  fermer",
    scp_in_progress: "SCP {} en cours...",
};

// ─── Anglais ──────────────────────────────────────────────────────────────────

pub static STRINGS_EN: Strings = Strings {
    error_title: " ⚠  Error ",
    error_dismiss: "Press Enter or Esc to close",

    tab_title: " Connection Mode (Tab to switch) ",
    tab_direct: "Direct [1]",
    tab_jump: "Jump [2]",
    tab_wallix: "Wallix [3]",

    verbose_title: " Options (v to toggle) ",
    verbose_label: "Verbose (-v)",

    search_idle_hint: "Press / to search...",
    search_title_idle: " Search (press /) ",
    search_placeholder: "(search by name or host, ESC to cancel)",
    search_title_active: " 🔍 Search by name/host ({} servers) ",
    search_no_results: " 🔍 No results for '{}' ",
    search_all_match: " 🔍 All {} servers match ",
    search_partial: " 🔍 {} / {} servers ",
    search_result_all: " ✓ Showing all {} servers ",
    search_result_partial: " ✓ {} / {} servers match '{}' ",

    panel_servers: " Servers ",
    panel_details: " Details ",
    details_placeholder: "Select a server to view details.",

    label_name: "Name:   ",
    label_host: "Host:   ",
    label_port: "Port:   ",
    label_user: "User:   ",
    label_mode: "Mode:   ",
    label_key: "Key:    ",
    label_jump: "Jump:   ",
    label_wallix: "Wallix:",
    label_options: "Options:",

    probe_section: "─── System ──────────────────────",
    probe_hint: "  d — probe",
    probe_running: "Running probe…",
    probe_kernel: "Kernel   ",
    probe_cpu: "CPU      ",
    probe_cpu_cores: "Cores    ",
    probe_os: "OS       ",
    probe_load: "Load     ",
    probe_ram: "RAM",
    probe_disk: "Disk /",
    probe_wallix_error: "Probe unavailable in Wallix mode",
    probe_disk_extra: "Disk {}",
    probe_fs_absent: "⚠  {} — not mounted",

    status_normal: "Navigate: ↑/↓ | Expand: Space/Enter | Search: / | Mode: Tab/1-3 | v: Verbose | y: Copy | d: Probe | f: Fav | F: Favs | r: Reload | x: Cmd | H: Sort | C: Collapse all | q: Quit",
    status_searching: "Search Mode: Type to filter | ESC: Cancel | Ctrl+U: Clear | Enter: Apply",
    status_search_active: "Navigate: ↑/↓ | Clear: ESC | New search: / | Verbose: v | Enter: Connect | q: Quit",

    include_warn_load: "Failed to load '{}' ({}) : {}",
    include_warn_circular: "Circular dependency ignored: '{}' ({})",
    include_warn_nested: "Nested includes in '{}' are ignored (v0.7)",

    copied: "Copied: {}",
    clipboard_error: "Clipboard error: {}",
    clipboard_unavailable: "Clipboard unavailable",
    ssh_error: "SSH error: {}",

    last_seen_label: "Last conn.:  ",
    last_seen_never: "—",
    last_seen_ago: "{} ago",
    last_seen_just_now: "just now",

    config_reloaded: "Config reloaded ({} servers)",
    config_reload_error: "Config reload error",

    favorites_title: " ⭐ Favorites ",
    favorite_added: "⭐ Added to favorites",
    favorite_removed: "Removed from favorites",

    sort_recent_on: "Sort: recent [H]",
    sort_recent_off: "Sort: alpha  [H]",

    cmd_prompt: "Command: ",
    cmd_running: "Running…",
    cmd_exit_err: "Error (exit {})",

    validation_title: " ⚠  Configuration warnings ",
    validation_unknown_field: "{} ({}): unknown field \"{}\"",

    tunnel_wallix_unavailable: "SSH tunnels unavailable in Wallix mode",
    tunnel_not_found: "Tunnel #{} not found for this server",
    tunnel_already_active: "Tunnel '{}' already active (port {})",
    tunnel_started: "Tunnel '{}' started on port {}",
    tunnel_error: "Tunnel error: {}",
    tunnel_stopped: "Tunnel '{}' (port {}) stopped",
    tunnel_died: "Tunnel '{}' (port {}) died: {}",
    tunnel_deleted: "Tunnel deleted",
    tunnel_updated: "Tunnel updated",
    tunnel_added: "Tunnel added",
    tunnel_overlay_new: "+ (new tunnel)",
    tunnel_overlay_hints1: "  ↑↓ navigate   Enter start/stop   Del delete",
    tunnel_overlay_hints2: "  e edit        a add              q/Esc close",
    tunnel_form_edit_title: " Edit tunnel — {} ",
    tunnel_form_new_title: " New tunnel — {} ",
    tunnel_form_field_label: "  Label       : ",
    tunnel_form_field_local_port: "  Local port  : ",
    tunnel_form_field_remote_host: "  Remote host : ",
    tunnel_form_field_remote_port: "  Remote port : ",
    tunnel_form_hint: "  Tab next field   Enter validate   Esc cancel",
    tunnel_form_local_port_invalid: "Invalid local port (1\u{2013}65535 expected)",
    tunnel_form_remote_host_empty: "Remote host required",
    tunnel_form_remote_port_invalid: "Invalid remote port (1\u{2013}65535 expected)",
    tunnel_badge_label: "Tunnels: ",
    tunnel_badge_active: "  {} active{} / {} configured{}",
    tunnel_badge_none: "  {} configured{}, none active",

    scp_wallix_unavailable: "SCP unavailable in Wallix mode",
    scp_done_ok: "SCP complete ✔",
    scp_done_err: "SCP completed with errors ✗",
    scp_failed: "SCP failed: {}",
    scp_form_local_required: "Local path required",
    scp_form_remote_required: "Remote path required",
    scp_direction_title: " SCP Transfer — {} ",
    scp_direction_upload: "(local → server)",
    scp_direction_download: "(server → local)",
    scp_direction_hint: "  Esc cancel",
    scp_form_field_local: "  Local  : ",
    scp_form_field_remote: "  Remote : ",
    scp_form_hint: "  Tab switch field   Enter confirm   Esc cancel",
    scp_result_title: " SCP Result ",
    scp_result_success: "SCP {} completed successfully",
    scp_result_errors: "SCP {} completed with errors",
    scp_result_fail: "SCP error: {}",
    scp_result_hint: "  Enter / Esc  close",
    scp_in_progress: "SCP {} in progress...",
};

// ─── API publique ─────────────────────────────────────────────────────────────

/// Détecte la langue depuis `LC_ALL` → `LC_MESSAGES` → `LANG`.
/// Retourne [`Lang::Fr`] si la valeur commence par `"fr"`, [`Lang::En`] sinon.
pub fn detect_lang() -> Lang {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .find_map(|var| std::env::var(var).ok())
        .map(|val| {
            if val.starts_with("fr") {
                Lang::Fr
            } else {
                Lang::En
            }
        })
        .unwrap_or(Lang::En)
}

/// Retourne la référence statique vers le jeu de chaînes correspondant à `lang`.
pub fn get_strings(lang: Lang) -> &'static Strings {
    match lang {
        Lang::Fr => &STRINGS_FR,
        Lang::En => &STRINGS_EN,
    }
}

/// Substitue les occurrences de `{}` dans `template` par les valeurs de `args`
/// dans l'ordre (première occurrence d'abord).
///
/// # Exemple
/// ```
/// use susshi::i18n::fmt;
/// assert_eq!(fmt("Hello {}!", &["world"]), "Hello world!");
/// ```
pub fn fmt(template: &'static str, args: &[&str]) -> String {
    let mut result = template.to_string();
    for arg in args {
        result = result.replacen("{}", arg, 1);
    }
    result
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    /// Exécute `f` après avoir positionné les variables d'env, puis les restaure.
    /// Utilise un mutex pour éviter la concurrence entre tests.
    fn with_env<F: FnOnce()>(vars: &[(&str, Option<&str>)], f: F) {
        use std::sync::Mutex;
        static ENV_LOCK: Mutex<()> = Mutex::new(());
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // On retire d'abord toutes les variables pertinentes pour éviter les
        // interférences avec d'éventuelles valeurs déjà définies dans l'env de test.
        let saved: Vec<(&str, Option<String>)> =
            vars.iter().map(|(k, _)| (*k, env::var(k).ok())).collect();

        for (k, v) in vars {
            match v {
                Some(val) => unsafe { std::env::set_var(k, val) },
                None => unsafe { std::env::remove_var(k) },
            }
        }

        f();

        // Restauration
        for (k, saved_v) in &saved {
            match saved_v {
                Some(v) => unsafe { std::env::set_var(k, v) },
                None => unsafe { std::env::remove_var(k) },
            }
        }
    }

    #[test]
    fn detect_lang_fr() {
        with_env(
            &[
                ("LC_ALL", None),
                ("LC_MESSAGES", None),
                ("LANG", Some("fr_FR.UTF-8")),
            ],
            || {
                assert_eq!(detect_lang(), Lang::Fr);
            },
        );
    }

    #[test]
    fn detect_lang_en() {
        with_env(
            &[
                ("LC_ALL", None),
                ("LC_MESSAGES", None),
                ("LANG", Some("en_US.UTF-8")),
            ],
            || {
                assert_eq!(detect_lang(), Lang::En);
            },
        );
    }

    #[test]
    fn detect_lang_no_env() {
        with_env(
            &[("LC_ALL", None), ("LC_MESSAGES", None), ("LANG", None)],
            || {
                assert_eq!(detect_lang(), Lang::En);
            },
        );
    }

    #[test]
    fn detect_lang_lc_all_takes_priority() {
        with_env(
            &[
                ("LC_ALL", Some("fr_FR.UTF-8")),
                ("LC_MESSAGES", Some("en_US.UTF-8")),
                ("LANG", Some("en_US.UTF-8")),
            ],
            || {
                assert_eq!(detect_lang(), Lang::Fr);
            },
        );
    }

    #[test]
    fn fr_and_en_differ_on_key_strings() {
        let fr = get_strings(Lang::Fr);
        let en = get_strings(Lang::En);

        assert_ne!(fr.error_title, en.error_title);
        assert_ne!(fr.panel_servers, en.panel_servers);
        assert_ne!(fr.status_normal, en.status_normal);
    }

    #[test]
    fn fmt_single_arg() {
        assert_eq!(
            fmt("Copié : {}", &["ssh root@host"]),
            "Copié : ssh root@host"
        );
    }

    #[test]
    fn fmt_two_args() {
        assert_eq!(fmt("{} / {} serveurs", &["3", "10"]), "3 / 10 serveurs");
    }

    #[test]
    fn fmt_three_args() {
        assert_eq!(
            fmt("{} / {} correspondent à '{}'", &["2", "5", "web"]),
            "2 / 5 correspondent à 'web'"
        );
    }
}
