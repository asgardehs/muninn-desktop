use std::collections::HashMap;
use std::path::Path;

use regex::Regex;
use thiserror::Error;

use super::inherit::resolve_inheritance;
use super::types::TypeDef;
use crate::markdown;

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("io error reading types: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error in type {file}: {message}")]
    Parse { file: String, message: String },
    #[error("invalid type name {name:?}: must match [a-z][a-z0-9_-]{{0,63}}")]
    InvalidName { name: String },
    #[error("type name {name:?} does not match filename {filename:?}")]
    NameMismatch { name: String, filename: String },
    #[error("duplicate type name: {0}")]
    Duplicate(String),
    #[error("inheritance error: {0}")]
    Inheritance(String),
}

/// Regex for valid type names: lowercase start, alphanumeric + hyphens + underscores, max 64 chars.
fn type_name_re() -> Regex {
    Regex::new(r"^[a-z][a-z0-9_-]{0,63}$").unwrap()
}

/// Load all type definitions from a `.muninn/types/` directory.
/// Returns a map of type name to resolved TypeDef.
pub fn load_types(types_dir: &Path) -> Result<HashMap<String, TypeDef>, LoadError> {
    let entries = match std::fs::read_dir(types_dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(HashMap::new());
        }
        Err(e) => return Err(LoadError::Io(e)),
    };

    let mut types = HashMap::new();
    let name_re = type_name_re();

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str());
        if ext != Some("md") {
            continue;
        }

        let td = load_type_file(&path)?;

        // Validate name matches filename.
        let expected_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        if td.name != expected_name {
            return Err(LoadError::NameMismatch {
                name: td.name,
                filename: expected_name.to_string(),
            });
        }

        // Validate name format.
        if !name_re.is_match(&td.name) {
            return Err(LoadError::InvalidName { name: td.name });
        }

        if types.contains_key(&td.name) {
            return Err(LoadError::Duplicate(td.name));
        }

        types.insert(td.name.clone(), td);
    }

    resolve_inheritance(&mut types).map_err(LoadError::Inheritance)?;

    Ok(types)
}

/// Load a single type definition from a markdown file.
fn load_type_file(path: &Path) -> Result<TypeDef, LoadError> {
    let content = std::fs::read_to_string(path)?;
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let (raw_fm, body) = markdown::extract_frontmatter(&content);

    if raw_fm.is_empty() {
        return Err(LoadError::Parse {
            file: filename,
            message: "no frontmatter found".to_string(),
        });
    }

    let mut td: TypeDef = serde_yaml::from_str(&raw_fm).map_err(|e| LoadError::Parse {
        file: filename.clone(),
        message: e.to_string(),
    })?;

    td.body = body;

    Ok(td)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_type_file(dir: &Path, name: &str, content: &str) {
        fs::write(dir.join(format!("{}.md", name)), content).unwrap();
    }

    #[test]
    fn load_single_type() {
        let tmp = TempDir::new().unwrap();
        write_type_file(
            tmp.path(),
            "note",
            "---\nname: note\ndescription: A basic note\nfields:\n  title:\n    type: string\n    required: true\n  tags:\n    type: list\n    items:\n      type: string\n---\nA basic note type.\n",
        );

        let types = load_types(tmp.path()).unwrap();
        assert_eq!(types.len(), 1);
        let note = &types["note"];
        assert_eq!(note.name, "note");
        assert!(note.fields.contains_key("title"));
        assert!(note.fields["title"].required);
    }

    #[test]
    fn reject_invalid_name() {
        let tmp = TempDir::new().unwrap();
        write_type_file(tmp.path(), "Bad-Name", "---\nname: Bad-Name\n---\n");

        let err = load_types(tmp.path()).unwrap_err();
        assert!(matches!(err, LoadError::InvalidName { .. }));
    }

    #[test]
    fn reject_name_mismatch() {
        let tmp = TempDir::new().unwrap();
        write_type_file(tmp.path(), "note", "---\nname: journal\n---\n");

        let err = load_types(tmp.path()).unwrap_err();
        assert!(matches!(err, LoadError::NameMismatch { .. }));
    }

    #[test]
    fn empty_directory_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let types = load_types(tmp.path()).unwrap();
        assert!(types.is_empty());
    }

    #[test]
    fn nonexistent_directory_returns_empty() {
        let types = load_types(Path::new("/nonexistent/path")).unwrap();
        assert!(types.is_empty());
    }
}
