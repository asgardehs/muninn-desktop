use std::path::PathBuf;

use muninn_core::query::{Projection, Value};
use muninn_core::vault::Vault;

fn test_vault_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("testdata/test-vault")
}

#[test]
fn query_any_type_source() {
    let vault = Vault::open(test_vault_path()).unwrap();
    let rs = vault.query("SELECT title FROM note").unwrap();
    assert!(rs.rows.len() >= 3, "expected at least 3 notes, got {}", rs.rows.len());
    assert_eq!(rs.columns, vec!["title"]);
}

#[test]
fn query_filters_by_status() {
    let vault = Vault::open(test_vault_path()).unwrap();
    let rs = vault
        .query("SELECT title, status FROM note WHERE status = 'active'")
        .unwrap();
    assert!(!rs.rows.is_empty());
    for row in &rs.rows {
        let status = &row.cells[1];
        assert!(matches!(status, Value::String(s) if s == "active"));
    }
}

#[test]
fn query_order_by_title() {
    let vault = Vault::open(test_vault_path()).unwrap();
    let rs = vault.query("SELECT title FROM note ORDER BY title ASC").unwrap();
    let titles: Vec<String> = rs
        .rows
        .iter()
        .map(|r| match &r.cells[0] {
            Value::String(s) => s.clone(),
            _ => String::new(),
        })
        .collect();
    let mut sorted = titles.clone();
    sorted.sort();
    assert_eq!(titles, sorted);
}

#[test]
fn query_limit_and_offset() {
    let vault = Vault::open(test_vault_path()).unwrap();
    let full = vault.query("SELECT title FROM note ORDER BY title").unwrap();
    let limited = vault
        .query("SELECT title FROM note ORDER BY title LIMIT 1 OFFSET 1")
        .unwrap();
    assert_eq!(limited.rows.len(), 1);
    assert_eq!(limited.rows[0].cells[0].to_string(), full.rows[1].cells[0].to_string());
}

#[test]
fn query_unknown_type_errors() {
    let vault = Vault::open(test_vault_path()).unwrap();
    let err = vault.query("SELECT title FROM nonexistent_type").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("unknown type") || msg.contains("nonexistent_type"));
}

#[test]
fn query_specific_type_restricts_rows() {
    let vault = Vault::open(test_vault_path()).unwrap();
    let rs = vault.query("SELECT title FROM journal").unwrap();
    assert_eq!(rs.rows.len(), 1);
}

#[test]
fn query_wildcard_projection() {
    let vault = Vault::open(test_vault_path()).unwrap();
    let rs = vault.query("SELECT * FROM note").unwrap();
    assert_eq!(rs.columns, vec!["*"]);
    assert!(!rs.rows.is_empty());
}

#[test]
fn query_in_list() {
    let vault = Vault::open(test_vault_path()).unwrap();
    let rs = vault
        .query("SELECT title FROM note WHERE status IN ('active', 'pending')")
        .unwrap();
    assert!(!rs.rows.is_empty());
}

#[test]
fn query_rejects_insert() {
    let vault = Vault::open(test_vault_path()).unwrap();
    assert!(vault.query("INSERT INTO note VALUES (1)").is_err());
}

#[test]
fn query_alias_and_label() {
    let vault = Vault::open(test_vault_path()).unwrap();
    let rs = vault
        .query("SELECT title AS name FROM note LIMIT 1")
        .unwrap();
    assert_eq!(rs.columns, vec!["name"]);
    assert_eq!(rs.rows.len(), 1);
}

#[test]
fn query_projection_variants() {
    let q = muninn_core::query::parse_query("SELECT *, title FROM note").unwrap();
    assert_eq!(q.projections.len(), 2);
    assert!(matches!(q.projections[0], Projection::Wildcard));
    assert!(matches!(q.projections[1], Projection::Expr { .. }));
}

