//! Evaluate a Runestone against a vault — turn the saved YAML definition
//! into a materialized grid of rows. The underlying engine is the existing
//! `query` module: a Runestone compiles to SQL, runs through `Vault::query`,
//! and returns rows the UI (or CLI) can render as a spreadsheet.

use thiserror::Error;

use crate::query::QueryResultRow;
use crate::vault::{Vault, VaultError};

use super::runestone::{ColumnDef, Runestone, SortDirection};

#[derive(Debug, Error)]
pub enum ViewError {
    #[error("runestone '{name}' has {count} source types; Part 1 supports exactly one")]
    UnsupportedMultiType { name: String, count: usize },
    #[error("runestone '{name}' has no source type")]
    NoSourceType { name: String },
    #[error("duplicate column field in runestone: {0}")]
    DuplicateColumn(String),
    #[error("invalid identifier in column: {0}")]
    InvalidIdentifier(String),
    #[error("query error: {0}")]
    Query(#[from] VaultError),
}

/// Fully evaluated Runestone: column definitions plus materialized rows.
#[derive(Debug, Clone)]
pub struct RunestoneView {
    pub name: String,
    pub description: Option<String>,
    pub columns: Vec<ColumnDef>,
    pub group_by: Option<String>,
    pub rows: Vec<QueryResultRow>,
}

/// Evaluate a Runestone against the vault, returning rows in column order.
pub fn evaluate(vault: &Vault, runestone: &Runestone) -> Result<RunestoneView, ViewError> {
    validate(runestone)?;
    let sql = build_sql(runestone)?;
    let result = vault.query(&sql)?;

    // Re-order cells so they match the declared column order even if the user
    // stuck computed columns or hidden ones in the middle of the list. The
    // SQL we build matches column order exactly, so no reorder needed — this
    // is just the place to shim in grouping / hidden filtering if we want to.
    let visible_cols: Vec<ColumnDef> = runestone
        .columns
        .iter()
        .filter(|c| !c.hidden)
        .cloned()
        .collect();

    Ok(RunestoneView {
        name: runestone.name.clone(),
        description: runestone.description.clone(),
        columns: visible_cols,
        group_by: runestone.group_by.clone(),
        rows: result.rows,
    })
}

fn validate(runestone: &Runestone) -> Result<(), ViewError> {
    if runestone.source.types.is_empty() {
        return Err(ViewError::NoSourceType {
            name: runestone.name.clone(),
        });
    }
    if runestone.source.types.len() > 1 {
        return Err(ViewError::UnsupportedMultiType {
            name: runestone.name.clone(),
            count: runestone.source.types.len(),
        });
    }

    let mut seen = std::collections::HashSet::new();
    for col in &runestone.columns {
        if !seen.insert(&col.field) {
            return Err(ViewError::DuplicateColumn(col.field.clone()));
        }
        if !is_valid_ident(&col.field) {
            return Err(ViewError::InvalidIdentifier(col.field.clone()));
        }
    }

    Ok(())
}

fn is_valid_ident(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Translate a Runestone into a SELECT statement. Virtual computed columns
/// become aliased expressions; the filter slot becomes `WHERE`; per-column or
/// top-level sorts become `ORDER BY`.
pub(crate) fn build_sql(runestone: &Runestone) -> Result<String, ViewError> {
    let type_name = &runestone.source.types[0];

    let visible: Vec<&ColumnDef> = runestone.columns.iter().filter(|c| !c.hidden).collect();

    let mut parts: Vec<String> = Vec::with_capacity(visible.len());
    if visible.is_empty() {
        // Empty Runestone column list → SELECT * so the underlying result
        // still has something to show.
        parts.push("*".to_string());
    } else {
        for col in &visible {
            match &col.computed {
                Some(expr) => parts.push(format!("({expr}) AS {}", col.field)),
                None => parts.push(col.field.clone()),
            }
        }
    }

    let mut sql = format!("SELECT {} FROM {}", parts.join(", "), type_name);

    if let Some(filter) = &runestone.source.filter {
        sql.push_str(" WHERE ");
        sql.push_str(filter.trim());
    }

    let order_parts = collect_order(runestone);
    if !order_parts.is_empty() {
        sql.push_str(" ORDER BY ");
        sql.push_str(&order_parts.join(", "));
    }

    if let Some(limit) = runestone.limit {
        sql.push_str(&format!(" LIMIT {limit}"));
    }

    Ok(sql)
}

fn collect_order(runestone: &Runestone) -> Vec<String> {
    if !runestone.order_by.is_empty() {
        return runestone
            .order_by
            .iter()
            .map(|o| format!("{} {}", o.field, sort_sql(o.sort)))
            .collect();
    }
    runestone
        .columns
        .iter()
        .filter_map(|c| c.sort.map(|s| format!("{} {}", c.field, sort_sql(s))))
        .collect()
}

fn sort_sql(dir: SortDirection) -> &'static str {
    match dir {
        SortDirection::Asc => "ASC",
        SortDirection::Desc => "DESC",
    }
}

#[cfg(test)]
mod tests {
    use super::super::runestone::{
        ColumnDef, Runestone, RunestoneOrderBy, RunestoneSource, SortDirection,
    };
    use super::*;

    fn basic_runestone() -> Runestone {
        Runestone {
            name: "Active Work".to_string(),
            description: None,
            source: RunestoneSource {
                types: vec!["task".to_string()],
                filter: Some("status = 'active'".to_string()),
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
                    field: "priority".to_string(),
                    header: None,
                    width: None,
                    sort: Some(SortDirection::Desc),
                    hidden: false,
                    computed: None,
                },
            ],
            group_by: None,
            order_by: Vec::new(),
            limit: Some(50),
        }
    }

    #[test]
    fn builds_basic_sql() {
        let rs = basic_runestone();
        let sql = build_sql(&rs).unwrap();
        assert_eq!(
            sql,
            "SELECT title, priority FROM task WHERE status = 'active' ORDER BY priority DESC LIMIT 50"
        );
    }

    #[test]
    fn computed_column_becomes_aliased_expression() {
        let mut rs = basic_runestone();
        rs.columns.push(ColumnDef {
            field: "days_left".to_string(),
            header: None,
            width: None,
            sort: None,
            hidden: false,
            computed: Some("DATE_DIFF(deadline, TODAY())".to_string()),
        });
        let sql = build_sql(&rs).unwrap();
        assert!(sql.contains("(DATE_DIFF(deadline, TODAY())) AS days_left"));
    }

    #[test]
    fn top_level_order_wins_over_column_sort() {
        let mut rs = basic_runestone();
        rs.order_by = vec![RunestoneOrderBy {
            field: "updated".to_string(),
            sort: SortDirection::Desc,
        }];
        let sql = build_sql(&rs).unwrap();
        assert!(sql.contains("ORDER BY updated DESC"));
        assert!(!sql.contains("priority DESC"));
    }

    #[test]
    fn rejects_multi_type_source() {
        let mut rs = basic_runestone();
        rs.source.types.push("project".to_string());
        let err = validate(&rs).unwrap_err();
        assert!(matches!(err, ViewError::UnsupportedMultiType { .. }));
    }

    #[test]
    fn rejects_duplicate_column() {
        let mut rs = basic_runestone();
        rs.columns.push(ColumnDef {
            field: "title".to_string(),
            header: None,
            width: None,
            sort: None,
            hidden: false,
            computed: None,
        });
        let err = validate(&rs).unwrap_err();
        assert!(matches!(err, ViewError::DuplicateColumn(_)));
    }
}
