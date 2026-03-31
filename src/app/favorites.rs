use super::*;

impl App {
    /// Indique si le serveur selectionne est un favori.
    pub fn is_selected_favorite(&mut self) -> bool {
        if let Some(s) = self.selected_server() {
            self.favorites.contains(&Self::server_key(&s))
        } else {
            false
        }
    }

    /// Bascule le statut favori du serveur selectionne.
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

    /// Enregistre une connexion pour le serveur selectionne (timestamp UNIX).
    pub fn record_connection(&mut self, server: &ResolvedServer) {
        let key = Self::server_key(server);
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.last_seen.insert(key, ts);
    }

    /// Retourne le timestamp UNIX de derniere connexion pour un serveur.
    pub fn last_seen_for(&self, server: &ResolvedServer) -> Option<u64> {
        self.last_seen.get(&Self::server_key(server)).copied()
    }
}
