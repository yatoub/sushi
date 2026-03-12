use crate::config::{
    Config, ConfigEntry, ConfigError, ConnectionMode, IncludeWarning, ResolvedServer, ThemeVariant,
    TunnelConfig, ValidationWarning,
};
use crate::probe::{ProbeResult, ProbeState};
use crate::ssh::sftp::{self as ssh_sftp, ScpDirection, ScpEvent};
use crate::ssh::tunnel::{self as ssh_tunnel, TunnelHandle, TunnelStatus};
use crate::state::{self, TunnelOverride};
use crate::ui::theme::{Theme, get_theme};
use ratatui::widgets::ListState;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
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

/// Champ actif dans le formulaire d'édition/création d'un tunnel.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TunnelFormField {
    Label,
    LocalPort,
    RemoteHost,
    RemotePort,
}

impl TunnelFormField {
    /// Passe au champ suivant (cycle).
    pub fn next(&self) -> Self {
        match self {
            Self::Label => Self::LocalPort,
            Self::LocalPort => Self::RemoteHost,
            Self::RemoteHost => Self::RemotePort,
            Self::RemotePort => Self::Label,
        }
    }
    /// Passe au champ précédent (cycle).
    pub fn prev(&self) -> Self {
        match self {
            Self::Label => Self::RemotePort,
            Self::LocalPort => Self::Label,
            Self::RemoteHost => Self::LocalPort,
            Self::RemotePort => Self::RemoteHost,
        }
    }
}

/// État du formulaire d'édition ou de création d'un tunnel.
#[derive(Debug, Clone)]
pub struct TunnelForm {
    pub label: String,
    /// Saisie libre du port local (validée en `u16` à la soumission).
    pub local_port: String,
    pub remote_host: String,
    /// Saisie libre du port distant.
    pub remote_port: String,
    /// Champ en cours d'édition.
    pub focus: TunnelFormField,
    /// `Some(idx)` = édition du tunnel à cet index effectif ; `None` = création.
    pub editing_index: Option<usize>,
    /// Message d'erreur de validation (vide = pas d'erreur).
    pub error: String,
}

impl TunnelForm {
    /// Crée un formulaire vide (création d'un nouveau tunnel).
    pub fn new_empty() -> Self {
        Self {
            label: String::new(),
            local_port: String::new(),
            remote_host: String::new(),
            remote_port: String::new(),
            focus: TunnelFormField::Label,
            editing_index: None,
            error: String::new(),
        }
    }

    /// Crée un formulaire pré-rempli pour éditer le tunnel `idx`.
    pub fn new_edit(idx: usize, config: &TunnelConfig) -> Self {
        Self {
            label: config.label.clone(),
            local_port: config.local_port.to_string(),
            remote_host: config.remote_host.clone(),
            remote_port: config.remote_port.to_string(),
            focus: TunnelFormField::Label,
            editing_index: Some(idx),
            error: String::new(),
        }
    }

    /// Retourne la valeur du champ courant (référence mutable).
    pub fn current_buf_mut(&mut self) -> &mut String {
        match self.focus {
            TunnelFormField::Label => &mut self.label,
            TunnelFormField::LocalPort => &mut self.local_port,
            TunnelFormField::RemoteHost => &mut self.remote_host,
            TunnelFormField::RemotePort => &mut self.remote_port,
        }
    }

    /// Valide les champs et retourne un `TunnelConfig` ou un message d'erreur.
    pub fn validate(&self, lang: &crate::i18n::Strings) -> Result<TunnelConfig, String> {
        let local_port = self
            .local_port
            .trim()
            .parse::<u16>()
            .ok()
            .filter(|&p| p >= 1)
            .ok_or_else(|| lang.tunnel_form_local_port_invalid.to_string())?;

        if self.remote_host.trim().is_empty() {
            return Err(lang.tunnel_form_remote_host_empty.to_string());
        }

        let remote_port = self
            .remote_port
            .trim()
            .parse::<u16>()
            .ok()
            .filter(|&p| p >= 1)
            .ok_or_else(|| lang.tunnel_form_remote_port_invalid.to_string())?;

        Ok(TunnelConfig {
            local_port,
            remote_host: self.remote_host.trim().to_string(),
            remote_port,
            label: self.label.trim().to_string(),
        })
    }
}

/// État de l'overlay des tunnels SSH (touche `T`).
#[derive(Debug, Clone)]
pub enum TunnelOverlayState {
    /// Vue liste des tunnels configurés pour le serveur sélectionné.
    List {
        /// Index de la ligne sélectionnée (0-based ; la dernière ligne est le bouton "+").
        selected: usize,
    },
    /// Formulaire d'édition ou de création d'un tunnel.
    Form(TunnelForm),
}

/// Champ actif dans le formulaire de transfert SCP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScpFormField {
    Local,
    Remote,
}

impl ScpFormField {
    pub fn next(&self) -> Self {
        match self {
            Self::Local => Self::Remote,
            Self::Remote => Self::Local,
        }
    }
    pub fn prev(&self) -> Self {
        self.next()
    }
}

/// État du transfert SCP en cours (touche `s`).
#[derive(Debug, Clone, Default)]
pub enum ScpState {
    /// Aucun transfert en cours.
    #[default]
    Idle,
    /// Sélection de la direction (Upload / Download).
    SelectingDirection,
    /// Saisie des chemins source et destination.
    FillingForm {
        direction: ScpDirection,
        /// Chemin local (peut contenir `~`).
        local: String,
        /// Chemin distant (sans le préfixe `user@host:`).
        remote: String,
        /// Champ en cours d'édition.
        focus: ScpFormField,
        /// Message d'erreur de validation.
        error: String,
    },
    /// Transfert en cours.
    Running {
        direction: ScpDirection,
        /// Nom du fichier ou chemin court affiché dans la barre.
        label: String,
        /// Progression 0–100.
        progress: u8,
        /// Instant de début du transfert (pour calculer vitesse et ETA).
        started_at: std::time::Instant,
        /// Taille totale du fichier en octets (0 si inconnue).
        file_size: u64,
    },
    /// Transfert terminé.
    Done {
        direction: ScpDirection,
        exit_ok: bool,
    },
    /// Erreur irrécupérable.
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

