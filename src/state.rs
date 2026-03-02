use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::config::TunnelConfig;

/// Un tunnel effectif tel qu'il sera présenté dans l'UI : fusion du YAML et des overrides.
#[derive(Debug, Clone)]
pub struct EffectiveTunnel {
    /// Configuration résultante (YAML d'origine ou override utilisateur).
    pub config: TunnelConfig,
    /// Index du tunnel dans la liste YAML du serveur (`Some(i)`),
    /// ou `None` si ce tunnel a été ajouté manuellement depuis la TUI.
    pub yaml_index: Option<usize>,
    /// Position parmi les tunnels ajoutés par l'utilisateur (uniquement quand `yaml_index = None`).
    pub user_idx: usize,
    /// `true` si un override utilisateur existe pour ce tunnel (édition ou ajout).
    pub is_overridden: bool,
}

/// Calcule la liste effective des tunnels pour un serveur donné en fusionnant
/// la liste YAML avec les overrides persistants de l'utilisateur.
///
/// Règles de fusion :
/// - Override avec `yaml_index = Some(i)` et `hidden = false` → remplace le tunnel YAML #i.
/// - Override avec `yaml_index = Some(i)` et `hidden = true`  → masque le tunnel YAML #i.
/// - Override avec `yaml_index = None`    et `hidden = false` → tunnel ajouté par l'utilisateur.
/// - Override avec `yaml_index = None`    et `hidden = true`  → ignoré (ajouté puis supprimé).
pub fn effective_tunnels_for(
    yaml_tunnels: &[TunnelConfig],
    server_key: &str,
    overrides: &[TunnelOverride],
) -> Vec<EffectiveTunnel> {
    let server_overrides: Vec<&TunnelOverride> = overrides
        .iter()
        .filter(|o| o.server_key == server_key)
        .collect();

    let mut result = Vec::new();

    // ── Tunnels YAML (potentiellement surchargés / masqués) ──
    for (i, yaml_cfg) in yaml_tunnels.iter().enumerate() {
        let override_entry = server_overrides.iter().find(|o| o.yaml_index == Some(i));

        match override_entry {
            Some(o) if o.hidden => {
                // Tunnel masqué par l'utilisateur — ne pas inclure.
            }
            Some(o) => {
                result.push(EffectiveTunnel {
                    config: o.config.clone(),
                    yaml_index: Some(i),
                    user_idx: 0,
                    is_overridden: true,
                });
            }
            None => {
                result.push(EffectiveTunnel {
                    config: yaml_cfg.clone(),
                    yaml_index: Some(i),
                    user_idx: 0,
                    is_overridden: false,
                });
            }
        }
    }

    // ── Tunnels ajoutés par l'utilisateur (yaml_index = None, hidden = false) ──
    let user_tunnels: Vec<&TunnelOverride> = server_overrides
        .iter()
        .filter(|o| o.yaml_index.is_none() && !o.hidden)
        .copied()
        .collect();

    for (user_idx, o) in user_tunnels.iter().enumerate() {
        result.push(EffectiveTunnel {
            config: o.config.clone(),
            yaml_index: None,
            user_idx,
            is_overridden: true,
        });
    }

    result
}

/// Override utilisateur pour un tunnel SSH d'un serveur donné.
///
/// Persistant dans `~/.susshi_state.json` ; fusionné par-dessus la config YAML
/// au démarrage pour ne jamais modifier le fichier source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelOverride {
    /// Clé unique du serveur (`[NS:{ns}:]Group:{g}[:Env:{e}]:Server:{name}`).
    pub server_key: String,
    /// Index d'origine dans la liste YAML du serveur (`Some(i)`) ;
    /// `None` = tunnel ajouté manuellement depuis la TUI.
    pub yaml_index: Option<usize>,
    /// Configuration effective du tunnel (peut différer du YAML en cas d'édition).
    pub config: TunnelConfig,
    /// Si `true`, le tunnel est masqué dans la TUI (supprimé par l'utilisateur)
    /// mais conservé en mémoire pour permettre une restauration via rechargement.
    pub hidden: bool,
}

/// État persistant de l'application (sauvegardé dans ~/.susshi_state.json).
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AppState {
    /// Clés des groupes/environnements actuellement développés ("Group:foo", "Env:foo:bar").
    pub expanded_items: HashSet<String>,
    /// Horodatage Unix (secondes) de la dernière connexion SSH réussie, indexé
    /// par clé de serveur (`[NS:{ns}:]Group:{g}[:Env:{e}]:Server:{name}`).
    #[serde(default)]
    pub last_seen: HashMap<String, u64>,
    /// Clés des serveurs marqués favoris.
    #[serde(default)]
    pub favorites: HashSet<String>,
    /// Si `true`, l'arbre est trié par connexion la plus récente (vue plate).
    #[serde(default)]
    pub sort_by_recent: bool,
    /// Overrides utilisateur sur les tunnels SSH (ajouts, éditions, suppressions TUI).
    /// Fusionnés par-dessus les tunnels de la config YAML au chargement.
    #[serde(default)]
    pub tunnel_overrides: Vec<TunnelOverride>,
}

