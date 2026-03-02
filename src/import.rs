/// Module d'import depuis `~/.ssh/config`.
///
/// Lit le fichier de configuration SSH (et ses `Include` récursifs), extrait
/// les blocs `Host` non-génériques et génère un YAML compatible susshi.
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

// ─── Types internes ──────────────────────────────────────────────────────────

/// Représente une entrée `Host` extraite du fichier ssh_config.
#[derive(Debug, Default, Clone)]
pub struct SshConfigEntry {
    /// Valeur de la directive `Host` (pattern).
    pub host_pattern: String,
    /// `HostName` si défini, sinon égal à `host_pattern`.
    pub hostname: Option<String>,
    pub user: Option<String>,
    pub port: Option<u16>,
    pub identity_file: Option<String>,
    /// Valeur brute de `ProxyJump`.
    pub proxy_jump: Option<String>,
    /// Options supplémentaires (ex : `ServerAliveInterval=60`).
    pub ssh_options: Vec<String>,
    /// Avertissement si `ProxyCommand` non-`ProxyJump` est détecté.
    pub proxy_command_warning: Option<String>,
}

impl SshConfigEntry {
    /// Retourne l'hôte effectif (`HostName` si dispo, sinon `Host`).
    pub fn effective_host(&self) -> &str {
        self.hostname.as_deref().unwrap_or(&self.host_pattern)
    }
}

/// Résultat de l'import.
pub struct ImportResult {
    pub entries: Vec<SshConfigEntry>,
    /// Avertissements non-bloquants (ProxyCommand, fichiers manquants…).
    pub warnings: Vec<String>,
}

// ─── Parser ──────────────────────────────────────────────────────────────────

/// Point d'entrée principal : parse `path` et suit les `Include` récursivement.
/// `visited` évite les cycles.
pub fn import_ssh_config(path: &Path) -> ImportResult {
    let mut entries = Vec::new();
    let mut warnings = Vec::new();
    let mut visited = HashSet::new();

    parse_file(path, &mut entries, &mut warnings, &mut visited);

    ImportResult { entries, warnings }
}

fn parse_file(
    path: &Path,
    entries: &mut Vec<SshConfigEntry>,
    warnings: &mut Vec<String>,
    visited: &mut HashSet<PathBuf>,
) {
    let canonical = match std::fs::canonicalize(path) {
        Ok(p) => p,
        Err(_) => {
            warnings.push(format!(
                "Fichier ssh_config introuvable : {}",
                path.display()
            ));
            return;
        }
    };
    if visited.contains(&canonical) {
        return;
    }
    visited.insert(canonical.clone());

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            warnings.push(format!("Impossible de lire {} : {}", path.display(), e));
            return;
        }
    };

    let parent = path.parent().unwrap_or(Path::new("."));
    let mut current: Option<SshConfigEntry> = None;

    for line in content.lines() {
        let trimmed = line.trim();

        // Ignorer commentaires et lignes vides
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Découper clé / valeur (séparateur : espace ou `=`)
        let (key, value) = match split_kv(trimmed) {
            Some(kv) => kv,
            None => continue,
        };

        let key_lc = key.to_lowercase();

        match key_lc.as_str() {
            "include" => {
                // Sauvegarder l'entrée courante avant de descendre
                if let Some(e) = current.take() {
                    push_entry(e, entries, warnings);
                }
                // Expand tilde + glob
                let expanded = shellexpand::tilde(value).into_owned();
                for inc_path in expand_glob(&expanded, parent) {
                    parse_file(&inc_path, entries, warnings, visited);
                }
            }
            "host" => {
                if let Some(e) = current.take() {
                    push_entry(e, entries, warnings);
                }
                current = Some(SshConfigEntry {
                    host_pattern: value.to_string(),
                    ..Default::default()
                });
            }
            "hostname" => {
                if let Some(ref mut e) = current {
                    e.hostname = Some(value.to_string());
                }
            }
            "user" => {
                if let Some(ref mut e) = current {
                    e.user = Some(value.to_string());
                }
            }
            "port" => {
                if let Some(ref mut e) = current {
                    e.port = value.parse().ok();
                }
            }
            "identityfile" => {
                if let Some(ref mut e) = current {
                    e.identity_file = Some(value.to_string());
                }
            }
            "proxyjump" => {
                if let Some(ref mut e) = current {
                    e.proxy_jump = Some(value.to_string());
                }
            }
            "proxycommand" => {
                // ProxyCommand non-ProxyJump → avertissement
                if let Some(ref mut e) = current {
                    e.proxy_command_warning = Some(format!(
                        "Host '{}' : ProxyCommand non supporté, ignoré (utiliser ProxyJump)",
                        e.host_pattern
                    ));
                }
            }
            "serveraliveinterval" => {
                if let Some(ref mut e) = current {
                    e.ssh_options.push(format!("ServerAliveInterval={value}"));
                }
            }
            "serveralivecountmax" => {
                if let Some(ref mut e) = current {
                    e.ssh_options.push(format!("ServerAliveCountMax={value}"));
                }
            }
            _ => {} // Directives ignorées
        }
    }

    if let Some(e) = current.take() {
        push_entry(e, entries, warnings);
    }
}

