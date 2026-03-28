use super::*;

impl App {
    /// Invalide le cache de la liste visible (a appeler apres toute modification
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
        self.items_dirty = true; // l'etat d'expansion a change
    }

    /// Replie tous les groupes, namespaces et environnements developpes.
    pub fn collapse_all(&mut self) {
        self.expanded_items.clear();
        self.selected_index = 0;
        self.items_dirty = true;
    }
}