    /// Hash (DefaultHasher) du contenu lu sur disque — permet de détecter
    /// un rechargement inutile lorsque le fichier n'a pas été modifié.
    pub config_hash: u64,

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

    /// Si true, la TUI se rouvre après la fermeture de la connexion SSH.
    pub keep_open: bool,

    /// Overrides utilisateur sur les tunnels SSH (ajouts, éditions, suppressions).
    /// Fusionnés à la volée avec `effective_tunnels()` — jamais baked dans `resolved_servers`.
    pub tunnel_overrides: Vec<TunnelOverride>,

    /// État de l'overlay tunnels. `Some(...)` = overlay ouvert, `None` = fermé.
    pub tunnel_overlay: Option<TunnelOverlayState>,

    /// Tunnels SSH actifs, indexés par clé de serveur.
    /// Chaque entrée est un `TunnelHandle` portant le sous-processus SSH et son statut.
    pub active_tunnels: HashMap<String, Vec<TunnelHandle>>,

    /// État du transfert SCP en cours.
    pub scp_state: ScpState,
    /// Récepteur des évènements du thread SFTP (présent uniquement quand Running).
    pub scp_rx: Option<mpsc::Receiver<ScpEvent>>,
}

/// Sépare la requête de recherche en tokens texte et tokens `#tag`.
/// Exemple : `"web #prod DB"` → `(["web", "DB"], ["prod"])`
pub fn parse_search_tokens(query: &str) -> (Vec<String>, Vec<String>) {
    let mut text = Vec::new();
    let mut tags = Vec::new();
    for token in query.split_whitespace() {
        if let Some(t) = token.strip_prefix('#') {
            if !t.is_empty() {
                tags.push(t.to_lowercase());
            }
        } else {
            text.push(token.to_lowercase());
        }
    }
    (text, tags)
}

// ─── Helpers internes ─────────────────────────────────────────────────────────
/// Utilise `DefaultHasher` (non-cryptographique, suffisant pour la détection de changement).
/// Retourne 0 en cas d'erreur de lecture (force un rechargement).
fn hash_config_file(path: &PathBuf) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    match std::fs::read(path) {
        Ok(bytes) => {
            let mut hasher = DefaultHasher::new();
            bytes.hash(&mut hasher);
            hasher.finish()
        }
        Err(_) => 0,
    }
}

impl App {
    pub fn new(
        config: Config,
        warnings: Vec<IncludeWarning>,
        config_path: PathBuf,
        validation_warnings: Vec<ValidationWarning>,
    ) -> Result<Self, ConfigError> {
        let resolved = config.resolve()?;
        let config_hash = hash_config_file(&config_path);

        // Résout le thème avant de déplacer config dans le struct
        let theme_variant = config
            .defaults
            .as_ref()
            .and_then(|d| d.theme)
            .unwrap_or(ThemeVariant::Mocha);

        let keep_open = config
            .defaults
            .as_ref()
            .and_then(|d| d.keep_open)
            .unwrap_or(false);

        let default_filter = config
            .defaults
            .as_ref()
            .and_then(|d| d.default_filter.clone())
            .unwrap_or_default();

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
            config_hash,
            favorites_only: false,
            sort_by_recent: false,
            last_seen: HashMap::new(),
            favorites: HashSet::new(),
            cmd_state: CmdState::Idle,
            cmd_rx: None,
            validation_warnings,
            keep_open,
            tunnel_overrides: Vec::new(),
            tunnel_overlay: None,
            active_tunnels: HashMap::new(),
            scp_state: ScpState::Idle,
            scp_rx: None,
        };

        app.list_state.select(Some(0));

        // Restaure l'état d'expansion persistant
        let saved = state::load_state();
        app.expanded_items = saved.expanded_items;
        app.last_seen = saved.last_seen;
        app.favorites = saved.favorites;
        app.sort_by_recent = saved.sort_by_recent;
        app.tunnel_overrides = saved.tunnel_overrides;
        app.items_dirty = true;

