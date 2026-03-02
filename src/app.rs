use crate::config::{
    Config, ConfigEntry, ConfigError, ConnectionMode, IncludeWarning, ResolvedServer, ThemeVariant,
    ValidationWarning,
};
use crate::probe::{ProbeResult, ProbeState};
use crate::state;
use crate::ui::theme::{Theme, get_theme};
use ratatui::widgets::ListState;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Instant;

/// Mode courant de l'application.
#[derive(Debug, Default, PartialEq)]
pub enum AppMode {
    #[default]
    Normal,
    /// Affiche un panneau d'erreur bloquant jusqu'à la confirmation.
    Error(String),
}

/// État d'une commande SSH ad-hoc (touche `x`).
#[derive(Debug, Clone, Default)]
pub enum CmdState {
    #[default]
    Idle,
    /// L'utilisateur saisit la commande (buffer).
    Prompting(String),
    /// La commande est en cours d'exécution.
    Running(String),
    /// La commande s'est terminée avec son output et code de sortie.
    Done {
        cmd: String,
        output: String,
        exit_ok: bool,
    },
    /// Erreur de lancement.
    Error(String),
}

#[derive(Debug, Clone)]
pub enum ConfigItem {
    /// En-tête de namespace (fichier inclus).
    Namespace(String),
    /// En-tête de groupe — `(name, ns_label)`, ns_label="" si niveau racine.
    Group(String, String),
    /// En-tête d'environnement — `(group_name, env_name, ns_label)`.
    Environment(String, String, String),
    Server(Box<ResolvedServer>),
}

pub struct App {
    pub config: Config,
    pub resolved_servers: Vec<ResolvedServer>,

    pub selected_index: usize,
    pub list_state: ListState,
    pub expanded_items: HashSet<String>,

    pub search_query: String,
    pub is_searching: bool,

    pub connection_mode: ConnectionMode,
    pub verbose_mode: bool,

    /// Mode courant (Normal ou Error).
    pub app_mode: AppMode,

    /// Thème Catppuccin actif (résolu à l'initialisation depuis la config).
    pub theme: &'static Theme,

    /// Message temporaire affiché dans la barre de statut (texte, timestamp)
    pub status_message: Option<(String, Instant)>,

    /// Cache de la liste visible — recalculé seulement quand `items_dirty` est vrai.
    cached_items: Vec<ConfigItem>,
    pub items_dirty: bool,

    /// Avertissements non-bloquants collectés lors du chargement des includes.
    pub warnings: Vec<IncludeWarning>,

    /// Instance du presse-papiers gardée vivante pour éviter le drop prématuré
    /// (arboard affiche un warning si l'objet est détruit trop vite après set_text).
    pub clipboard: Option<arboard::Clipboard>,

    /// État du diagnostic SSH lancé avec `d`.
    pub probe_state: ProbeState,
    /// Récepteur du thread de diagnostic (présent seulement quand Running).
    pub probe_rx: Option<mpsc::Receiver<Result<ProbeResult, String>>>,

    /// Jeu de chaînes localisées détecté au démarrage.
    pub lang: &'static crate::i18n::Strings,

    /// Chemin du fichier de configuration principal (pour le rechargement).
    pub config_path: PathBuf,

    /// Si true, seuls les favoris sont affichés dans la liste.
    pub favorites_only: bool,

    /// Si true, la liste est triée par dernière connexion (mode plat).
    pub sort_by_recent: bool,

    /// Timestamps UNIX de dernière connexion, indexés par clé de serveur.
    pub last_seen: HashMap<String, u64>,

    /// Ensemble des clés de serveurs marqués comme favoris.
    pub favorites: HashSet<String>,

    /// État de la commande ad-hoc en cours (touche `x`).
    pub cmd_state: CmdState,

    /// Récepteur du thread de commande ad-hoc.
    pub cmd_rx: Option<mpsc::Receiver<(String, bool)>>,

    /// Avertissements de validation YAML (champs inconnus).
    pub validation_warnings: Vec<ValidationWarning>,
}

