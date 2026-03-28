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

pub fn matches_search_query(query: &str, name: &str, host: &str, tags: &[String]) -> bool {
    let (text_tokens, tag_tokens) = parse_search_tokens(query);

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
}
