//! Priority-based ordering for project-init extensions

use super::ProjectInitEntry;

/// Sort entries by priority ascending, then alphabetically by name for ties.
pub fn priority_sort(mut entries: Vec<ProjectInitEntry>) -> Vec<ProjectInitEntry> {
    entries.sort_by(|a, b| {
        a.priority
            .cmp(&b.priority)
            .then_with(|| a.extension_name.cmp(&b.extension_name))
    });
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collision::tests::make_entry;

    #[test]
    fn test_priority_sort_ascending() {
        let entries = vec![
            make_entry("agentic-qe", 50),
            make_entry("ruflo", 20),
            make_entry("gitnexus", 100),
        ];
        let sorted = priority_sort(entries);
        assert_eq!(sorted[0].extension_name, "ruflo");
        assert_eq!(sorted[1].extension_name, "agentic-qe");
        assert_eq!(sorted[2].extension_name, "gitnexus");
    }

    #[test]
    fn test_priority_sort_alphabetical_tiebreak() {
        let entries = vec![
            make_entry("zebra", 50),
            make_entry("alpha", 50),
            make_entry("middle", 50),
        ];
        let sorted = priority_sort(entries);
        assert_eq!(sorted[0].extension_name, "alpha");
        assert_eq!(sorted[1].extension_name, "middle");
        assert_eq!(sorted[2].extension_name, "zebra");
    }

    #[test]
    fn test_priority_sort_single_extension() {
        let entries = vec![make_entry("solo", 42)];
        let sorted = priority_sort(entries);
        assert_eq!(sorted.len(), 1);
        assert_eq!(sorted[0].extension_name, "solo");
    }

    #[test]
    fn test_priority_sort_empty() {
        let entries: Vec<ProjectInitEntry> = vec![];
        let sorted = priority_sort(entries);
        assert!(sorted.is_empty());
    }

    #[test]
    fn test_priority_sort_default_priority() {
        let entries = vec![
            make_entry("high", 10),
            make_entry("default", 100),
            make_entry("mid", 50),
        ];
        let sorted = priority_sort(entries);
        assert_eq!(sorted[0].extension_name, "high");
        assert_eq!(sorted[1].extension_name, "mid");
        assert_eq!(sorted[2].extension_name, "default");
    }
}
