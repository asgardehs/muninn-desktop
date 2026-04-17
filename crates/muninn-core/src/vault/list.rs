use std::collections::HashMap;
use std::path::PathBuf;

/// A concise note summary for listing.
#[derive(Debug, Clone)]
pub struct NoteSummary {
    pub path: PathBuf,
    pub title: String,
    pub note_type: Option<String>,
    pub tags: Vec<String>,
}

/// Filter for listing notes.
#[derive(Debug, Clone, Default)]
pub struct NoteFilter {
    /// Filter by type name.
    pub note_type: Option<String>,
    /// Filter by tag (note must have this tag).
    pub tag: Option<String>,
    /// Filter by title substring (case-insensitive).
    pub title_contains: Option<String>,
    /// Filter by frontmatter field values.
    pub field_filters: HashMap<String, String>,
}

impl NoteFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_type(mut self, note_type: &str) -> Self {
        self.note_type = Some(note_type.to_string());
        self
    }

    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tag = Some(tag.to_string());
        self
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.title_contains = Some(title.to_string());
        self
    }

    /// Check if a note passes this filter.
    pub fn matches(
        &self,
        frontmatter: &HashMap<String, serde_yaml::Value>,
        title: &str,
        tags: &[String],
    ) -> bool {
        // Type filter.
        if let Some(ref ft) = self.note_type {
            let note_type = frontmatter
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !note_type.eq_ignore_ascii_case(ft) {
                return false;
            }
        }

        // Tag filter.
        if let Some(ref ft) = self.tag {
            let ft_lower = ft.to_lowercase();
            if !tags.iter().any(|t| t.to_lowercase() == ft_lower) {
                return false;
            }
        }

        // Title filter.
        if let Some(ref ft) = self.title_contains
            && !title.to_lowercase().contains(&ft.to_lowercase())
        {
            return false;
        }

        // Field filters.
        for (key, expected) in &self.field_filters {
            match frontmatter.get(key) {
                Some(val) => {
                    let val_str = match val.as_str() {
                        Some(s) => s.to_string(),
                        None => format!("{:?}", val),
                    };
                    if !val_str.eq_ignore_ascii_case(expected) {
                        return false;
                    }
                }
                None => return false,
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fm(pairs: Vec<(&str, &str)>) -> HashMap<String, serde_yaml::Value> {
        pairs
            .into_iter()
            .map(|(k, v)| (k.to_string(), serde_yaml::Value::String(v.to_string())))
            .collect()
    }

    #[test]
    fn no_filter_matches_all() {
        let filter = NoteFilter::new();
        assert!(filter.matches(&HashMap::new(), "any title", &[]));
    }

    #[test]
    fn type_filter() {
        let filter = NoteFilter::new().with_type("journal");
        assert!(filter.matches(&fm(vec![("type", "journal")]), "x", &[]));
        assert!(!filter.matches(&fm(vec![("type", "note")]), "x", &[]));
    }

    #[test]
    fn tag_filter() {
        let filter = NoteFilter::new().with_tag("rust");
        assert!(filter.matches(&HashMap::new(), "x", &["rust".to_string()]));
        assert!(!filter.matches(&HashMap::new(), "x", &["go".to_string()]));
    }

    #[test]
    fn title_filter() {
        let filter = NoteFilter::new().with_title("test");
        assert!(filter.matches(&HashMap::new(), "My Test Note", &[]));
        assert!(!filter.matches(&HashMap::new(), "Something Else", &[]));
    }
}