impl App {
    pub fn new(
        config: Config,
        warnings: Vec<IncludeWarning>,
        config_path: PathBuf,
        validation_warnings: Vec<ValidationWarning>,
    ) -> Result<Self, ConfigError> {
        let resolved = config.resolve()?;

        // Résout le thème avant de déplacer config dans le struct
        let theme_variant = config
            .defaults
            .as_ref()
            .and_then(|d| d.theme)
            .unwrap_or(ThemeVariant::Mocha);

        let mut app = Self {
            config,
            resolved_servers: resolved,
            selected_index: 0,
            list_state: ListState::default(),
            expanded_items: HashSet::new(),
            search_query: String::new(),
            is_searching: false,
            connection_mode: ConnectionMode::Direct,
            verbose_mode: false,
            app_mode: AppMode::Normal,
            theme: get_theme(theme_variant),
            status_message: None,
            cached_items: Vec::new(),
            items_dirty: true,
            clipboard: arboard::Clipboard::new().ok(),
            probe_state: ProbeState::Idle,
            probe_rx: None,
            lang: crate::i18n::get_strings(crate::i18n::detect_lang()),
            warnings,
            config_path,
            favorites_only: false,
            sort_by_recent: false,
            last_seen: HashMap::new(),
            favorites: HashSet::new(),
            cmd_state: CmdState::Idle,
            cmd_rx: None,
            validation_warnings,
        };

        app.list_state.select(Some(0));

        // Restaure l'état d'expansion persistant
        let saved = state::load_state();
        app.expanded_items = saved.expanded_items;
        app.last_seen = saved.last_seen;
        app.favorites = saved.favorites;
        app.sort_by_recent = saved.sort_by_recent;
        app.items_dirty = true;

        app.update_mode_from_selection();

        // Affiche les avertissements d'includes comme erreur non-bloquante
        if !app.warnings.is_empty() {
            let lines: Vec<String> = app
                .warnings
                .iter()
                .map(|w| match w {
                    crate::config::IncludeWarning::LoadError { label, path, error } => app
                        .lang
                        .include_warn_load
                        .replacen("{}", label, 1)
                        .replacen("{}", path, 1)
                        .replacen("{}", error, 1),
                    crate::config::IncludeWarning::Circular { label, path } => app
                        .lang
                        .include_warn_circular
                        .replacen("{}", label, 1)
                        .replacen("{}", path, 1),
                })
                .collect();
            app.app_mode = AppMode::Error(lines.join("\n"));
        }

        Ok(app)
    }

    /// Retourne l'état persistable de l'application (pour la sauvegarde).
    pub fn to_app_state(&self) -> crate::state::AppState {
        crate::state::AppState {
            expanded_items: self.expanded_items.clone(),
            last_seen: self.last_seen.clone(),
            favorites: self.favorites.clone(),
            sort_by_recent: self.sort_by_recent,
        }
    }

