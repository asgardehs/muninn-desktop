use std::collections::HashMap;
use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("invalid YAML frontmatter: {0}")]
    InvalidFrontmatter(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// A parsed markdown note with extracted frontmatter.
#[derive(Debug, Clone)]
pub struct Note {
    pub path: PathBuf,
    pub title: String,
    pub frontmatter: HashMap<String, serde_yaml::Value>,
    pub raw_frontmatter: String,
    pub body: String,
    pub tags: Vec<String>,
}

/// Parse a markdown document from a file path and its contents.
pub fn parse_document(path: &Path, content: &str) -> Result<Note, ParseError> {
    let (raw_fm, body) = extract_frontmatter(content);

    let frontmatter: HashMap<String, serde_yaml::Value> = if raw_fm.is_empty() {
        HashMap::new()
    } else {
        serde_yaml::from_str(&raw_fm).map_err(|e| ParseError::InvalidFrontmatter(e.to_string()))?
    };

    let title = extract_title(&frontmatter, &body);
    let tags = extract_tags(&frontmatter);

    Ok(Note {
        path: path.to_path_buf(),
        title,
        frontmatter,
        raw_frontmatter: raw_fm,
        body,
        tags,
    })
}

/// Split YAML frontmatter from markdown body.
/// Frontmatter must start at the very beginning of the document with "---".
pub fn extract_frontmatter(source: &str) -> (String, String) {
    let trimmed = source.trim_start();

    if !trimmed.starts_with("---") {
        return (String::new(), source.to_string());
    }

    // Find the opening delimiter line.
    let after_open = match trimmed.strip_prefix("---") {
        Some(rest) => rest,
        None => return (String::new(), source.to_string()),
    };

    // The opening --- must be followed by a newline (or be exactly "---\n").
    let after_open = match after_open.strip_prefix('\n') {
        Some(rest) => rest,
        None if after_open.starts_with('\r') => {
            after_open.strip_prefix("\r\n").unwrap_or(after_open)
        }
        _ => return (String::new(), source.to_string()),
    };

    // Find closing "---" on its own line.
    if let Some(pos) = find_closing_delimiter(after_open) {
        let frontmatter = after_open[..pos].to_string();
        let rest = &after_open[pos..];
        // Skip past the closing --- and its newline.
        let body = rest.strip_prefix("---").unwrap_or(rest);
        let body = body
            .strip_prefix('\n')
            .or_else(|| body.strip_prefix("\r\n"))
            .unwrap_or(body);
        (frontmatter, body.to_string())
    } else {
        // No closing delimiter found — treat entire content as body.
        (String::new(), source.to_string())
    }
}

fn find_closing_delimiter(s: &str) -> Option<usize> {
    let mut pos = 0;
    for line in s.lines() {
        if line.trim() == "---" {
            return Some(pos);
        }
        // +1 for the \n (or the line itself if it's the last line without \n)
        pos += line.len() + 1;
    }
    None
}

fn extract_title(frontmatter: &HashMap<String, serde_yaml::Value>, body: &str) -> String {
    // Try frontmatter "title" field first.
    if let Some(serde_yaml::Value::String(title)) = frontmatter.get("title")
        && !title.is_empty()
    {
        return title.clone();
    }

    // Fall back to first # heading in body.
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed.strip_prefix("# ") {
            let heading = heading.trim();
            if !heading.is_empty() {
                return heading.to_string();
            }
        }
    }

    String::new()
}

fn extract_tags(frontmatter: &HashMap<String, serde_yaml::Value>) -> Vec<String> {
    match frontmatter.get("tags") {
        Some(serde_yaml::Value::Sequence(seq)) => seq
            .iter()
            .filter_map(|v| match v {
                serde_yaml::Value::String(s) => Some(s.clone()),
                _ => None,
            })
            .collect(),
        Some(serde_yaml::Value::String(s)) => {
            // Support comma-separated string form.
            s.split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect()
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_note_with_frontmatter() {
        let content =
            "---\ntitle: Test Note\ntags:\n  - rust\n  - test\n---\n# Content\n\nSome body text.\n";
        let note = parse_document(Path::new("test.md"), content).unwrap();
        assert_eq!(note.title, "Test Note");
        assert_eq!(note.tags, vec!["rust", "test"]);
        assert!(note.body.contains("Some body text."));
        assert!(!note.raw_frontmatter.is_empty());
    }

    #[test]
    fn parse_note_without_frontmatter() {
        let content = "# My Title\n\nJust a plain note.\n";
        let note = parse_document(Path::new("test.md"), content).unwrap();
        assert_eq!(note.title, "My Title");
        assert!(note.tags.is_empty());
        assert!(note.frontmatter.is_empty());
    }

    #[test]
    fn parse_empty_file() {
        let note = parse_document(Path::new("empty.md"), "").unwrap();
        assert_eq!(note.title, "");
        assert!(note.body.is_empty());
    }

    #[test]
    fn title_from_frontmatter_preferred_over_heading() {
        let content = "---\ntitle: FM Title\n---\n# Heading Title\n";
        let note = parse_document(Path::new("test.md"), content).unwrap();
        assert_eq!(note.title, "FM Title");
    }

    #[test]
    fn no_closing_delimiter_treats_all_as_body() {
        let content = "---\ntitle: Broken\n";
        let note = parse_document(Path::new("test.md"), content).unwrap();
        assert!(note.frontmatter.is_empty());
        assert!(note.body.contains("---"));
    }

    #[test]
    fn tags_as_comma_separated_string() {
        let content = "---\ntags: a, b, c\n---\n";
        let note = parse_document(Path::new("test.md"), content).unwrap();
        assert_eq!(note.tags, vec!["a", "b", "c"]);
    }

    #[test]
    fn extract_frontmatter_basic() {
        let (fm, body) = extract_frontmatter("---\nkey: value\n---\nbody\n");
        assert_eq!(fm, "key: value\n");
        assert_eq!(body, "body\n");
    }

    #[test]
    fn extract_frontmatter_no_delimiters() {
        let (fm, body) = extract_frontmatter("just plain text");
        assert!(fm.is_empty());
        assert_eq!(body, "just plain text");
    }
}
