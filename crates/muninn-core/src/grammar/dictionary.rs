use std::collections::HashSet;
use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DictionaryError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// A vault-local custom dictionary (`.muninn/dictionary.txt`).
/// One word per line, case-insensitive matching.
#[derive(Debug, Clone)]
pub struct Dictionary {
    words: HashSet<String>,
    path: Option<PathBuf>,
}

impl Dictionary {
    /// Create an empty dictionary.
    pub fn new() -> Self {
        Self {
            words: HashSet::new(),
            path: None,
        }
    }

    /// Load dictionary from a file. One word per line.
    pub fn load(path: &Path) -> Result<Self, DictionaryError> {
        let mut dict = Self {
            words: HashSet::new(),
            path: Some(path.to_path_buf()),
        };

        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            for line in content.lines() {
                let word = line.trim().to_lowercase();
                if !word.is_empty() && !word.starts_with('#') {
                    dict.words.insert(word);
                }
            }
        }

        Ok(dict)
    }

    /// Check if a word is in the custom dictionary.
    pub fn contains(&self, word: &str) -> bool {
        self.words.contains(&word.to_lowercase())
    }

    /// Add a word to the dictionary and save.
    pub fn add(&mut self, word: &str) -> Result<(), DictionaryError> {
        let normalized = word.trim().to_lowercase();
        if normalized.is_empty() {
            return Ok(());
        }

        self.words.insert(normalized.clone());

        if let Some(ref path) = self.path {
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)?;
            writeln!(file, "{}", normalized)?;
        }

        Ok(())
    }

    /// Get all words in the dictionary.
    pub fn words(&self) -> &HashSet<String> {
        &self.words
    }
}

impl Default for Dictionary {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn empty_dictionary() {
        let dict = Dictionary::new();
        assert!(!dict.contains("hello"));
    }

    #[test]
    fn load_dictionary_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("dictionary.txt");
        std::fs::write(&path, "muninn\nosha\nehs\n").unwrap();

        let dict = Dictionary::load(&path).unwrap();
        assert!(dict.contains("muninn"));
        assert!(dict.contains("MUNINN")); // case-insensitive
        assert!(dict.contains("osha"));
        assert!(!dict.contains("unknown"));
    }

    #[test]
    fn add_word_persists() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("dictionary.txt");

        let mut dict = Dictionary::load(&path).unwrap();
        dict.add("NewWord").unwrap();

        assert!(dict.contains("newword"));

        // Reload and verify persistence.
        let dict2 = Dictionary::load(&path).unwrap();
        assert!(dict2.contains("newword"));
    }

    #[test]
    fn skip_comments_and_blanks() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("dictionary.txt");
        std::fs::write(&path, "word1\n# comment\n\nword2\n").unwrap();

        let dict = Dictionary::load(&path).unwrap();
        assert!(dict.contains("word1"));
        assert!(dict.contains("word2"));
        assert!(!dict.contains("# comment"));
    }

    #[test]
    fn load_nonexistent_file() {
        let dict = Dictionary::load(Path::new("/nonexistent/dict.txt")).unwrap();
        assert!(dict.words().is_empty());
    }
}
