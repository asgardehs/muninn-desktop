use regex::Regex;
use std::sync::LazyLock;

/// A single wikilink found in a document.
#[derive(Debug, Clone, PartialEq)]
pub struct WikiLink {
    /// The link target (note name, folder path, or attachment).
    pub target: String,
    /// Optional heading fragment (without `#`).
    pub fragment: Option<String>,
    /// Display text if using `[[target|alias]]` syntax.
    pub alias: Option<String>,
    /// Whether this is an embed (`![[...]]`) rather than a link.
    pub is_embed: bool,
    /// Whether the target ends with `/` (folder link).
    pub is_folder_link: bool,
    /// Byte offset of `[[` (or `![[`) in the source text.
    pub start: usize,
    /// Byte offset past `]]` in the source text.
    pub end: usize,
}

/// Matches `[[target]]`, `[[target#fragment]]`, `[[target|alias]]`,
/// `[[target#fragment|alias]]`, and `![[...]]` (embed) variants.
/// Also handles folder links (`[[folder/]]`).
static WIKILINK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"!?\[\[([^\[\]|#]+?)(?:#([^\[\]|]+?))?(?:\|([^\[\]]+?))?\]\]").unwrap()
});

/// Extract all wikilinks from the given text.
pub fn extract(text: &str) -> Vec<WikiLink> {
    let mut links = Vec::new();

    for cap in WIKILINK_RE.captures_iter(text) {
        let full_match = cap.get(0).unwrap();
        let target_raw = cap.get(1).unwrap().as_str().trim();

        if target_raw.is_empty() {
            continue;
        }

        let is_embed = full_match.as_str().starts_with('!');
        let is_folder_link = target_raw.ends_with('/');

        // Clean target: remove trailing slash for folder links.
        let target = if is_folder_link {
            target_raw.trim_end_matches('/').to_string()
        } else {
            target_raw.to_string()
        };

        let fragment = cap.get(2).map(|m| m.as_str().trim().to_string());
        let alias = cap.get(3).map(|m| m.as_str().trim().to_string());

        links.push(WikiLink {
            target,
            fragment,
            alias,
            is_embed,
            is_folder_link,
            start: full_match.start(),
            end: full_match.end(),
        });
    }

    links
}

/// Returns deduplicated, normalized target names from the given text.
pub fn targets(text: &str) -> Vec<String> {
    let links = extract(text);
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();

    for link in links {
        let normalized = normalize_target(&link.target);
        if seen.insert(normalized.clone()) {
            result.push(normalized);
        }
    }

    result
}

/// Normalize a wikilink target for matching: lowercase and trim.
pub fn normalize_target(target: &str) -> String {
    target.to_lowercase().trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_link() {
        let links = extract("See [[my note]] for details.");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "my note");
        assert!(links[0].fragment.is_none());
        assert!(links[0].alias.is_none());
        assert!(!links[0].is_embed);
        assert!(!links[0].is_folder_link);
    }

    #[test]
    fn link_with_fragment() {
        let links = extract("See [[note#section]].");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "note");
        assert_eq!(links[0].fragment.as_deref(), Some("section"));
    }

    #[test]
    fn link_with_alias() {
        let links = extract("See [[note|display text]].");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "note");
        assert_eq!(links[0].alias.as_deref(), Some("display text"));
    }

    #[test]
    fn link_with_fragment_and_alias() {
        let links = extract("[[note#heading|alias]]");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "note");
        assert_eq!(links[0].fragment.as_deref(), Some("heading"));
        assert_eq!(links[0].alias.as_deref(), Some("alias"));
    }

    #[test]
    fn embed_link() {
        let links = extract("![[image.png]]");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "image.png");
        assert!(links[0].is_embed);
    }

    #[test]
    fn folder_link() {
        let links = extract("See [[projects/]] for details.");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "projects");
        assert!(links[0].is_folder_link);
    }

    #[test]
    fn folder_link_with_alias() {
        let links = extract("[[projects/|My Projects]]");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "projects");
        assert!(links[0].is_folder_link);
        assert_eq!(links[0].alias.as_deref(), Some("My Projects"));
    }

    #[test]
    fn multiple_links() {
        let links = extract("See [[note1]] and [[note2#heading]] and ![[image.png]].");
        assert_eq!(links.len(), 3);
    }

    #[test]
    fn empty_target_skipped() {
        let links = extract("[[]]");
        assert!(links.is_empty());
    }

    #[test]
    fn targets_deduplicated() {
        let t = targets("[[Note]] and [[note]] again");
        assert_eq!(t.len(), 1);
        assert_eq!(t[0], "note");
    }

    #[test]
    fn normalize_target_lowercase_trim() {
        assert_eq!(normalize_target("  My Note "), "my note");
    }
}
