use super::*;

impl App {
    /// Retourne l'etat persistable de l'application (pour la sauvegarde).
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

    /// Passe en mode erreur avec le message donne.
    pub fn set_error(&mut self, msg: impl Into<String>) {
        self.app_mode = AppMode::Error(msg.into());
    }

    /// Revient au mode normal (ferme le panneau d'erreur).
    pub fn clear_error(&mut self) {
        self.app_mode = AppMode::Normal;
    }

    /// Calcule la cle unique d'un serveur (stable, independante de l'ordre de config).
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
}