/// Pousse une entrée si elle n'est pas un wildcard.
fn push_entry(
    entry: SshConfigEntry,
    entries: &mut Vec<SshConfigEntry>,
    warnings: &mut Vec<String>,
) {
    // Ignorer Host * et patterns avec wildcards
    if entry.host_pattern.contains('*') || entry.host_pattern.contains('?') {
        return;
    }
    if let Some(ref w) = entry.proxy_command_warning {
        warnings.push(w.clone());
    }
    entries.push(entry);
}

/// Découpe `"Key Value"` ou `"Key=Value"` en `(key, value)`.
fn split_kv(s: &str) -> Option<(&str, &str)> {
    // Essayer séparateur espace d'abord
    if let Some(pos) = s.find([' ', '\t', '=']) {
        let key = s[..pos].trim();
        let value = s[pos + 1..].trim().trim_start_matches('=').trim();
        if key.is_empty() || value.is_empty() {
            return None;
        }
        Some((key, value))
    } else {
        None
    }
}

/// Résout un chemin `Include` relativement à `base` s'il est relatif.
/// Les wildcards ne sont pas étendus en v0.11.0 (retourne le chemin tel quel).
fn expand_glob(pattern: &str, base: &Path) -> Vec<PathBuf> {
    let raw = Path::new(pattern);
    let resolved = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        base.join(raw)
    };
    vec![resolved]
}

// ─── Générateur YAML ─────────────────────────────────────────────────────────

/// Convertit les entrées importées en YAML susshi.
///
/// Les serveurs sont regroupés :
/// - `Direct`  : pas de ProxyJump
/// - Un groupe par ProxyJump distinct
pub fn import_to_yaml(entries: &[SshConfigEntry]) -> String {
    let mut out = String::new();

    out.push_str("# Généré par susshi --import-ssh-config\n");
    out.push_str("# Vérifiez et ajustez avant d'utiliser.\n\n");
    out.push_str("groups:\n");

    // Partitionner : direct vs jump
    let direct: Vec<&SshConfigEntry> = entries.iter().filter(|e| e.proxy_jump.is_none()).collect();

    let mut by_jump: std::collections::BTreeMap<String, Vec<&SshConfigEntry>> =
        std::collections::BTreeMap::new();

    for e in entries.iter().filter(|e| e.proxy_jump.is_some()) {
        by_jump
            .entry(e.proxy_jump.clone().unwrap())
            .or_default()
            .push(e);
    }

    // Groupe "Direct"
    if !direct.is_empty() {
        out.push_str("  - name: \"Imported (direct)\"\n");
        out.push_str("    servers:\n");
        for e in &direct {
            write_server(&mut out, e, 6);
        }
    }

    // Groupes par jump host
    for (jump, servers) in &by_jump {
        out.push_str(&format!("  - name: \"Imported (via {})\"\n", jump));
        out.push_str("    mode: jump\n");
        // Extraire user@host du ProxyJump
        let jump_host = jump.split(',').next().unwrap_or(jump).trim();
        let (jump_user, jump_host_only) = if let Some((u, h)) = jump_host.split_once('@') {
            (Some(u), h)
        } else {
            (None, jump_host)
        };
        if let Some(u) = jump_user {
            out.push_str(&format!("    user: \"{u}\"\n"));
        }
        out.push_str(&format!("    jump:\n      - host: \"{jump_host_only}\"\n"));
        out.push_str("    servers:\n");
        for e in servers {
            write_server(&mut out, e, 6);
        }
    }

    out
}