fn state_path() -> PathBuf {
    let raw = shellexpand::tilde("~/.susshi_state.json");
    PathBuf::from(raw.as_ref())
}

/// Charge l'état depuis `~/.susshi_state.json`.
/// Retourne un `AppState` par défaut si le fichier est absent ou invalide.
pub fn load_state() -> AppState {
    let path = state_path();
    let Ok(content) = fs::read_to_string(&path) else {
        return AppState::default();
    };
    serde_json::from_str(&content).unwrap_or_default()
}

/// Persiste l'état dans `~/.susshi_state.json`.
/// Les erreurs d'écriture sont silencieuses (non bloquantes).
pub fn save_state(state: &AppState) {
    let path = state_path();
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = fs::write(path, json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(local: u16, remote: u16, label: &str) -> TunnelConfig {
        TunnelConfig {
            local_port: local,
            remote_host: "127.0.0.1".into(),
            remote_port: remote,
            label: label.into(),
        }
    }

    const KEY: &str = "Group:G:Server:S";

    #[test]
    fn no_overrides_returns_yaml() {
        let yaml = vec![t(5432, 5432, "pg"), t(6379, 6379, "redis")];
        let result = effective_tunnels_for(&yaml, KEY, &[]);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].config.label, "pg");
        assert_eq!(result[0].yaml_index, Some(0));
        assert!(!result[0].is_overridden);
        assert_eq!(result[1].config.label, "redis");
        assert_eq!(result[1].yaml_index, Some(1));
        assert!(!result[1].is_overridden);
    }

    #[test]
    fn override_replaces_yaml_tunnel() {
        let yaml = vec![t(5432, 5432, "pg")];
        let overrides = vec![TunnelOverride {
            server_key: KEY.into(),
            yaml_index: Some(0),
            config: t(15432, 5432, "pg-edited"),
            hidden: false,
        }];
        let result = effective_tunnels_for(&yaml, KEY, &overrides);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].config.label, "pg-edited");
        assert_eq!(result[0].config.local_port, 15432);
        assert!(result[0].is_overridden);
    }

    #[test]
    fn hidden_override_removes_yaml_tunnel() {
        let yaml = vec![t(5432, 5432, "pg"), t(6379, 6379, "redis")];
        let overrides = vec![TunnelOverride {
            server_key: KEY.into(),
            yaml_index: Some(0),
            config: t(5432, 5432, "pg"),
            hidden: true,
        }];
        let result = effective_tunnels_for(&yaml, KEY, &overrides);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].config.label, "redis");
        assert_eq!(result[0].yaml_index, Some(1));
    }

    #[test]
    fn user_tunnel_appended_after_yaml() {
        let yaml = vec![t(5432, 5432, "pg")];
        let overrides = vec![TunnelOverride {
            server_key: KEY.into(),
            yaml_index: None,
            config: t(8080, 8080, "web"),
            hidden: false,
        }];
        let result = effective_tunnels_for(&yaml, KEY, &overrides);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].yaml_index, Some(0));
        assert_eq!(result[1].yaml_index, None);
        assert_eq!(result[1].config.label, "web");
        assert_eq!(result[1].user_idx, 0);
    }

    #[test]
    fn hidden_user_tunnel_not_shown() {
        let yaml = vec![];
        let overrides = vec![TunnelOverride {
            server_key: KEY.into(),
            yaml_index: None,
            config: t(8080, 8080, "web"),
            hidden: true,
        }];
        let result = effective_tunnels_for(&yaml, KEY, &overrides);
        assert!(result.is_empty());
    }

    #[test]
    fn overrides_for_other_server_ignored() {
        let yaml = vec![t(5432, 5432, "pg")];
        let overrides = vec![TunnelOverride {
            server_key: "Group:Other:Server:X".into(),
            yaml_index: Some(0),
            config: t(9999, 9999, "wrong"),
            hidden: false,
        }];
        let result = effective_tunnels_for(&yaml, KEY, &overrides);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].config.label, "pg");
        assert!(!result[0].is_overridden);
    }

    #[test]
    fn multiple_user_tunnels_indexed_correctly() {
        let yaml = vec![];
        let overrides = vec![
            TunnelOverride {
                server_key: KEY.into(),
                yaml_index: None,
                config: t(8080, 8080, "web"),
                hidden: false,
            },
            TunnelOverride {
                server_key: KEY.into(),
                yaml_index: None,
                config: t(9090, 9090, "metrics"),
                hidden: false,
            },
        ];
        let result = effective_tunnels_for(&yaml, KEY, &overrides);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].user_idx, 0);
        assert_eq!(result[1].user_idx, 1);
    }
}
