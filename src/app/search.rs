use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};

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

/// Retourne le score fuzzy combiné (name + host) pour un token textuel.
/// Score > 0 = match, 0 = pas de match.
fn token_score(matcher: &SkimMatcherV2, token: &str, name_lc: &str, host_lc: &str) -> i64 {
    let name_score = matcher.fuzzy_match(name_lc, token).unwrap_or(0);
    let host_score = matcher.fuzzy_match(host_lc, token).unwrap_or(0);
    name_score.max(host_score)
}

/// Score fuzzy agrégé pour tous les tokens textuels d'une query.
/// Retourne `None` si un token ne matche pas (AND strict).
pub(crate) fn fuzzy_score(query: &str, name: &str, host: &str) -> Option<i64> {
    let (text_tokens, _) = parse_search_tokens(query);
    if text_tokens.is_empty() {
        return Some(0);
    }
    let matcher = SkimMatcherV2::default();
    let name_lc = name.to_lowercase();
    let host_lc = host.to_lowercase();
    let mut total = 0i64;
    for token in &text_tokens {
        let score = token_score(&matcher, token, &name_lc, &host_lc);
        if score <= 0 {
            return None;
        }
        total += score;
    }
    Some(total)
}

pub fn matches_search_query(query: &str, name: &str, host: &str, tags: &[String]) -> bool {
    let (text_tokens, tag_tokens) = parse_search_tokens(query);

    // Tous les #tag doivent être présents (AND exact)
    let tags_ok = tag_tokens.iter().all(|t| {
        tags.iter()
            .any(|tag| tag.to_lowercase() == t.to_lowercase())
    });
    if !tags_ok {
        return false;
    }

    if text_tokens.is_empty() {
        return true;
    }

    // Tokens textuels : fuzzy match AND sur name ou host
    let matcher = SkimMatcherV2::default();
    let name_lc = name.to_lowercase();
    let host_lc = host.to_lowercase();
    text_tokens
        .iter()
        .all(|t| token_score(&matcher, t, &name_lc, &host_lc) > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tokens_text_only() {
        let (text, tags) = parse_search_tokens("web DB");
        assert_eq!(text, vec!["web", "db"]);
        assert!(tags.is_empty());
    }

    #[test]
    fn parse_tokens_tags_only() {
        let (text, tags) = parse_search_tokens("#prod #eu");
        assert!(text.is_empty());
        assert_eq!(tags, vec!["prod", "eu"]);
    }

    #[test]
    fn parse_tokens_mixed() {
        let (text, tags) = parse_search_tokens("web #prod DB");
        assert_eq!(text, vec!["web", "db"]);
        assert_eq!(tags, vec!["prod"]);
    }

    #[test]
    fn parse_tokens_empty_hash() {
        let (text, tags) = parse_search_tokens("# word");
        assert_eq!(text, vec!["word"]);
        assert!(tags.is_empty());
    }

    #[test]
    fn matches_query_with_tag_and_text() {
        let tags = vec!["prod".to_string(), "web".to_string()];
        assert!(matches_search_query(
            "#prod web",
            "prod-web-01",
            "198.51.100.10",
            &tags
        ));
        assert!(!matches_search_query(
            "#staging web",
            "prod-web-01",
            "198.51.100.10",
            &tags
        ));
    }

    // ── fuzzy match ──────────────────────────────────────────────────────────

    #[test]
    fn fuzzy_matches_abbreviated_name() {
        // "appm" doit trouver "app-mysql"
        assert!(matches_search_query("appm", "app-mysql", "10.0.0.1", &[]));
    }

    #[test]
    fn fuzzy_matches_abbreviated_multi_token() {
        // deux tokens flous : tous deux doivent matcher
        assert!(matches_search_query(
            "appm db",
            "app-mysql-db",
            "10.0.0.1",
            &[]
        ));
    }

    #[test]
    fn fuzzy_no_match_unrelated() {
        assert!(!matches_search_query("xyz", "app-mysql", "10.0.0.1", &[]));
    }

    #[test]
    fn fuzzy_score_returns_some_on_match() {
        assert!(fuzzy_score("appm", "app-mysql", "10.0.0.1").is_some());
    }

    #[test]
    fn fuzzy_score_returns_none_on_miss() {
        assert!(fuzzy_score("xyz", "app-mysql", "10.0.0.1").is_none());
    }

    #[test]
    fn fuzzy_score_both_match() {
        // "app" et "appm" matchent tous les deux "app-mysql"
        assert!(fuzzy_score("app", "app-mysql", "10.0.0.1").is_some());
        assert!(fuzzy_score("appm", "app-mysql", "10.0.0.1").is_some());
    }

    #[test]
    fn exact_substring_still_matches() {
        // la recherche exacte doit toujours fonctionner
        assert!(matches_search_query("mysql", "app-mysql", "10.0.0.1", &[]));
    }
}
