use sindri_core::registry::ComponentEntry;

/// Search result with relevance score
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub entry: ComponentEntry,
    pub score: u32,
    pub match_field: MatchField,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatchField {
    ExactName,
    Backend,
    Tag,
    Description,
    Fuzzy,
}

/// Fuzzy search across the registry index with relevance scoring (ADR-011, Sprint 8)
///
/// Score: exact name (100) > backend:name (80) > tag (60) > description (40) > fuzzy (20)
pub fn search(query: &str, entries: &[ComponentEntry], filters: &SearchFilters) -> Vec<SearchResult> {
    let q = query.to_lowercase();
    let mut results: Vec<SearchResult> = entries
        .iter()
        .filter(|e| {
            filters.backend.as_ref().map(|b| &e.backend == b).unwrap_or(true)
        })
        .filter_map(|entry| score_entry(entry, &q))
        .collect();

    // Sort by score descending, then name ascending
    results.sort_by(|a, b| b.score.cmp(&a.score).then(a.entry.name.cmp(&b.entry.name)));
    results
}

fn score_entry(entry: &ComponentEntry, query: &str) -> Option<SearchResult> {
    let name = entry.name.to_lowercase();
    let desc = entry.description.to_lowercase();
    let addr = format!("{}:{}", entry.backend, entry.name).to_lowercase();

    if name == query {
        return Some(SearchResult { entry: entry.clone(), score: 100, match_field: MatchField::ExactName });
    }
    if addr == query {
        return Some(SearchResult { entry: entry.clone(), score: 90, match_field: MatchField::ExactName });
    }
    if name.starts_with(query) || name.contains(query) {
        return Some(SearchResult { entry: entry.clone(), score: 80, match_field: MatchField::ExactName });
    }
    if addr.contains(query) {
        return Some(SearchResult { entry: entry.clone(), score: 70, match_field: MatchField::Backend });
    }
    if desc.contains(query) {
        return Some(SearchResult { entry: entry.clone(), score: 40, match_field: MatchField::Description });
    }
    // Fuzzy: count matching chars
    let fuzzy_score = fuzzy_match(&name, query);
    if fuzzy_score > 0 {
        return Some(SearchResult { entry: entry.clone(), score: fuzzy_score as u32, match_field: MatchField::Fuzzy });
    }
    None
}

fn fuzzy_match(text: &str, query: &str) -> usize {
    let text_chars: Vec<char> = text.chars().collect();
    let query_chars: Vec<char> = query.chars().collect();
    let mut qi = 0;
    let mut score = 0;
    for &tc in &text_chars {
        if qi < query_chars.len() && tc == query_chars[qi] {
            qi += 1;
            score += 1;
        }
    }
    if qi == query_chars.len() && score > 2 { score } else { 0 }
}

#[derive(Default)]
pub struct SearchFilters {
    pub backend: Option<String>,
    pub category: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sindri_core::registry::ComponentKind;

    fn make_entry(backend: &str, name: &str, desc: &str) -> ComponentEntry {
        ComponentEntry {
            name: name.to_string(),
            backend: backend.to_string(),
            latest: "1.0.0".to_string(),
            versions: vec!["1.0.0".to_string()],
            description: desc.to_string(),
            kind: ComponentKind::Component,
            oci_ref: format!("ghcr.io/test/{}:{}", name, "1.0.0"),
            license: "MIT".to_string(),
            depends_on: Vec::new(),
        }
    }

    #[test]
    fn exact_name_scores_highest() {
        let entries = vec![
            make_entry("mise", "nodejs", "Node.js runtime"),
            make_entry("npm", "claude-code", "Claude Code CLI"),
        ];
        let results = search("nodejs", &entries, &SearchFilters::default());
        assert!(!results.is_empty());
        assert_eq!(results[0].entry.name, "nodejs");
        assert!(results[0].score >= 80);
    }

    #[test]
    fn description_match_found() {
        let entries = vec![
            make_entry("binary", "gh", "GitHub CLI tool"),
        ];
        let results = search("github", &entries, &SearchFilters::default());
        assert!(!results.is_empty());
        assert!(matches!(results[0].match_field, MatchField::Description));
    }
}