fn write_server(out: &mut String, e: &SshConfigEntry, indent: usize) {
    let pad = " ".repeat(indent);
    let pad2 = " ".repeat(indent + 2);

    // Sanitiser le nom (remplacer les espaces par des tirets)
    let name = e.host_pattern.replace(' ', "-");
    out.push_str(&format!("{pad}- name: \"{name}\"\n"));
    out.push_str(&format!("{pad2}host: \"{}\"\n", e.effective_host()));
    if let Some(ref u) = e.user {
        out.push_str(&format!("{pad2}user: \"{u}\"\n"));
    }
    if let Some(port) = e.port {
        out.push_str(&format!("{pad2}ssh_port: {port}\n"));
    }
    if let Some(ref key) = e.identity_file {
        out.push_str(&format!("{pad2}ssh_key: \"{key}\"\n"));
    }
    if !e.ssh_options.is_empty() {
        out.push_str(&format!("{pad2}ssh_options:\n"));
        for opt in &e.ssh_options {
            out.push_str(&format!("{pad2}  - \"{opt}\"\n"));
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_temp(content: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    #[test]
    fn test_parse_basic_host() {
        let f = write_temp("Host myserver\n  HostName 10.0.0.1\n  User admin\n  Port 2222\n");
        let result = import_ssh_config(f.path());
        assert!(result.warnings.is_empty());
        assert_eq!(result.entries.len(), 1);
        let e = &result.entries[0];
        assert_eq!(e.host_pattern, "myserver");
        assert_eq!(e.effective_host(), "10.0.0.1");
        assert_eq!(e.user, Some("admin".to_string()));
        assert_eq!(e.port, Some(2222));
    }

    #[test]
    fn test_skip_wildcard() {
        let f = write_temp("Host *\n  User default\n\nHost real\n  HostName 1.2.3.4\n");
        let result = import_ssh_config(f.path());
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].host_pattern, "real");
    }

    #[test]
    fn test_proxy_jump() {
        let f = write_temp(
            "Host bastion\n  HostName jump.example.com\n\nHost prod\n  HostName 10.0.0.2\n  ProxyJump bastion\n",
        );
        let result = import_ssh_config(f.path());
        assert_eq!(result.entries.len(), 2);
        let prod = result
            .entries
            .iter()
            .find(|e| e.host_pattern == "prod")
            .unwrap();
        assert_eq!(prod.proxy_jump, Some("bastion".to_string()));
    }

    #[test]
    fn test_proxy_command_warning() {
        let f = write_temp("Host legacy\n  HostName 1.2.3.4\n  ProxyCommand ssh -W %h:%p jump\n");
        let result = import_ssh_config(f.path());
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("ProxyCommand"));
    }

    #[test]
    fn test_server_alive_interval() {
        let f = write_temp("Host monitored\n  HostName 10.0.0.5\n  ServerAliveInterval 60\n");
        let result = import_ssh_config(f.path());
        let e = &result.entries[0];
        assert!(
            e.ssh_options
                .contains(&"ServerAliveInterval=60".to_string())
        );
    }

    #[test]
    fn test_identity_file() {
        let f = write_temp("Host secure\n  HostName 10.0.0.6\n  IdentityFile ~/.ssh/prod_key\n");
        let result = import_ssh_config(f.path());
        let e = &result.entries[0];
        assert_eq!(e.identity_file, Some("~/.ssh/prod_key".to_string()));
    }

    #[test]
    fn test_import_to_yaml_direct() {
        let entries = vec![SshConfigEntry {
            host_pattern: "web-01".to_string(),
            hostname: Some("10.0.0.1".to_string()),
            user: Some("admin".to_string()),
            ..Default::default()
        }];
        let yaml = import_to_yaml(&entries);
        assert!(yaml.contains("name: \"web-01\""));
        assert!(yaml.contains("host: \"10.0.0.1\""));
        assert!(yaml.contains("user: \"admin\""));
    }

    #[test]
    fn test_import_to_yaml_jump_group() {
        let entries = vec![
            SshConfigEntry {
                host_pattern: "bastion".to_string(),
                hostname: Some("jump.example.com".to_string()),
                ..Default::default()
            },
            SshConfigEntry {
                host_pattern: "prod-api".to_string(),
                hostname: Some("10.0.0.2".to_string()),
                proxy_jump: Some("bastion".to_string()),
                ..Default::default()
            },
        ];
        let yaml = import_to_yaml(&entries);
        assert!(yaml.contains("via bastion"));
        assert!(yaml.contains("prod-api"));
    }

    #[test]
    fn test_include_recursive() {
        let sub = write_temp("Host sub-server\n  HostName 192.168.1.1\n");
        let main_content = format!(
            "Host main-server\n  HostName 10.0.0.1\n\nInclude {}\n",
            sub.path().display()
        );
        let main = write_temp(&main_content);
        let result = import_ssh_config(main.path());
        assert_eq!(result.entries.len(), 2);
        let names: Vec<_> = result
            .entries
            .iter()
            .map(|e| e.host_pattern.as_str())
            .collect();
        assert!(names.contains(&"main-server"));
        assert!(names.contains(&"sub-server"));
    }
}
