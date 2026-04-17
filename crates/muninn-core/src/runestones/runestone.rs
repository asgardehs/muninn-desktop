//! Runestone definition — a saved, named view over the vault.
//!
//! Loaded from `.muninn/runestones/*.yaml`. Each Runestone targets a type and
//! pins a column list, filter, sort, and group rule. The underlying data is
//! always the vault — a Runestone is a lens, not a copy.

use serde::{Deserialize, Serialize};

/// A saved Runestone view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Runestone {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub source: RunestoneSource,
    #[serde(default)]
    pub columns: Vec<ColumnDef>,
    /// Presentational grouping — column name used to insert group headers in
    /// the UI. Not a SQL `GROUP BY` (which would aggregate rows away).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    /// Runestone-level ordering. Takes precedence over per-column `sort:`
    /// hints, and can reference multiple columns in priority order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub order_by: Vec<RunestoneOrderBy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunestoneSource {
    /// Types to draw rows from. Part 1 supports exactly one entry; multi-type
    /// UNION-style sources are a follow-up.
    pub types: Vec<String>,
    /// SQL `WHERE` clause body (without the `WHERE` keyword).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDef {
    /// Column identifier. For regular columns this is a frontmatter field or
    /// a TypeDef computed-field name; for virtual columns, this is the name
    /// the cell is addressed by.
    pub field: String,
    /// Display header. Falls back to `field` if unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// Per-column sort hint used only when the Runestone has no top-level
    /// `order_by:` list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort: Option<SortDirection>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hidden: bool,
    /// Virtual column: inline SQL expression evaluated at query time. Unlike
    /// a TypeDef computed field, this lives on the Runestone and is not
    /// visible to plain `muninn query` calls.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub computed: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunestoneOrderBy {
    pub field: String,
    #[serde(default)]
    pub sort: SortDirection,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    #[default]
    Asc,
    Desc,
}

impl ColumnDef {
    /// The label used for this column in output (header or field name).
    pub fn display(&self) -> &str {
        self.header.as_deref().unwrap_or(&self.field)
    }

    /// Whether this column can be written to by `update_cell` — virtual
    /// computed columns and the path column are read-only.
    pub fn is_writable(&self) -> bool {
        self.computed.is_none() && self.field != "path"
    }
}
