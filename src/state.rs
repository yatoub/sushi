use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

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