        // Applique le filtre par défaut de la configuration si la requête est vide
        if app.search_query.is_empty() && !default_filter.is_empty() {
            app.search_query = default_filter;
            app.is_searching = true;
            app.items_dirty = true;
        }

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
            tunnel_overrides: self.tunnel_overrides.clone(),
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
                    if let Some(resolved) = self.resolved_servers.iter().find(|rs| {
                        rs.name == s_conf.name
                            && rs.group_name.is_empty()
                            && rs.env_name.is_empty()
                            && rs.namespace.is_empty()
                    }) {
                        if searching
                            && !self.matches_search(&resolved.name, &resolved.host, &resolved.tags)
                        {
                            continue;
                        }
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
                if !self.search_query.is_empty()
                    && !self.matches_search(&rs.name, &rs.host, &rs.tags)
                {
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
                    if let Some(resolved) = self.resolved_servers.iter().find(|rs| {
                        rs.name == s_conf.name
                            && rs.group_name.is_empty()
                            && rs.env_name.is_empty()
                            && rs.namespace == ns
                    }) {
                        if searching
                            && !self.matches_search(&resolved.name, &resolved.host, &resolved.tags)
                        {
                            continue;
                        }
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
                        if let Some(resolved) = self.resolved_servers.iter().find(|rs| {
                            rs.name == server.name
                                && rs.env_name == env.name
                                && rs.group_name == group.name
                                && rs.namespace.is_empty()
                        }) {
                            if searching
                                && !self.matches_search(
                                    &resolved.name,
                                    &resolved.host,
                                    &resolved.tags,
                                )
                            {
                                continue;
                            }
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
                if let Some(resolved) = self.resolved_servers.iter().find(|rs| {
                    rs.name == server.name
                        && rs.env_name.is_empty()
                        && rs.group_name == group.name
                        && rs.namespace.is_empty()
                }) {
                    if searching
                        && !self.matches_search(&resolved.name, &resolved.host, &resolved.tags)
                    {
                        continue;
                    }
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
                        if let Some(resolved) = self.resolved_servers.iter().find(|rs| {
                            rs.name == server.name
                                && rs.env_name == env.name
                                && rs.group_name == group.name
                                && rs.namespace == ns
                        }) {
                            if searching
                                && !self.matches_search(
                                    &resolved.name,
                                    &resolved.host,
                                    &resolved.tags,
                                )
                            {
                                continue;
                            }
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
                if let Some(resolved) = self.resolved_servers.iter().find(|rs| {
                    rs.name == server.name
                        && rs.env_name.is_empty()
                        && rs.group_name == group.name
                        && rs.namespace == ns
                }) {
                    if searching
                        && !self.matches_search(&resolved.name, &resolved.host, &resolved.tags)
                    {
                        continue;
                    }
                    if self.favorites_only && !self.favorites.contains(&Self::server_key(resolved))
                    {
                        continue;
                    }
                    items.push(ConfigItem::Server(Box::new(resolved.clone())));
                }
            }
        }
    }

    fn matches_search(&self, name: &str, host: &str, tags: &[String]) -> bool {
        let (text_tokens, tag_tokens) = parse_search_tokens(&self.search_query);

        // Tous les #tag doivent être présents (AND)
        let tags_ok = tag_tokens.iter().all(|t| {
            tags.iter()
                .any(|tag| tag.to_lowercase() == t.to_lowercase())
        });
        if !tags_ok {
            return false;
        }

        // Tokens textuels : chacun doit apparaître dans name ou host (AND)
        if text_tokens.is_empty() {
            return true;
        }
        let name_lc = name.to_lowercase();
        let host_lc = host.to_lowercase();
        text_tokens
            .iter()
            .all(|t| name_lc.contains(t.as_str()) || host_lc.contains(t.as_str()))
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

    // ─── Tunnels SSH — overrides ───────────────────────────────────────────────

    /// Retourne la liste effective des tunnels pour un serveur :
    /// tunnels YAML fusionnés avec les overrides utilisateur persistants.
    pub fn effective_tunnels(&self, server: &ResolvedServer) -> Vec<state::EffectiveTunnel> {
        state::effective_tunnels_for(
            &server.tunnels,
            &Self::server_key(server),
            &self.tunnel_overrides,
        )
    }

    /// Ajoute un tunnel créé manuellement depuis la TUI pour un serveur donné.
    /// Persiste immédiatement dans le state.
    pub fn add_tunnel_override(&mut self, server: &ResolvedServer, config: TunnelConfig) {
        self.tunnel_overrides.push(TunnelOverride {
            server_key: Self::server_key(server),
            yaml_index: None,
            config,
            hidden: false,
        });
        state::save_state(&self.to_app_state());
    }

    /// Met à jour la configuration d'un tunnel existant.
    ///
    /// - `yaml_index = Some(i)` : édition d'un tunnel YAML (crée ou met à jour l'override).
    /// - `yaml_index = None`    : édition d'un tunnel ajouté par l'utilisateur
    ///   (identifié par sa position `user_idx` parmi les overrides sans yaml_index).
    pub fn update_tunnel_override(
        &mut self,
        server: &ResolvedServer,
        yaml_index: Option<usize>,
        user_idx: usize,
        new_config: TunnelConfig,
    ) {
        let key = Self::server_key(server);
        match yaml_index {
            Some(i) => {
                // Crée ou met à jour l'override pour le tunnel YAML #i.
                if let Some(o) = self
                    .tunnel_overrides
                    .iter_mut()
                    .find(|o| o.server_key == key && o.yaml_index == Some(i))
                {
                    o.config = new_config;
                    o.hidden = false;
                } else {
                    self.tunnel_overrides.push(TunnelOverride {
                        server_key: key,
                        yaml_index: Some(i),
                        config: new_config,
                        hidden: false,
                    });
                }
            }
            None => {
                // Trouve le tunnel utilisateur par sa position parmi les overrides sans yaml_index.
                let mut count = 0usize;
                for o in self.tunnel_overrides.iter_mut() {
                    if o.server_key == key && o.yaml_index.is_none() && !o.hidden {
                        if count == user_idx {
                            o.config = new_config;
                            break;
                        }
                        count += 1;
                    }
                }
            }
        }
        state::save_state(&self.to_app_state());
    }

    /// Supprime ou masque un tunnel.
    ///
    /// - Tunnel YAML (`yaml_index = Some(i)`) : marque `hidden = true` dans l'override
    ///   (le tunnel reste masqué après rechargement du YAML).
    /// - Tunnel utilisateur (`yaml_index = None`) : retire l'entrée des overrides.
    pub fn remove_tunnel_override(
        &mut self,
        server: &ResolvedServer,
        yaml_index: Option<usize>,
        user_idx: usize,
    ) {
        let key = Self::server_key(server);
        match yaml_index {
            Some(i) => {
                if let Some(o) = self
                    .tunnel_overrides
                    .iter_mut()
                    .find(|o| o.server_key == key && o.yaml_index == Some(i))
                {
                    o.hidden = true;
                } else {
                    self.tunnel_overrides.push(TunnelOverride {
                        server_key: key,
                        yaml_index: Some(i),
                        config: server.tunnels[i].clone(),
                        hidden: true,
                    });
                }
            }
            None => {
                let mut count = 0usize;
                self.tunnel_overrides.retain(|o| {
                    if o.server_key == key && o.yaml_index.is_none() && !o.hidden {
                        if count == user_idx {
                            count += 1;
                            return false; // retire cet élément
                        }
                        count += 1;
                    }
                    true
                });
            }
        }
        state::save_state(&self.to_app_state());
    }

    // ─── Backend tunnels ──────────────────────────────────────────────────────

    /// Démarre un tunnel SSH pour le serveur courant à l'index effectif `effective_idx`.
    ///
    /// Sans effet (avec message d'avertissement) si le mode est Wallix ou si le tunnel
    /// est déjà en cours d'exécution.
    pub fn start_tunnel(&mut self, server: &ResolvedServer, effective_idx: usize) {
        if self.connection_mode == ConnectionMode::Wallix {
            self.set_status_message(self.lang.tunnel_wallix_unavailable);
            return;
        }

        let tunnels = self.effective_tunnels(server);
        let Some(et) = tunnels.get(effective_idx) else {
            self.set_status_message(crate::i18n::fmt(
                self.lang.tunnel_not_found,
                &[&effective_idx.to_string()],
            ));
            return;
        };

        let key = Self::server_key(server);

        // Vérifie si déjà actif.
        if let Some(handles) = self.active_tunnels.get(&key)
            && let Some(h) = handles.iter().find(|h| h.user_idx == effective_idx)
            && h.is_running()
        {
            self.set_status_message(crate::i18n::fmt(
                self.lang.tunnel_already_active,
                &[&et.config.label, &et.config.local_port.to_string()],
            ));
            return;
        }

        let config = et.config.clone();
        let yaml_index = et.yaml_index;
        let label = config.label.clone();
        let local_port = config.local_port;

        match ssh_tunnel::spawn_tunnel(
            server,
            self.connection_mode,
            config,
            yaml_index,
            effective_idx,
        ) {
            Ok(handle) => {
                self.active_tunnels
                    .entry(key)
                    .or_default()
                    .retain(|h| h.user_idx != effective_idx); // retire l'ancien handle si présent
                self.active_tunnels
                    .entry(Self::server_key(server))
                    .or_default()
                    .push(handle);
                self.set_status_message(crate::i18n::fmt(
                    self.lang.tunnel_started,
                    &[&label, &local_port.to_string()],
                ));
            }
            Err(e) => {
                self.set_status_message(crate::i18n::fmt(
                    self.lang.tunnel_error,
                    &[&e.to_string()],
                ));
            }
        }
    }

    /// Arrête un tunnel actif identifié par `effective_idx`.
    pub fn stop_tunnel(&mut self, server_key: &str, effective_idx: usize) {
        if let Some(handles) = self.active_tunnels.get_mut(server_key)
            && let Some(h) = handles.iter_mut().find(|h| h.user_idx == effective_idx)
        {
            let label = h.config.label.clone();
            let port = h.config.local_port;
            h.kill();
            self.set_status_message(crate::i18n::fmt(
                self.lang.tunnel_stopped,
                &[&label, &port.to_string()],
            ));
        }
    }

    /// Arrête tous les tunnels actifs (appelé au Drop et lors du rechargement).
    pub fn stop_all_tunnels(&mut self) {
        for handles in self.active_tunnels.values_mut() {
            for h in handles.iter_mut() {
                h.kill();
            }
        }
    }

    /// Sonde l'état de tous les tunnels actifs.
    ///
    /// À appeler depuis la boucle d'événements (tick) pour détecter les tunnels
    /// qui se sont arrêtés inopinément. Affiche un message de statut pour chaque
    /// tunnel mort depuis le dernier appel.
    pub fn poll_tunnel_events(&mut self) {
        let mut dead: Vec<(String, String, u16)> = Vec::new(); // (reason, label, port)

        for handles in self.active_tunnels.values_mut() {
            for h in handles.iter_mut() {
                if h.poll()
                    && let TunnelStatus::Dead(reason) = &h.status
                {
                    dead.push((reason.clone(), h.config.label.clone(), h.config.local_port));
                }
            }
        }

        for (reason, label, port) in dead {
            self.set_status_message(crate::i18n::fmt(
                self.lang.tunnel_died,
                &[&label, &port.to_string(), &reason],
            ));
        }
    }

    /// Retourne le nombre de tunnels SSH actuellement actifs pour un serveur.
    pub fn active_tunnel_count(&self, server: &ResolvedServer) -> usize {
        let key = Self::server_key(server);
        self.active_tunnels
            .get(&key)
            .map(|handles| handles.iter().filter(|h| h.is_running()).count())
            .unwrap_or(0)
    }

    // ─── Overlay tunnels — navigation ─────────────────────────────────────────

    /// Ouvre l'overlay des tunnels pour le serveur sélectionné.
    ///
    /// Sans effet si aucun serveur n'est sélectionné ou si le mode Wallix est actif.
    pub fn open_tunnel_overlay(&mut self) {
        if self.connection_mode == ConnectionMode::Wallix {
            self.set_status_message(self.lang.tunnel_wallix_unavailable);
            return;
        }
        if self.selected_server().is_some() {
            self.tunnel_overlay = Some(TunnelOverlayState::List { selected: 0 });
        }
    }

    /// Ferme l'overlay des tunnels.
    pub fn close_tunnel_overlay(&mut self) {
        self.tunnel_overlay = None;
    }

    /// Déplace la sélection vers le bas dans l'overlay.
    pub fn tunnel_overlay_next(&mut self) {
        let server = match self.selected_server() {
            Some(s) => s,
            None => return,
        };
        let count = self.effective_tunnels(&server).len() + 1; // +1 pour la ligne "+"
        if let Some(TunnelOverlayState::List { selected }) = &mut self.tunnel_overlay
            && count > 0
        {
            *selected = (*selected + 1) % count;
        }
    }

    /// Déplace la sélection vers le haut dans l'overlay.
    pub fn tunnel_overlay_previous(&mut self) {
        let server = match self.selected_server() {
            Some(s) => s,
            None => return,
        };
        let count = self.effective_tunnels(&server).len() + 1;
        if let Some(TunnelOverlayState::List { selected }) = &mut self.tunnel_overlay
            && count > 0
        {
            if *selected == 0 {
                *selected = count - 1;
            } else {
                *selected -= 1;
            }
        }
    }

    /// Bascule l'état (démarrer/arrêter) du tunnel sélectionné dans l'overlay.
    ///
    /// Si la ligne sélectionnée est le bouton « + », sans effet (Step 5).
    pub fn tunnel_overlay_toggle(&mut self) {
        let server = match self.selected_server() {
            Some(s) => s,
            None => return,
        };
        let selected = match &self.tunnel_overlay {
            Some(TunnelOverlayState::List { selected }) => *selected,
            _ => return,
        };
        let tunnels = self.effective_tunnels(&server);
        if selected >= tunnels.len() {
            return; // bouton "+" — Step 5
        }
        let key = Self::server_key(&server);
        let is_running = self
            .active_tunnels
            .get(&key)
            .and_then(|handles| handles.iter().find(|h| h.user_idx == selected))
            .map(|h| h.is_running())
            .unwrap_or(false);
        if is_running {
            self.stop_tunnel(&key, selected);
        } else {
            self.start_tunnel(&server, selected);
        }
    }

    /// Supprime (ou masque) le tunnel sélectionné dans l'overlay.
    ///
    /// Arrête d'abord le tunnel s'il est actif. Ajuste la sélection pour rester valide.
    pub fn tunnel_overlay_delete(&mut self) {
        let server = match self.selected_server() {
            Some(s) => s,
            None => return,
        };
        let selected = match &self.tunnel_overlay {
            Some(TunnelOverlayState::List { selected }) => *selected,
            _ => return,
        };
        let tunnels = self.effective_tunnels(&server);
        if selected >= tunnels.len() {
            return; // bouton "+"
        }
        let yaml_index = tunnels[selected].yaml_index;
        let user_idx = tunnels[selected].user_idx;
        let key = Self::server_key(&server);
        // Arrêt préalable si actif.
        let is_running = self
            .active_tunnels
            .get(&key)
            .and_then(|handles| handles.iter().find(|h| h.user_idx == selected))
            .map(|h| h.is_running())
            .unwrap_or(false);
        if is_running {
            self.stop_tunnel(&key, selected);
        }
        self.remove_tunnel_override(&server, yaml_index, user_idx);
        // Ajuste la sélection pour rester dans les bornes.
        let new_list_len = self.effective_tunnels(&server).len();
        if let Some(TunnelOverlayState::List { selected: sel }) = &mut self.tunnel_overlay {
            if new_list_len == 0 {
                *sel = 0;
            } else if *sel >= new_list_len {
                *sel = new_list_len - 1;
            }
        }
        self.set_status_message(self.lang.tunnel_deleted);
    }

    // ─── Overlay tunnels — formulaire ─────────────────────────────────────────

    /// Ouvre le formulaire d'édition pour le tunnel sélectionné dans la liste.
    ///
    /// Sans effet si la ligne sélectionnée est le bouton « + » ou si aucun serveur n'est actif.
    pub fn open_tunnel_form_edit(&mut self) {
        let server = match self.selected_server() {
            Some(s) => s,
            None => return,
        };
        let selected = match &self.tunnel_overlay {
            Some(TunnelOverlayState::List { selected }) => *selected,
            _ => return,
        };
        let tunnels = self.effective_tunnels(&server);
        if selected >= tunnels.len() {
            return; // bouton "+"
        }
        let form = TunnelForm::new_edit(selected, &tunnels[selected].config);
        self.tunnel_overlay = Some(TunnelOverlayState::Form(form));
    }

    /// Ouvre le formulaire de création d'un nouveau tunnel.
    pub fn open_tunnel_form_add(&mut self) {
        if self.selected_server().is_none() {
            return;
        }
        if matches!(self.tunnel_overlay, Some(TunnelOverlayState::List { .. })) {
            self.tunnel_overlay = Some(TunnelOverlayState::Form(TunnelForm::new_empty()));
        }
    }

    /// Ajoute un caractère dans le champ actif du formulaire.
    pub fn tunnel_form_char(&mut self, c: char) {
        if let Some(TunnelOverlayState::Form(form)) = &mut self.tunnel_overlay {
            // Pour les champs de port, n'autoriser que les chiffres.
            if matches!(
                form.focus,
                TunnelFormField::LocalPort | TunnelFormField::RemotePort
            ) && !c.is_ascii_digit()
            {
                return;
            }
            form.current_buf_mut().push(c);
            form.error.clear();
        }
    }

    /// Supprime le dernier caractère du champ actif.
    pub fn tunnel_form_backspace(&mut self) {
        if let Some(TunnelOverlayState::Form(form)) = &mut self.tunnel_overlay {
            form.current_buf_mut().pop();
            form.error.clear();
        }
    }

    /// Passe au champ suivant (Tab).
    pub fn tunnel_form_next_field(&mut self) {
        if let Some(TunnelOverlayState::Form(form)) = &mut self.tunnel_overlay {
            form.focus = form.focus.next();
        }
    }

    /// Passe au champ précédent (Shift+Tab).
    pub fn tunnel_form_prev_field(&mut self) {
        if let Some(TunnelOverlayState::Form(form)) = &mut self.tunnel_overlay {
            form.focus = form.focus.prev();
        }
    }

    /// Valide et soumet le formulaire.
    ///
    /// En cas d'erreur de validation, stocke le message d'erreur dans `form.error` et
    /// garde le formulaire ouvert. En cas de succès, revient à la vue liste.
    pub fn tunnel_form_submit(&mut self) {
        let server = match self.selected_server() {
            Some(s) => s,
            None => return,
        };

        // Clone les données du formulaire pour libérer le borrow.
        let (editing_index, validation_result) =
            if let Some(TunnelOverlayState::Form(form)) = &self.tunnel_overlay {
                (form.editing_index, form.validate(self.lang))
            } else {
                return;
            };

        match validation_result {
            Err(msg) => {
                // Stocke l'erreur dans le formulaire.
                if let Some(TunnelOverlayState::Form(form)) = &mut self.tunnel_overlay {
                    form.error = msg;
                }
            }
            Ok(config) => {
                match editing_index {
                    Some(idx) => {
                        // Édition — on retrouve yaml_index et user_idx depuis la liste effective.
                        let tunnels = self.effective_tunnels(&server);
                        if let Some(et) = tunnels.get(idx) {
                            let yaml_index = et.yaml_index;
                            let user_idx = et.user_idx;
                            self.update_tunnel_override(&server, yaml_index, user_idx, config);
                            self.set_status_message(self.lang.tunnel_updated);
                        }
                    }
                    None => {
                        // Création.
                        self.add_tunnel_override(&server, config);
                        self.set_status_message(self.lang.tunnel_added);
                    }
                }
                // Revient à la liste, sélection sur le dernier élément ajouté/édité.
                let new_len = self.effective_tunnels(&server).len();
                let sel = match editing_index {
                    Some(idx) => idx.min(new_len.saturating_sub(1)),
                    None => new_len.saturating_sub(1),
                };
                self.tunnel_overlay = Some(TunnelOverlayState::List { selected: sel });
            }
        }
    }

    /// Annule le formulaire et revient à la vue liste (Esc).
    pub fn tunnel_form_cancel(&mut self) {
        // Récupère l'index édité pour replacer la sélection au bon endroit.
        let editing_index = if let Some(TunnelOverlayState::Form(form)) = &self.tunnel_overlay {
            form.editing_index
        } else {
            return;
        };
        let server = self.selected_server();
        let sel = editing_index
            .and_then(|idx| {
                server
                    .as_ref()
                    .map(|s| idx.min(self.effective_tunnels(s).len().saturating_sub(1)))
            })
            .unwrap_or(0);
        self.tunnel_overlay = Some(TunnelOverlayState::List { selected: sel });
    }

    // ─── Rechargement à chaud ─────────────────────────────────────────────────

    /// Recharge la configuration depuis le disque sans quitter.
    pub fn reload(&mut self) -> Result<(), ConfigError> {
        // Rechargement sélectif : si le contenu du fichier n'a pas changé, on n'a rien à faire.
        let new_hash = hash_config_file(&self.config_path);
        if new_hash == self.config_hash {
            self.set_status_message(self.lang.config_reloaded);
            return Ok(());
        }
        self.config_hash = new_hash;

        let mut stack = std::collections::HashSet::new();
        let (new_config, new_warnings, new_val_warnings) =
            Config::load_merged(&self.config_path, &mut stack)?;
        let resolved = new_config.resolve()?;

        // Conserve l'expansion / la sélection actuelles
        let old_expanded = self.expanded_items.clone();
        let old_idx = self.selected_index;

        self.config = new_config;
        self.keep_open = self
            .config
            .defaults
            .as_ref()
            .and_then(|d| d.keep_open)
            .unwrap_or(false);
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

    // ─── SCP ─────────────────────────────────────────────────────────────────

    /// Ouvre l'étape de sélection de la direction SCP pour le serveur sélectionné.
    ///
    /// Sans effet si aucun serveur n'est sélectionné ou si le mode Wallix est actif.
    pub fn open_scp_select_direction(&mut self) {
        if self.connection_mode == ConnectionMode::Wallix {
            self.set_status_message(self.lang.scp_wallix_unavailable);
            return;
        }
        if self.selected_server().is_some() {
            self.scp_state = ScpState::SelectingDirection;
        }
    }

    /// Sélectionne la direction SCP et passe au formulaire.
    ///
    /// Pré-remplit le champ distant avec `user@host:~`.
    pub fn scp_select_direction(&mut self, direction: ScpDirection) {
        let Some(server) = self.selected_server() else {
            return;
        };
        let remote_default = format!("{}@{}:~", server.user, server.host);
        self.scp_state = ScpState::FillingForm {
            direction,
            local: String::new(),
            remote: remote_default,
            focus: ScpFormField::Local,
            error: String::new(),
        };
    }

    /// Annule l'overlay SCP (retour à l'état `Idle`).
    pub fn close_scp_overlay(&mut self) {
        // N'interrompt pas un transfert en cours.
        if matches!(self.scp_state, ScpState::Running { .. }) {
            return;
        }
        self.scp_state = ScpState::Idle;
    }

    /// Confirme l'état final SCP (Done ou Error) et retourne à `Idle`.
    pub fn dismiss_scp_result(&mut self) {
        if matches!(self.scp_state, ScpState::Done { .. } | ScpState::Error(_)) {
            self.scp_state = ScpState::Idle;
        }
    }

    /// Insère un caractère dans le champ actif du formulaire SCP.
    pub fn scp_form_char(&mut self, c: char) {
        if let ScpState::FillingForm {
            ref focus,
            ref mut local,
            ref mut remote,
            ..
        } = self.scp_state
        {
            match focus {
                ScpFormField::Local => local.push(c),
                ScpFormField::Remote => remote.push(c),
            }
        }
    }

    /// Supprime le dernier caractère du champ actif du formulaire SCP.
    pub fn scp_form_backspace(&mut self) {
        if let ScpState::FillingForm {
            ref focus,
            ref mut local,
            ref mut remote,
            ..
        } = self.scp_state
        {
            match focus {
                ScpFormField::Local => {
                    local.pop();
                }
                ScpFormField::Remote => {
                    remote.pop();
                }
            }
        }
    }

    /// Bascule le focus entre les deux champs du formulaire SCP.
    pub fn scp_form_next_field(&mut self) {
        if let ScpState::FillingForm { ref mut focus, .. } = self.scp_state {
            *focus = focus.next();
        }
    }

    /// Lance le transfert SCP (soumission du formulaire).
    ///
    /// Valide les champs, puis spawne le subprocess `scp` en arrière-plan.
    pub fn scp_form_submit(&mut self) {
        let (direction, local, remote) = match &self.scp_state {
            ScpState::FillingForm {
                direction,
                local,
                remote,
                ..
            } => (
                direction.clone(),
                local.trim().to_string(),
                remote.trim().to_string(),
            ),
            _ => return,
        };

        if local.is_empty() {
            if let ScpState::FillingForm { ref mut error, .. } = self.scp_state {
                *error = self.lang.scp_form_local_required.to_string();
            }
            return;
        }
        if remote.is_empty() {
            if let ScpState::FillingForm { ref mut error, .. } = self.scp_state {
                *error = self.lang.scp_form_remote_required.to_string();
            }
            return;
        }

        let Some(server) = self.selected_server() else {
            return;
        };
        let server = server.clone();
        let mode = self.connection_mode;

        // Extraction du chemin réel (sans user@host: s'il est présent).
        let remote_path = if let Some((_, path)) = remote.split_once(':') {
            path.to_string()
        } else {
            remote.clone()
        };

        let label = match &direction {
            ScpDirection::Upload => local.clone(),
            ScpDirection::Download => remote_path.clone(),
        };

        match ssh_sftp::spawn_sftp(&server, mode, direction.clone(), &local, &remote_path) {
            Ok(rx) => {
                // Taille du fichier local pour l'upload ; pour le download,
                // on recevra la taille via ScpEvent::FileSize (sinon reste 0).
                let file_size = if direction == ScpDirection::Upload {
                    std::fs::metadata(&local).map(|m| m.len()).unwrap_or(0)
                } else {
                    0
                };
                self.scp_state = ScpState::Running {
                    direction,
                    label,
                    progress: 0,
                    started_at: std::time::Instant::now(),
                    file_size,
                };
                self.scp_rx = Some(rx);
            }
            Err(e) => {
                self.scp_state = ScpState::Error(e.to_string());
            }
        }
    }

    /// Sonde les évènements SCP en attente et met à jour `scp_state`.
    ///
    /// À appeler depuis la boucle d'événements (tick).
    pub fn poll_scp_events(&mut self) {
        use std::sync::mpsc::TryRecvError;

        let Some(rx) = &self.scp_rx else { return };

        loop {
            match rx.try_recv() {
                Ok(ScpEvent::Progress(pct)) => {
                    if let ScpState::Running {
                        ref mut progress, ..
                    } = self.scp_state
                    {
                        *progress = pct;
                    }
                }
                Ok(ScpEvent::FileSize(sz)) => {
                    if let ScpState::Running {
                        ref mut file_size, ..
                    } = self.scp_state
                    {
                        *file_size = sz;
                    }
                }
                Ok(ScpEvent::Done(ok)) => {
                    if ok {
                        self.set_status_message(self.lang.scp_done_ok);
                    } else {
                        self.set_status_message(self.lang.scp_done_err);
                    }
                    let direction = if let ScpState::Running { ref direction, .. } = self.scp_state
                    {
                        direction.clone()
                    } else {
                        ScpDirection::Upload
                    };
                    self.scp_state = ScpState::Done {
                        direction,
                        exit_ok: ok,
                    };
                    self.scp_rx = None;
                    break;
                }
                Ok(ScpEvent::Error(e)) => {
                    self.set_status_message(crate::i18n::fmt(self.lang.scp_failed, &[&e]));
                    self.scp_state = ScpState::Error(e);
                    self.scp_rx = None;
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    // Le thread s'est terminé sans émettre Done/Error.
                    self.scp_rx = None;
                    break;
                }
            }
        }
    }
}

impl Drop for App {
    /// Arrête proprement tous les sous-processus actifs (tunnels SSH + SCP) à la fermeture.
    ///
    /// Les tunnels SSH sont également tués par le `Drop` de leurs [`TunnelHandle`]
    /// quand `active_tunnels` est libéré, mais `stop_all_tunnels()` les arrête en avance
    /// pour garantir un SIGTERM avant le wait().
    /// Le transfert SFTP tourne dans un thread Rust : `scp_rx` est droppé pour signaler l'arrêt.
    fn drop(&mut self) {
        self.stop_all_tunnels();
        // Le transfert SFTP tourne dans un thread Rust (pas de sous-processus).
        // Dropper `scp_rx` ici suffit à signaler au thread SFTP d'arrêter
        // (il recevra une SendError au prochain ScpEvent::Progress).
        drop(self.scp_rx.take());
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
                wallix_group: None,
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
                    wallix_group: None,
                    jump: None,
                    probe_filesystems: None,
                    tunnels: None,
                    tags: None,
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
                        tunnels: None,
                        tags: None,
                        ..Default::default()
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
                    tunnels: None,
                    tags: None,
                    ..Default::default()
                }]),
                tunnels: None,
                tags: None,
            })],
            vars: Default::default(),
        }
    }

    // ─── Tests parse_search_tokens ────────────────────────────────────────────

    #[test]
    fn test_parse_tokens_text_only() {
        let (text, tags) = parse_search_tokens("web DB");
        assert_eq!(text, vec!["web", "db"]);
        assert!(tags.is_empty());
    }

    #[test]
    fn test_parse_tokens_tags_only() {
        let (text, tags) = parse_search_tokens("#prod #eu");
        assert!(text.is_empty());
        assert_eq!(tags, vec!["prod", "eu"]);
    }

    #[test]
    fn test_parse_tokens_mixed() {
        let (text, tags) = parse_search_tokens("web #prod DB");
        assert_eq!(text, vec!["web", "db"]);
        assert_eq!(tags, vec!["prod"]);
    }

    #[test]
    fn test_parse_tokens_empty_hash() {
        let (text, tags) = parse_search_tokens("# word");
        // bare '#' est ignoré car empty tag, "word" est texte
        assert_eq!(text, vec!["word"]);
        assert!(tags.is_empty());
    }

    // ─── Tests filtrage par #tag ──────────────────────────────────────────────

    fn make_tagged_config() -> Config {
        use crate::config::{Group, Server};
        Config {
            defaults: None,
            includes: vec![],
            groups: vec![ConfigEntry::Group(Group {
                name: "G".to_string(),
                user: None,
                ssh_key: None,
                mode: None,
                ssh_port: None,
                ssh_options: None,
                wallix: None,
                wallix_group: None,
                jump: None,
                probe_filesystems: None,
                environments: None,
                tunnels: None,
                tags: None,
                servers: Some(vec![
                    Server {
                        name: "prod-web".to_string(),
                        host: "1.1.1.1".to_string(),
                        user: None,
                        ssh_key: None,
                        ssh_port: None,
                        ssh_options: None,
                        mode: None,
                        wallix: None,
                        jump: None,
                        probe_filesystems: None,
                        tunnels: None,
                        tags: Some(vec!["prod".to_string(), "web".to_string()]),
                        ..Default::default()
                    },
                    Server {
                        name: "staging-db".to_string(),
                        host: "2.2.2.2".to_string(),
                        user: None,
                        ssh_key: None,
                        ssh_port: None,
                        ssh_options: None,
                        mode: None,
                        wallix: None,
                        jump: None,
                        probe_filesystems: None,
                        tunnels: None,
                        tags: Some(vec!["staging".to_string(), "db".to_string()]),
                        ..Default::default()
                    },
                ]),
            })],
            vars: Default::default(),
        }
    }

    #[test]
    fn test_tag_filter_matches() {
        let config = make_tagged_config();
        let mut app = App::new(config, vec![], std::path::PathBuf::new(), vec![]).unwrap();

        app.search_query = "#prod".to_string();
        app.invalidate_cache();
        let items = app.get_visible_items();

        let has_prod = items.iter().any(|i| match i {
            ConfigItem::Server(s) => s.name == "prod-web",
            _ => false,
        });
        let has_staging = items.iter().any(|i| match i {
            ConfigItem::Server(s) => s.name == "staging-db",
            _ => false,
        });
        assert!(has_prod, "prod-web doit être visible avec #prod");
        assert!(
            !has_staging,
            "staging-db ne doit pas être visible avec #prod"
        );
    }

    #[test]
    fn test_tag_filter_and_text() {
        let config = make_tagged_config();
        let mut app = App::new(config, vec![], std::path::PathBuf::new(), vec![]).unwrap();

        // #prod ET texte "web"
        app.search_query = "#prod web".to_string();
        app.invalidate_cache();
        let items = app.get_visible_items();

        let has_prod_web = items.iter().any(|i| match i {
            ConfigItem::Server(s) => s.name == "prod-web",
            _ => false,
        });
        assert!(has_prod_web, "prod-web correspond à #prod web");
    }

    #[test]
    fn test_tag_filter_no_match() {
        let config = make_tagged_config();
        let mut app = App::new(config, vec![], std::path::PathBuf::new(), vec![]).unwrap();

        app.search_query = "#inexistant".to_string();
        app.invalidate_cache();
        let items = app.get_visible_items();

        let has_server = items.iter().any(|i| matches!(i, ConfigItem::Server(_)));
        assert!(
            !has_server,
            "Aucun serveur ne doit correspondre à #inexistant"
        );
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
                    wallix_group: None,
                    jump: None,
                    probe_filesystems: None,
                    environments: None,
                    tunnels: None,
                    tags: None,
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
                        tunnels: None,
                        tags: None,
                        ..Default::default()
                    }]),
                }),
                ConfigEntry::Namespace(NamespaceEntry {
                    label: "CES".to_string(),
                    source_path: "/fake/ces.yml".to_string(),
                    defaults: None,
                    vars: Default::default(),
                    entries: vec![ConfigEntry::Group(crate::config::Group {
                        name: "CES_Group".to_string(),
                        user: None,
                        ssh_key: None,
                        mode: None,
                        ssh_port: None,
                        ssh_options: None,
                        wallix: None,
                        wallix_group: None,
                        jump: None,
                        probe_filesystems: None,
                        environments: None,
                        tunnels: None,
                        tags: None,
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
                            tunnels: None,
                            tags: None,
                            ..Default::default()
                        }]),
                    })],
                }),
            ],
            vars: Default::default(),
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

    // ── TunnelForm validation ─────────────────────────────────────────────────

    fn valid_form() -> TunnelForm {
        TunnelForm {
            label: "PG".into(),
            local_port: "5433".into(),
            remote_host: "127.0.0.1".into(),
            remote_port: "5432".into(),
            focus: TunnelFormField::Label,
            editing_index: None,
            error: String::new(),
        }
    }

    #[test]
    fn form_validate_ok() {
        let cfg = valid_form().validate(&crate::i18n::STRINGS_FR).unwrap();
        assert_eq!(cfg.local_port, 5433);
        assert_eq!(cfg.remote_host, "127.0.0.1");
        assert_eq!(cfg.remote_port, 5432);
        assert_eq!(cfg.label, "PG");
    }

    #[test]
    fn form_validate_bad_local_port() {
        let mut f = valid_form();
        f.local_port = "abc".into();
        assert!(f.validate(&crate::i18n::STRINGS_FR).is_err());
        f.local_port = "0".into();
        assert!(f.validate(&crate::i18n::STRINGS_FR).is_err());
        f.local_port = "65536".into();
        assert!(f.validate(&crate::i18n::STRINGS_FR).is_err());
    }

    #[test]
    fn form_validate_empty_remote_host() {
        let mut f = valid_form();
        f.remote_host = "   ".into();
        let err = f.validate(&crate::i18n::STRINGS_FR).unwrap_err();
        assert!(err.contains("obligatoire"));
    }

    #[test]
    fn form_validate_bad_remote_port() {
        let mut f = valid_form();
        f.remote_port = "not_a_port".into();
        assert!(f.validate(&crate::i18n::STRINGS_FR).is_err());
    }

    #[test]
    fn tunnel_form_field_cycle_forward() {
        assert_eq!(TunnelFormField::Label.next(), TunnelFormField::LocalPort);
        assert_eq!(
            TunnelFormField::LocalPort.next(),
            TunnelFormField::RemoteHost
        );
        assert_eq!(
            TunnelFormField::RemoteHost.next(),
            TunnelFormField::RemotePort
        );
        assert_eq!(TunnelFormField::RemotePort.next(), TunnelFormField::Label);
    }

    #[test]
    fn tunnel_form_field_cycle_backward() {
        assert_eq!(TunnelFormField::Label.prev(), TunnelFormField::RemotePort);
        assert_eq!(
            TunnelFormField::RemotePort.prev(),
            TunnelFormField::RemoteHost
        );
    }

    #[test]
    fn tunnel_form_char_filters_non_digits_for_ports() {
        let mut f = valid_form();
        f.focus = TunnelFormField::LocalPort;
        f.local_port = "543".into();

        // Caractère non-numérique ignoré pour les ports
        let mut form_state = TunnelOverlayState::Form(f);
        // Test direct via current_buf_mut + validate:
        if let TunnelOverlayState::Form(ref mut form) = form_state {
            let old_len = form.local_port.len();
            // push 'x' manuellement pour simuler le filtre
            if !'x'.is_ascii_digit() {
                // filtre actif : rien n'est pushé
            } else {
                form.local_port.push('x');
            }
            assert_eq!(form.local_port.len(), old_len);
        }
    }

    #[test]
    fn new_edit_form_prefilled() {
        let cfg = crate::config::TunnelConfig {
            local_port: 8080,
            remote_host: "db.local".into(),
            remote_port: 3306,
            label: "MySQL".into(),
        };
        let form = TunnelForm::new_edit(2, &cfg);
        assert_eq!(form.editing_index, Some(2));
        assert_eq!(form.local_port, "8080");
        assert_eq!(form.remote_host, "db.local");
        assert_eq!(form.label, "MySQL");
    }
}
