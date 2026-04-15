use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::extract::{WikiLink, normalize_target};

/// Describes what a wikilink points to after resolution.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LinkTarget {
    Note(PathBuf),
    Folder(PathBuf),
    Heading { note: PathBuf, heading: String },
    Attachment(PathBuf),
}

/// Bidirectional wikilink index.
/// Maps source files to their outgoing links and targets to their incoming links.
///
/// Thread safety is provided externally via `Arc<RwLock<WikilinkIndex>>`.
#[derive(Debug)]
pub struct WikilinkIndex {
    /// Source file → outgoing wikilinks.
    forward: HashMap<PathBuf, Vec<WikiLink>>,
    /// Normalized target name → source files that link to it.
    backlinks: HashMap<String, Vec<PathBuf>>,
}

impl WikilinkIndex {
    pub fn new() -> Self {
        Self {
            forward: HashMap::new(),
            backlinks: HashMap::new(),
        }
    }

    /// Replace the wikilinks for a given source file.
    pub fn update(&mut self, source_file: PathBuf, links: Vec<WikiLink>) {
        self.remove_backlinks(&source_file);

        for link in &links {
            let normalized = normalize_target(&link.target);
            self.backlinks
                .entry(normalized)
                .or_default()
                .push(source_file.clone());
        }

        self.forward.insert(source_file, links);
    }

    /// Remove all index entries for a file.
    pub fn remove(&mut self, source_file: &PathBuf) {
        self.remove_backlinks(source_file);
        self.forward.remove(source_file);
    }

    /// Get all wikilinks in the given source file.
    pub fn forward_links(&self, source_file: &Path) -> &[WikiLink] {
        self.forward.get(source_file).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Get all files that link to the given target name.
    pub fn backlinks_for(&self, target: &Path) -> Vec<PathBuf> {
        let target_name = target
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let normalized = normalize_target(target_name);

        self.backlinks
            .get(&normalized)
            .cloned()
            .unwrap_or_default()
    }

    /// Get all files that link to the given normalized target string.
    pub fn backlinks_for_target(&self, normalized_target: &str) -> Vec<PathBuf> {
        self.backlinks
            .get(normalized_target)
            .cloned()
            .unwrap_or_default()
    }

    /// Get all known source files.
    pub fn sources(&self) -> Vec<&PathBuf> {
        self.forward.keys().collect()
    }

    /// Get all known target names.
    pub fn all_targets(&self) -> Vec<&String> {
        self.backlinks.keys().collect()
    }

    fn remove_backlinks(&mut self, source_file: &PathBuf) {
        if let Some(old_links) = self.forward.get(source_file) {
            for link in old_links {
                let normalized = normalize_target(&link.target);
                if let Some(sources) = self.backlinks.get_mut(&normalized) {
                    sources.retain(|s| s != source_file);
                    if sources.is_empty() {
                        self.backlinks.remove(&normalized);
                    }
                }
            }
        }
    }
}

impl Default for WikilinkIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::extract;

    #[test]
    fn update_and_query_forward() {
        let mut index = WikilinkIndex::new();
        let links = extract("See [[other note]] and [[ref]].");
        let source = PathBuf::from("test.md");
        index.update(source.clone(), links);

        let fwd = index.forward_links(&source);
        assert_eq!(fwd.len(), 2);
    }

    #[test]
    fn backlinks_query() {
        let mut index = WikilinkIndex::new();
        let links = extract("Links to [[target]].");
        index.update(PathBuf::from("source.md"), links);

        let bl = index.backlinks_for(&PathBuf::from("target.md"));
        assert_eq!(bl.len(), 1);
        assert_eq!(bl[0], PathBuf::from("source.md"));
    }

    #[test]
    fn backlinks_multiple_sources() {
        let mut index = WikilinkIndex::new();
        index.update(PathBuf::from("a.md"), extract("[[target]]"));
        index.update(PathBuf::from("b.md"), extract("[[target]]"));

        let bl = index.backlinks_for(&PathBuf::from("target.md"));
        assert_eq!(bl.len(), 2);
    }

    #[test]
    fn update_replaces_old_links() {
        let mut index = WikilinkIndex::new();
        let source = PathBuf::from("note.md");

        index.update(source.clone(), extract("[[old-target]]"));
        assert_eq!(index.backlinks_for_target("old-target").len(), 1);

        index.update(source.clone(), extract("[[new-target]]"));
        assert!(index.backlinks_for_target("old-target").is_empty());
        assert_eq!(index.backlinks_for_target("new-target").len(), 1);
    }

    #[test]
    fn remove_clears_entries() {
        let mut index = WikilinkIndex::new();
        let source = PathBuf::from("note.md");
        index.update(source.clone(), extract("[[target]]"));

        index.remove(&source);
        assert!(index.forward_links(&source).is_empty());
        assert!(index.backlinks_for_target("target").is_empty());
    }

    #[test]
    fn case_insensitive_backlinks() {
        let mut index = WikilinkIndex::new();
        index.update(PathBuf::from("a.md"), extract("[[Target]]"));
        index.update(PathBuf::from("b.md"), extract("[[target]]"));

        let bl = index.backlinks_for(&PathBuf::from("target.md"));
        assert_eq!(bl.len(), 2);
    }
}
