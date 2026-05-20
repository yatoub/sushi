pub mod app;
pub mod config;
pub mod export;
pub mod handlers;
pub mod hooks;
pub mod i18n;
pub mod import;
pub mod probe;
pub mod ssh;
pub mod state;
pub mod ui;
pub mod wallix;

// ─── CLI ─────────────────────────────────────────────────────────────────────

/// 🍣 susshi — terminal SSH connection manager
#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Chemin vers le fichier de configuration (défaut : ~/.susshi.yml)
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<String>,

    /// Connexion directe sans TUI : [user@]host[:port]
    #[arg(long, value_name = "[USER@]HOST[:PORT]", conflicts_with_all = ["jump", "wallix"])]
    pub direct: Option<String>,

    /// Connexion via jump host sans TUI : [user@]host[:port]
    #[arg(long, value_name = "[USER@]HOST[:PORT]", conflicts_with_all = ["direct", "wallix"])]
    pub jump: Option<String>,

    /// Connexion via wallix sans TUI : [user@]host[:port]
    #[arg(long, value_name = "[USER@]HOST[:PORT]", conflicts_with_all = ["direct", "jump"])]
    pub wallix: Option<String>,

    /// Forcer un utilisateur SSH (remplace la config et le user@host)
    #[arg(short, long, value_name = "USER")]
    pub user: Option<String>,

    /// Forcer un port SSH
    #[arg(short, long, value_name = "PORT")]
    pub port: Option<u16>,

    /// Forcer une clé SSH
    #[arg(short, long, value_name = "PATH")]
    pub key: Option<String>,

    /// Activer le mode verbeux SSH (-v)
    #[arg(short, long)]
    pub verbose: bool,

    /// Valider la configuration et quitter (code 0 = OK, 1 = erreur bloquante).
    #[arg(long)]
    pub validate: bool,

    /// Importer ~/.ssh/config et générer un YAML susshi.
    #[arg(long, conflicts_with_all = ["validate", "direct", "jump", "wallix"])]
    pub import_ssh_config: bool,

    /// Chemin du fichier ssh_config à importer (défaut : ~/.ssh/config).
    #[arg(long, value_name = "FILE", requires = "import_ssh_config")]
    pub ssh_config_path: Option<String>,

    /// Fichier de sortie pour --import-ssh-config (défaut : stdout).
    #[arg(long, value_name = "FILE", requires = "import_ssh_config")]
    pub output: Option<String>,

    /// Afficher le résultat sans écrire de fichier (pour --import-ssh-config).
    #[arg(long, requires = "import_ssh_config")]
    pub dry_run: bool,

    /// Exporter la configuration vers un format externe : "ansible", "csv", "openssh".
    #[arg(long, value_name = "FORMAT", conflicts_with_all = ["validate", "direct", "jump", "wallix", "import_ssh_config"])]
    pub export: Option<String>,

    /// Fichier de sortie pour --export (défaut : stdout).
    #[arg(long = "export-output", value_name = "FILE", requires = "export")]
    pub export_output: Option<String>,

    /// Filtre pour --export : texte et/ou #tag (même syntaxe que la recherche TUI).
    #[arg(long = "export-filter", value_name = "QUERY", requires = "export")]
    pub export_filter: Option<String>,

    /// Lister tous les serveurs en JSON (compatible jq / fzf).
    #[arg(long, conflicts_with_all = ["validate", "direct", "jump", "wallix", "import_ssh_config", "export"])]
    pub list: bool,

    /// Filtre pour --list : texte et/ou #tag.
    #[arg(long = "list-filter", value_name = "QUERY", requires = "list")]
    pub list_filter: Option<String>,

    /// Exécuter une commande sur tous les serveurs d'un groupe en parallèle.
    #[arg(long, value_name = "GROUP", conflicts_with_all = ["validate", "direct", "jump", "wallix", "import_ssh_config", "export", "list"])]
    pub exec_group: Option<String>,

    /// Commande à exécuter avec --exec-group.
    #[arg(long = "exec-cmd", value_name = "CMD", requires = "exec_group")]
    pub exec_cmd: Option<String>,

    /// Timeout par hôte en secondes pour --exec-group (défaut : 30).
    #[arg(
        long = "exec-timeout",
        value_name = "SECS",
        requires = "exec_group",
        default_value = "30"
    )]
    pub exec_timeout: u64,
}