#[test]
fn query_count_star() {
    let vault = Vault::open(test_vault_path()).unwrap();
    let rs = vault.query("SELECT COUNT(*) AS n FROM note").unwrap();
    assert_eq!(rs.rows.len(), 1);
    match &rs.rows[0].cells[0] {
        Value::Integer(n) => assert!(*n >= 3),
        other => panic!("expected integer, got {:?}", other),
    }
}

#[test]
fn query_group_by_status() {
    let vault = Vault::open(test_vault_path()).unwrap();
    let rs = vault
        .query("SELECT status, COUNT(*) AS n FROM note GROUP BY status")
        .unwrap();
    assert!(!rs.rows.is_empty());
    // Every row should have a COUNT >= 1.
    for row in &rs.rows {
        match &row.cells[1] {
            Value::Integer(n) => assert!(*n >= 1),
            other => panic!("expected integer count, got {:?}", other),
        }
    }
}

#[test]
fn query_having_filters_groups() {
    let vault = Vault::open(test_vault_path()).unwrap();
    let rs = vault
        .query("SELECT status, COUNT(*) AS n FROM note GROUP BY status HAVING COUNT(*) >= 2")
        .unwrap();
    for row in &rs.rows {
        match &row.cells[1] {
            Value::Integer(n) => assert!(*n >= 2),
            other => panic!("expected integer count, got {:?}", other),
        }
    }
}

#[test]
fn query_scalar_functions() {
    let vault = Vault::open(test_vault_path()).unwrap();
    let rs = vault
        .query("SELECT UPPER(title) AS t FROM note WHERE LENGTH(title) > 5 ORDER BY t")
        .unwrap();
    assert!(!rs.rows.is_empty());
    // Results should be uppercase and sorted.
    let titles: Vec<String> = rs
        .rows
        .iter()
        .map(|r| match &r.cells[0] {
            Value::String(s) => s.clone(),
            _ => panic!("expected string"),
        })
        .collect();
    for t in &titles {
        assert_eq!(t, &t.to_uppercase());
    }
    let mut sorted = titles.clone();
    sorted.sort();
    assert_eq!(titles, sorted);
}

#[test]
fn query_coalesce() {
    let vault = Vault::open(test_vault_path()).unwrap();
    let rs = vault
        .query("SELECT COALESCE(status, 'unknown') AS s FROM note")
        .unwrap();
    // No row should produce NULL after COALESCE.
    for row in &rs.rows {
        assert!(!row.cells[0].is_null());
    }
}

#[test]
fn query_aggregate_outside_group_errors() {
    let vault = Vault::open(test_vault_path()).unwrap();
    // COUNT in a WHERE clause is nonsensical — should error.
    let err = vault
        .query("SELECT title FROM note WHERE COUNT(*) > 0")
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.to_lowercase().contains("aggregate"));
}

#[test]
fn query_order_by_alias() {
    let vault = Vault::open(test_vault_path()).unwrap();
    let rs = vault
        .query("SELECT UPPER(title) AS t FROM note ORDER BY t")
        .unwrap();
    let titles: Vec<String> = rs
        .rows
        .iter()
        .map(|r| match &r.cells[0] {
            Value::String(s) => s.clone(),
            _ => String::new(),
        })
        .collect();
    let mut sorted = titles.clone();
    sorted.sort();
    assert_eq!(titles, sorted);
}

#[test]
fn query_date_add_and_today() {
    let vault = Vault::open(test_vault_path()).unwrap();
    let rs = vault
        .query("SELECT DATE_ADD(TODAY(), 7) AS d FROM note LIMIT 1")
        .unwrap();
    assert!(matches!(rs.rows[0].cells[0], Value::Date(_)));
}

#[test]
fn query_group_by_with_select_star_errors() {
    let vault = Vault::open(test_vault_path()).unwrap();
    let err = vault
        .query("SELECT * FROM note GROUP BY status")
        .unwrap_err();
    assert!(err.to_string().to_lowercase().contains("*"));
}
