//! Conversions between Muninn data and Rhai's `Dynamic`.
//!
//! `query::Value` → `Dynamic` is lossy for dates: they serialize as ISO
//! strings so scripts can compare with string literals and print them.
//! The inverse direction isn't needed in Phase 4 — scripts never write
//! back into the vault.

use std::collections::HashMap;

use rhai::{Array, Dynamic, Map};

use crate::markdown::Note;
use crate::query::{QueryResultSet, Value};

pub fn value_to_dynamic(v: &Value) -> Dynamic {
    match v {
        Value::Null => Dynamic::UNIT,
        Value::Bool(b) => Dynamic::from(*b),
        Value::Integer(n) => Dynamic::from(*n),
        Value::Float(f) => Dynamic::from(*f),
        Value::String(s) => Dynamic::from(s.clone()),
        Value::Date(d) => Dynamic::from(d.format("%Y-%m-%d").to_string()),
        Value::DateTime(dt) => Dynamic::from(dt.to_rfc3339()),
        Value::Time(t) => Dynamic::from(t.format("%H:%M:%S").to_string()),
        Value::List(l) => {
            let arr: Array = l.iter().map(value_to_dynamic).collect();
            Dynamic::from(arr)
        }
    }
}

/// Convert a raw YAML frontmatter map to a Rhai `Map` routed through
/// `query::Value` so dates and lists land in consistent shapes.
pub fn frontmatter_to_map(fm: &HashMap<String, serde_yaml::Value>) -> Map {
    let mut m = Map::new();
    for (k, v) in fm {
        m.insert(k.clone().into(), value_to_dynamic(&Value::from_yaml(v)));
    }
    m
}

pub fn result_set_to_array(rs: &QueryResultSet) -> Array {
    rs.rows
        .iter()
        .map(|row| {
            let mut m = Map::new();
            m.insert("path".into(), Dynamic::from(row.path.display().to_string()));
            for (col, cell) in rs.columns.iter().zip(row.cells.iter()) {
                m.insert(col.clone().into(), value_to_dynamic(cell));
            }
            Dynamic::from(m)
        })
        .collect()
}

pub fn note_to_map(note: &Note) -> Map {
    let mut m = Map::new();
    m.insert(
        "path".into(),
        Dynamic::from(note.path.display().to_string()),
    );
    m.insert("title".into(), Dynamic::from(note.title.clone()));
    m.insert("body".into(), Dynamic::from(note.body.clone()));
    let tags: Array = note.tags.iter().map(|t| Dynamic::from(t.clone())).collect();
    m.insert("tags".into(), Dynamic::from(tags));
    m.insert(
        "frontmatter".into(),
        Dynamic::from(frontmatter_to_map(&note.frontmatter)),
    );
    m
}
