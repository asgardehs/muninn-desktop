//! Update a single cell in a Runestone row — i.e. rewrite one frontmatter
//! field in one note.
//!
//! The Runestone is consulted only to enforce schema-level sanity: the target
//! column must exist, must not be a computed virtual column, and must not be
//! the synthetic `path` pseudo-column. The actual write walks through
//! existing frontmatter (preserving field order via `IndexMap`) and replaces
//! the value, then stitches the file back together as
//! `---\n<yaml>---\n<body>`.

use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use thiserror::Error;

use crate::query::Value;
use crate::query::writeback::{WritebackError, value_to_yaml};

use super::runestone::Runestone;

#[derive(Debug, Error)]
pub enum CellWriteError {
    #[error("runestone has no column '{0}'")]
    UnknownColumn(String),
    #[error("column '{0}' is read-only (computed or synthetic)")]
    ReadOnly(String),
    #[error("note not found: {0}")]
    NoteNotFound(PathBuf),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid frontmatter YAML: {0}")]
    InvalidYaml(#[from] serde_yaml::Error),
    #[error("value cannot be serialized to YAML: {0}")]
    Serialize(#[from] WritebackError),
}

/// Write `new_value` into frontmatter field `column` of the note at
/// `note_path` (relative to `vault_root`). Leaves the markdown body and
/// field order untouched.
pub fn update_cell(
    vault_root: &Path,
    runestone: &Runestone,
    note_path: &Path,
    column: &str,
    new_value: &Value,
) -> Result<(), CellWriteError> {
    let col = runestone
        .columns
        .iter()
        .find(|c| c.field == column)
        .ok_or_else(|| CellWriteError::UnknownColumn(column.to_string()))?;
    if !col.is_writable() {
        return Err(CellWriteError::ReadOnly(column.to_string()));
    }

    let abs_path = if note_path.is_absolute() {
        note_path.to_path_buf()
    } else {
        vault_root.join(note_path)
    };
    if !abs_path.exists() {
        return Err(CellWriteError::NoteNotFound(abs_path));
    }

    let content = std::fs::read_to_string(&abs_path)?;
    let (fm_raw, body) = crate::markdown::extract_frontmatter(&content);

    let mut fm: IndexMap<String, serde_yaml::Value> = if fm_raw.is_empty() {
        IndexMap::new()
    } else {
        serde_yaml::from_str(&fm_raw)?
    };

    let yaml_value = value_to_yaml(new_value)?;
    fm.insert(column.to_string(), yaml_value);

    let new_fm = serde_yaml::to_string(&fm)?;
    let stitched = format!("---\n{new_fm}---\n{body}");
    std::fs::write(&abs_path, stitched)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::runestone::{ColumnDef, Runestone, RunestoneSource};
    use super::*;

    fn sample_runestone() -> Runestone {
        Runestone {
            name: "Tasks".to_string(),
            description: None,
            source: RunestoneSource {
                types: vec!["task".to_string()],
                filter: None,
            },
            columns: vec![
                ColumnDef {
                    field: "title".to_string(),
                    header: None,
                    width: None,
                    sort: None,
                    hidden: false,
                    computed: None,
                },
                ColumnDef {
                    field: "days_left".to_string(),
                    header: None,
                    width: None,
                    sort: None,
                    hidden: false,
                    computed: Some("DATE_DIFF(deadline, TODAY())".to_string()),
                },
            ],
            group_by: None,
            order_by: Vec::new(),
            limit: None,
        }
    }

    #[test]
    fn rejects_unknown_column() {
        let tmp = tempfile::tempdir().unwrap();
        let err = update_cell(
            tmp.path(),
            &sample_runestone(),
            Path::new("x.md"),
            "bogus",
            &Value::Null,
        )
        .unwrap_err();
        assert!(matches!(err, CellWriteError::UnknownColumn(_)));
    }

    #[test]
    fn rejects_computed_column() {
        let tmp = tempfile::tempdir().unwrap();
        let err = update_cell(
            tmp.path(),
            &sample_runestone(),
            Path::new("x.md"),
            "days_left",
            &Value::Null,
        )
        .unwrap_err();
        assert!(matches!(err, CellWriteError::ReadOnly(_)));
    }

    #[test]
    fn updates_existing_field() {
        let tmp = tempfile::tempdir().unwrap();
        let note = tmp.path().join("t.md");
        std::fs::write(&note, "---\ntitle: old\nstatus: active\n---\nbody here\n").unwrap();

        update_cell(
            tmp.path(),
            &sample_runestone(),
            Path::new("t.md"),
            "title",
            &Value::String("new".to_string()),
        )
        .unwrap();

        let after = std::fs::read_to_string(&note).unwrap();
        assert!(after.contains("title: new"));
        assert!(after.contains("body here"));
    }
}