    /// Affiche un message temporaire dans la barre de statut.
    pub fn set_status_message(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), Instant::now()));
    }

    /// Passe en mode erreur avec le message donné.
    pub fn set_error(&mut self, msg: impl Into<String>) {
        self.app_mode = AppMode::Error(msg.into());
    }

    /// Revient au mode normal (ferme le panneau d'erreur).
    pub fn clear_error(&mut self) {
        self.app_mode = AppMode::Normal;
    }

    /// Invalide le cache de la liste visible (à appeler après toute modification
    /// de `search_query`, `expanded_items` ou `resolved_servers`).
    pub fn invalidate_cache(&mut self) {
        self.items_dirty = true;
    }

    pub fn toggle_expansion(&mut self) {
        let items = self.get_visible_items();
        if let Some(item) = items.get(self.selected_index) {
            let id = match item {
                ConfigItem::Namespace(label) => format!("NS:{}", label),
                ConfigItem::Group(name, ns) => {
                    if ns.is_empty() {
                        format!("Group:{}", name)
                    } else {
                        format!("NS:{}:Group:{}", ns, name)
                    }
                }
                ConfigItem::Environment(g, e, ns) => {
                    if ns.is_empty() {
                        format!("Env:{}:{}", g, e)
                    } else {
                        format!("NS:{}:Env:{}:{}", ns, g, e)
                    }
                }
                ConfigItem::Server(_) => return,
            };
            if self.expanded_items.contains(&id) {
                self.expanded_items.remove(&id);
            } else {
                self.expanded_items.insert(id);
            }
        }
        self.items_dirty = true; // l'état d'expansion a changé
    }

    /// Replie tous les groupes, namespaces et environnements développés.
    pub fn collapse_all(&mut self) {
        self.expanded_items.clear();
        self.selected_index = 0;
        self.items_dirty = true;
    }

    pub fn get_visible_items(&mut self) -> Vec<ConfigItem> {
        if self.items_dirty {
            self.cached_items = self.build_visible_items();
            self.items_dirty = false;
        }
        self.cached_items.clone()
    }

    fn build_visible_items(&self) -> Vec<ConfigItem> {
        // Mode "tri par récent" : liste plate de tous les serveurs triés par last_seen
        if self.sort_by_recent {
            return self.build_recent_items();
        }

        let mut items = Vec::new();
        let searching = !self.search_query.is_empty();

        for entry in &self.config.groups {
            match entry {
                ConfigEntry::Namespace(ns) => {
                    // En mode favorites_only, on n'affiche le namespace que s'il a des favoris
                    if self.favorites_only
                        && !self.namespace_has_visible_servers(&ns.entries, &ns.label)
                    {
                        continue;
                    }
                    items.push(ConfigItem::Namespace(ns.label.clone()));
                    let ns_id = format!("NS:{}", ns.label);
                    let ns_expanded = self.expanded_items.contains(&ns_id) || searching;
                    if ns_expanded {
                        self.push_entries(&ns.entries, &ns.label, &mut items);
                    }
                }
                ConfigEntry::Server(s_conf) => {
                    if searching && !self.matches_search(&s_conf.name, &s_conf.host) {
                        continue;
                    }
                    if let Some(resolved) = self.resolved_servers.iter().find(|rs| {
                        rs.name == s_conf.name
                            && rs.group_name.is_empty()
                            && rs.env_name.is_empty()
                            && rs.namespace.is_empty()
                    }) {
                        if self.favorites_only
                            && !self.favorites.contains(&Self::server_key(resolved))
                        {
                            continue;
                        }
                        items.push(ConfigItem::Server(Box::new(resolved.clone())));
                    }
                }
                ConfigEntry::Group(group) => {
                    if self.favorites_only && !self.group_has_visible_servers(group, "") {
                        continue;
                    }
                    items.push(ConfigItem::Group(group.name.clone(), String::new()));
                    let group_id = format!("Group:{}", group.name);
                    let group_expanded = self.expanded_items.contains(&group_id) || searching;

                    if group_expanded {
                        self.push_group_children(group, "", &mut items);
                    }
                }
            }
        }
        items
    }

    /// Construit une liste plate triée par dernière connexion (mode H actif).
    fn build_recent_items(&self) -> Vec<ConfigItem> {
        let mut servers: Vec<ConfigItem> = self
            .resolved_servers
            .iter()
            .filter(|rs| {
                if self.favorites_only && !self.favorites.contains(&Self::server_key(rs)) {
                    return false;
                }
                if !self.search_query.is_empty() && !self.matches_search(&rs.name, &rs.host) {
                    return false;
                }
                true
            })
            .cloned()
            .map(|s| ConfigItem::Server(Box::new(s)))
            .collect();

        servers.sort_by(|a, b| {
            let ts_a = if let ConfigItem::Server(s) = a {
                self.last_seen
                    .get(&Self::server_key(s))
                    .copied()
                    .unwrap_or(0)
            } else {
                0
            };
            let ts_b = if let ConfigItem::Server(s) = b {
                self.last_seen
                    .get(&Self::server_key(s))
                    .copied()
                    .unwrap_or(0)
            } else {
                0
            };
            ts_b.cmp(&ts_a) // décroissant : le plus récent en premier
        });
        servers
    }

    /// Indique si un namespace contient des serveurs favoris visibles.
    fn namespace_has_visible_servers(&self, entries: &[ConfigEntry], ns: &str) -> bool {
        for entry in entries {
            match entry {
                ConfigEntry::Group(g) => {
                    if self.group_has_visible_servers(g, ns) {
                        return true;
                    }
                }
                ConfigEntry::Server(s) => {
                    if let Some(resolved) = self
                        .resolved_servers
                        .iter()
                        .find(|rs| rs.name == s.name && rs.namespace == ns)
                        && self.favorites.contains(&Self::server_key(resolved))
                    {
                        return true;
                    }
                }
                ConfigEntry::Namespace(_) => {}
            }
        }
        false
    }

    /// Indique si un groupe contient des serveurs favoris visibles.
    fn group_has_visible_servers(&self, group: &crate::config::Group, ns: &str) -> bool {
        if let Some(envs) = &group.environments {
            for env in envs {
                for s in &env.servers {
                    if let Some(resolved) = self.resolved_servers.iter().find(|rs| {
                        rs.name == s.name
                            && rs.group_name == group.name
                            && rs.env_name == env.name
                            && rs.namespace == ns
                    }) && self.favorites.contains(&Self::server_key(resolved))
                    {
                        return true;
                    }
                }
            }
        }
        if let Some(servers) = &group.servers {
            for s in servers {
                if let Some(resolved) = self.resolved_servers.iter().find(|rs| {
                    rs.name == s.name
                        && rs.group_name == group.name
                        && rs.env_name.is_empty()
                        && rs.namespace == ns
                }) && self.favorites.contains(&Self::server_key(resolved))
                {
                    return true;
                }
            }
        }
        false
    }

    /// Itère les entrées d'un namespace et les pousse dans `items`.
    fn push_entries(&self, entries: &[ConfigEntry], ns: &str, items: &mut Vec<ConfigItem>) {
        let searching = !self.search_query.is_empty();

        for entry in entries {
            match entry {
                ConfigEntry::Group(group) => {
                    items.push(ConfigItem::Group(group.name.clone(), ns.to_string()));
                    let group_id = format!("NS:{}:Group:{}", ns, group.name);
                    let group_expanded = self.expanded_items.contains(&group_id) || searching;
                    if group_expanded {
                        self.push_group_children_ns(group, ns, items);
                    }
                }
                ConfigEntry::Server(s_conf) => {
                    if searching && !self.matches_search(&s_conf.name, &s_conf.host) {
                        continue;
                    }
                    if let Some(resolved) = self.resolved_servers.iter().find(|rs| {
                        rs.name == s_conf.name
                            && rs.group_name.is_empty()
                            && rs.env_name.is_empty()
                            && rs.namespace == ns
                    }) {
                        items.push(ConfigItem::Server(Box::new(resolved.clone())));
                    }
                }
                ConfigEntry::Namespace(_) => {} // imbriqué — ignoré
            }
        }
    }

    /// Enfants d'un groupe racine (pas de namespace).
    fn push_group_children(
        &self,
        group: &crate::config::Group,
        _ns: &str,
        items: &mut Vec<ConfigItem>,
    ) {
        let searching = !self.search_query.is_empty();

        if let Some(envs) = &group.environments {
            for env in envs {
                // En mode favoris, ignorer l'env s'il n'a aucun favori visible
                if self.favorites_only {
                    let has_fav = env.servers.iter().any(|s| {
                        self.resolved_servers.iter().any(|rs| {
                            rs.name == s.name
                                && rs.env_name == env.name
                                && rs.group_name == group.name
                                && rs.namespace.is_empty()
                                && self.favorites.contains(&Self::server_key(rs))
                        })
                    });
                    if !has_fav {
                        continue;
                    }
                }
                items.push(ConfigItem::Environment(
                    group.name.clone(),
                    env.name.clone(),
                    String::new(),
                ));
                let env_id = format!("Env:{}:{}", group.name, env.name);
                let env_expanded = self.expanded_items.contains(&env_id) || searching;
                if env_expanded {
                    for server in &env.servers {
                        if searching && !self.matches_search(&server.name, &server.host) {
                            continue;
                        }
                        if let Some(resolved) = self.resolved_servers.iter().find(|rs| {
                            rs.name == server.name
                                && rs.env_name == env.name
                                && rs.group_name == group.name
                                && rs.namespace.is_empty()
                        }) {
                            if self.favorites_only
                                && !self.favorites.contains(&Self::server_key(resolved))
                            {
                                continue;
                            }
                            items.push(ConfigItem::Server(Box::new(resolved.clone())));
                        }
                    }
                }
            }
        }

        if let Some(servers) = &group.servers {
            for server in servers {
                if searching && !self.matches_search(&server.name, &server.host) {
                    continue;
                }
                if let Some(resolved) = self.resolved_servers.iter().find(|rs| {
                    rs.name == server.name
                        && rs.env_name.is_empty()
                        && rs.group_name == group.name
                        && rs.namespace.is_empty()
                }) {
                    if self.favorites_only && !self.favorites.contains(&Self::server_key(resolved))
                    {
                        continue;
                    }
                    items.push(ConfigItem::Server(Box::new(resolved.clone())));
                }
            }
        }
    }

    /// Enfants d'un groupe sous namespace.
    fn push_group_children_ns(
        &self,
        group: &crate::config::Group,
        ns: &str,
        items: &mut Vec<ConfigItem>,
    ) {
        let searching = !self.search_query.is_empty();

        if let Some(envs) = &group.environments {
            for env in envs {
                if self.favorites_only {
                    let has_fav = env.servers.iter().any(|s| {
                        self.resolved_servers.iter().any(|rs| {
                            rs.name == s.name
                                && rs.env_name == env.name
                                && rs.group_name == group.name
                                && rs.namespace == ns
                                && self.favorites.contains(&Self::server_key(rs))
                        })
                    });
                    if !has_fav {
                        continue;
                    }
                }
                items.push(ConfigItem::Environment(
                    group.name.clone(),
                    env.name.clone(),
                    ns.to_string(),
                ));
                let env_id = format!("NS:{}:Env:{}:{}", ns, group.name, env.name);
                let env_expanded = self.expanded_items.contains(&env_id) || searching;
                if env_expanded {
                    for server in &env.servers {
                        if searching && !self.matches_search(&server.name, &server.host) {
                            continue;
                        }
                        if let Some(resolved) = self.resolved_servers.iter().find(|rs| {
                            rs.name == server.name
                                && rs.env_name == env.name
                                && rs.group_name == group.name
                                && rs.namespace == ns
                        }) {
                            if self.favorites_only
                                && !self.favorites.contains(&Self::server_key(resolved))
                            {
                                continue;
                            }
                            items.push(ConfigItem::Server(Box::new(resolved.clone())));
                        }
                    }
                }
            }
        }

        if let Some(servers) = &group.servers {
            for server in servers {
                if searching && !self.matches_search(&server.name, &server.host) {
                    continue;
                }
                if let Some(resolved) = self.resolved_servers.iter().find(|rs| {
                    rs.name == server.name
                        && rs.env_name.is_empty()
                        && rs.group_name == group.name
                        && rs.namespace == ns
                }) {
                    if self.favorites_only && !self.favorites.contains(&Self::server_key(resolved))
                    {
                        continue;
                    }
                    items.push(ConfigItem::Server(Box::new(resolved.clone())));
                }
            }
        }
    }

    fn matches_search(&self, name: &str, host: &str) -> bool {
        let query = self.search_query.to_lowercase();
        name.to_lowercase().contains(&query) || host.to_lowercase().contains(&query)
    }

    pub fn next(&mut self) {
        let count = self.get_visible_items().len();
        if count > 0 {
            self.selected_index = (self.selected_index + 1) % count;
            self.list_state.select(Some(self.selected_index));
            self.update_mode_from_selection();
        }
    }

    pub fn previous(&mut self) {
        let count = self.get_visible_items().len();
        if count > 0 {
            if self.selected_index == 0 {
                self.selected_index = count - 1;
            } else {
                self.selected_index -= 1;
            }
            self.list_state.select(Some(self.selected_index));
            self.update_mode_from_selection();
        }
    }

    pub fn select(&mut self, index: usize) {
        let count = self.get_visible_items().len();
        if count > 0 && index < count {
            let changed = self.selected_index != index;
            self.selected_index = index;
            self.list_state.select(Some(self.selected_index));
            if changed {
                self.update_mode_from_selection();
            }
        }
    }

    fn update_mode_from_selection(&mut self) {
        let items = self.get_visible_items();
        if let Some(ConfigItem::Server(server)) = items.get(self.selected_index) {
            self.connection_mode = server.default_mode;
        }
        // Réinitialise le diagnostic quand on change de serveur
        self.probe_state = ProbeState::Idle;
        self.probe_rx = None;
    }

    // ─── Identité des serveurs ─────────────────────────────────────────────────

    /// Calcule la clé unique d'un serveur (stable, indépendante de l'ordre de config).
    pub fn server_key(server: &ResolvedServer) -> String {
        let mut key = String::new();
        if !server.namespace.is_empty() {
            key.push_str("NS:");
            key.push_str(&server.namespace);
            key.push(':');
        }
        if !server.group_name.is_empty() {
            key.push_str("Group:");
            key.push_str(&server.group_name);
            key.push(':');
        }
        if !server.env_name.is_empty() {
            key.push_str("Env:");
            key.push_str(&server.env_name);
            key.push(':');
        }
        key.push_str("Server:");
        key.push_str(&server.name);
        key
    }

    /// Retourne le serveur actuellement sélectionné (s'il y en a un).
    pub fn selected_server(&mut self) -> Option<ResolvedServer> {
        let items = self.get_visible_items();
        if let Some(ConfigItem::Server(s)) = items.get(self.selected_index) {
            Some(*s.clone())
        } else {
            None
        }
    }

    // ─── Favoris ──────────────────────────────────────────────────────────────

    /// Indique si le serveur sélectionné est un favori.
    pub fn is_selected_favorite(&mut self) -> bool {
        if let Some(s) = self.selected_server() {
            self.favorites.contains(&Self::server_key(&s))
        } else {
            false
        }
    }

    /// Bascule le statut favori du serveur sélectionné.
    pub fn toggle_favorite(&mut self) {
        if let Some(server) = self.selected_server() {
            let key = Self::server_key(&server);
            if self.favorites.contains(&key) {
                self.favorites.remove(&key);
                self.set_status_message(self.lang.favorite_removed.replacen("{}", &server.name, 1));
            } else {
                self.favorites.insert(key);
                self.set_status_message(self.lang.favorite_added.replacen("{}", &server.name, 1));
            }
        }
    }

    /// Bascule le mode "afficher seulement les favoris".
    pub fn toggle_favorites_view(&mut self) {
        self.favorites_only = !self.favorites_only;
        self.items_dirty = true;
        self.selected_index = 0;
        self.list_state.select(Some(0));
        let msg = if self.favorites_only {
            self.lang.favorites_title
        } else {
            self.lang.status_normal
        };
        self.set_status_message(msg);
    }

    // ─── Historique / last_seen ────────────────────────────────────────────────

    /// Enregistre une connexion pour le serveur sélectionné (timestamp UNIX).
    pub fn record_connection(&mut self, server: &ResolvedServer) {
        let key = Self::server_key(server);
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.last_seen.insert(key, ts);
    }

    /// Retourne le timestamp UNIX de dernière connexion pour un serveur.
    pub fn last_seen_for(&self, server: &ResolvedServer) -> Option<u64> {
        self.last_seen.get(&Self::server_key(server)).copied()
    }

    // ─── Rechargement à chaud ─────────────────────────────────────────────────

    /// Recharge la configuration depuis le disque sans quitter.
    pub fn reload(&mut self) -> Result<(), ConfigError> {
        let mut stack = std::collections::HashSet::new();
        let (new_config, new_warnings, new_val_warnings) =
            Config::load_merged(&self.config_path, &mut stack)?;
        let resolved = new_config.resolve()?;

        // Conserve l'expansion / la sélection actuelles
        let old_expanded = self.expanded_items.clone();
        let old_idx = self.selected_index;

        self.config = new_config;
        self.resolved_servers = resolved;
        self.warnings = new_warnings;
        self.validation_warnings = new_val_warnings;
        self.expanded_items = old_expanded;
        self.items_dirty = true;
        self.selected_index = old_idx;
        self.list_state.select(Some(old_idx));

        self.set_status_message(self.lang.config_reloaded);
        Ok(())
    }

    // ─── Commande ad-hoc ──────────────────────────────────────────────────────

    /// Lance une commande SSH non-interactive dans un thread dédié.
    /// Stocke le résultat via `cmd_rx`.
    pub fn start_cmd(&mut self, server: &ResolvedServer, cmd: String) {
        let host = server.host.clone();
        let user = server.user.clone();
        let port = server.port;
        let key = server.ssh_key.clone();
        let cmd_clone = cmd.clone();

        let (tx, rx) = mpsc::channel();
        self.cmd_state = CmdState::Running(cmd.clone());
        self.cmd_rx = Some(rx);

        std::thread::spawn(move || {
            let mut args = vec![
                "-o".to_string(),
                "BatchMode=yes".to_string(),
                "-o".to_string(),
                "ConnectTimeout=10".to_string(),
                "-p".to_string(),
                port.to_string(),
            ];
            if !key.is_empty() {
                let expanded = shellexpand::tilde(&key).to_string();
                args.push("-i".to_string());
                args.push(expanded);
            }
            args.push(format!("{}@{}", user, host));
            args.push(cmd_clone.clone());

            let result = std::process::Command::new("ssh").args(&args).output();

            match result {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                    let combined = if stderr.is_empty() {
                        stdout
                    } else if stdout.is_empty() {
                        stderr
                    } else {
                        format!("{}\n---\n{}", stdout, stderr)
                    };
                    let _ = tx.send((combined, out.status.success()));
                }
                Err(e) => {
                    let _ = tx.send((e.to_string(), false));
                }
            }
        });
    }

    /// Vérifie si le thread de commande a produit un résultat et met à jour `cmd_state`.
    pub fn poll_cmd(&mut self) {
        let done = if let Some(rx) = &self.cmd_rx {
            rx.try_recv().ok()
        } else {
            None
        };
        if let Some((output, exit_ok)) = done {
            let cmd = match &self.cmd_state {
                CmdState::Running(c) => c.clone(),
                _ => String::new(),
            };
            self.cmd_state = CmdState::Done {
                cmd,
                output,
                exit_ok,
            };
            self.cmd_rx = None;
        }
    }

    /// Réinitialise l'état de la commande ad-hoc.
    pub fn reset_cmd(&mut self) {
        self.cmd_state = CmdState::Idle;
        self.cmd_rx = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ConfigEntry, Environment, Group, Server};

    fn create_test_config() -> Config {
        Config {
            defaults: None,
            includes: vec![],
            groups: vec![ConfigEntry::Group(Group {
                name: "G1".to_string(),
                user: None,
                ssh_key: None,
                mode: None,
                ssh_port: None,
                ssh_options: None,
                wallix: None,
                jump: None,
                probe_filesystems: None,
                environments: Some(vec![Environment {
                    name: "E1".to_string(),
                    user: None,
                    ssh_key: None,
                    mode: None,
                    ssh_port: None,
                    ssh_options: None,
                    wallix: None,
                    jump: None,
                    probe_filesystems: None,
                    servers: vec![Server {
                        name: "S1".to_string(),
                        host: "10.0.0.1".to_string(),
                        user: None,
                        ssh_key: None,
                        ssh_port: None,
                        ssh_options: None,
                        mode: None,
                        wallix: None,
                        jump: None,
                        probe_filesystems: None,
                    }],
                }]),
                servers: Some(vec![Server {
                    name: "S2".to_string(),
                    host: "10.0.0.2".to_string(),
                    user: None,
                    ssh_key: None,
                    ssh_port: None,
                    ssh_options: None,
                    mode: None,
                    wallix: None,
                    jump: None,
                    probe_filesystems: None,
                }]),
            })],
        }
    }

    #[test]
    fn test_initial_visibility() {
        let config = create_test_config();
        let mut app = App::new(config, vec![], std::path::PathBuf::new(), vec![]).unwrap();
        let items = app.get_visible_items();

        // Initially only the group header is visible (collapsed state)
        assert_eq!(items.len(), 1);
        match &items[0] {
            ConfigItem::Group(name, _ns) => assert_eq!(name, "G1"),
            _ => panic!("Expected Group G1"),
        }
    }

    #[test]
    fn test_expansion() {
        let config = create_test_config();
        let mut app = App::new(config, vec![], std::path::PathBuf::new(), vec![]).unwrap();

        // Expand Expand group G1
        app.toggle_expansion();
        // Note: selected_index is 0, pointing to G1.

        let items = app.get_visible_items();

        // Should verify items: G1, E1, S2 (Environment header + direct server)
        // E1 is collapsed by default.

        // Order inside Group iteration:
        // 1. Environments (E1)
        // 2. Servers (S2)

        // So: G1, E1, S2
        assert_eq!(items.len(), 3);

        match &items[1] {
            ConfigItem::Environment(g, e, _ns) => {
                assert_eq!(g, "G1");
                assert_eq!(e, "E1");
            }
            _ => panic!("Expected Environment E1"),
        }

        match &items[2] {
            ConfigItem::Server(s) => assert_eq!(s.name, "S2"),
            _ => panic!("Expected Server S2"),
        }
    }

    #[test]
    fn test_collapse_all() {
        let config = create_test_config();
        let mut app = App::new(config, vec![], std::path::PathBuf::new(), vec![]).unwrap();

        // Expand G1, then navigate and expand E1
        app.toggle_expansion(); // expands G1 (index 0)
        app.selected_index = 1;
        app.items_dirty = true;
        app.toggle_expansion(); // expands E1

        // Vérifier que des items sont bien expandés
        assert!(!app.expanded_items.is_empty());

        // Replier tout
        app.collapse_all();

        assert!(app.expanded_items.is_empty());
        assert_eq!(app.selected_index, 0);
        // Seul le groupe racine doit être visible
        let items = app.get_visible_items();
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_search_filtering() {
        let config = create_test_config();
        let mut app = App::new(config, vec![], std::path::PathBuf::new(), vec![]).unwrap();

        app.search_query = "S1".to_string();
        app.invalidate_cache();
        let items = app.get_visible_items();

        // With search "S1":
        // G1 (header always shown)
        // E1 (header always shown inside expanded group)
        // S1 (matches "S1")
        // S2 (filtered out)

        assert!(items.len() >= 3);

        // Verify S1 is present
        let has_s1 = items.iter().any(|i| match i {
            ConfigItem::Server(s) => s.name == "S1",
            _ => false,
        });
        assert!(has_s1, "Should contain S1");

        // Verify S2 is filtered out
        let has_s2 = items.iter().any(|i| match i {
            ConfigItem::Server(s) => s.name == "S2",
            _ => false,
        });
        assert!(!has_s2, "Should NOT contain S2");
    }

    fn make_namespace_config() -> Config {
        use crate::config::{NamespaceEntry, Server};
        Config {
            defaults: None,
            includes: vec![],
            groups: vec![
                ConfigEntry::Group(crate::config::Group {
                    name: "RootGroup".to_string(),
                    user: None,
                    ssh_key: None,
                    mode: None,
                    ssh_port: None,
                    ssh_options: None,
                    wallix: None,
                    jump: None,
                    probe_filesystems: None,
                    environments: None,
                    servers: Some(vec![Server {
                        name: "root_srv".to_string(),
                        host: "1.1.1.1".to_string(),
                        user: None,
                        ssh_key: None,
                        ssh_port: None,
                        ssh_options: None,
                        mode: None,
                        wallix: None,
                        jump: None,
                        probe_filesystems: None,
                    }]),
                }),
                ConfigEntry::Namespace(NamespaceEntry {
                    label: "CES".to_string(),
                    source_path: "/fake/ces.yml".to_string(),
                    defaults: None,
                    entries: vec![ConfigEntry::Group(crate::config::Group {
                        name: "CES_Group".to_string(),
                        user: None,
                        ssh_key: None,
                        mode: None,
                        ssh_port: None,
                        ssh_options: None,
                        wallix: None,
                        jump: None,
                        probe_filesystems: None,
                        environments: None,
                        servers: Some(vec![Server {
                            name: "ces_srv".to_string(),
                            host: "2.2.2.2".to_string(),
                            user: None,
                            ssh_key: None,
                            ssh_port: None,
                            ssh_options: None,
                            mode: None,
                            wallix: None,
                            jump: None,
                            probe_filesystems: None,
                        }]),
                    })],
                }),
            ],
        }
    }

    #[test]
    fn test_namespace_visibility_collapsed() {
        let config = make_namespace_config();
        let mut app = App::new(config, vec![], std::path::PathBuf::new(), vec![]).unwrap();
        // Reset persistent state to ensure a clean initial state independent of ~/.susshi_state.json
        app.expanded_items.clear();
        app.invalidate_cache();
        let items = app.get_visible_items();

        // Collapsed: RootGroup header + Namespace header only (2 items)
        assert_eq!(items.len(), 2);
        matches!(&items[0], ConfigItem::Group(name, ns) if name == "RootGroup" && ns.is_empty());
        matches!(&items[1], ConfigItem::Namespace(label) if label == "CES");
    }

    #[test]
    fn test_namespace_expansion() {
        let config = make_namespace_config();
        let mut app = App::new(config, vec![], std::path::PathBuf::new(), vec![]).unwrap();
        // Reset persistent state to ensure a clean initial state independent of ~/.susshi_state.json
        app.expanded_items.clear();
        app.invalidate_cache();

        // Select the Namespace item (index 1) and expand it
        app.select(1);
        app.toggle_expansion();

        let items = app.get_visible_items();

        // After expanding CES: RootGroup, CES (ns header), CES_Group (group inside ns)
        assert_eq!(items.len(), 3);
        matches!(&items[2], ConfigItem::Group(name, ns) if name == "CES_Group" && ns == "CES");
    }

    #[test]
    fn test_search_crosses_namespaces() {
        let config = make_namespace_config();
        let mut app = App::new(config, vec![], std::path::PathBuf::new(), vec![]).unwrap();

        app.search_query = "ces_srv".to_string();
        app.invalidate_cache();
        let items = app.get_visible_items();

        let has_ces = items.iter().any(|i| match i {
            ConfigItem::Server(s) => s.name == "ces_srv",
            _ => false,
        });
        assert!(has_ces, "Search should find ces_srv in namespace CES");

        let has_root = items.iter().any(|i| match i {
            ConfigItem::Server(s) => s.name == "root_srv",
            _ => false,
        });
        assert!(!has_root, "root_srv should be filtered out");
    }
}
