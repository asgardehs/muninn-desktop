//! Integration tests for Phase 5 query extensions: JOINs (cross-type and
//! self-joins), qualified column references, and TypeDef computed fields.

use std::path::Path;

use muninn_core::query::Value;
use muninn_core::vault::Vault;

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, contents).unwrap();
}

/// Build a vault with `task` and `project` types plus a few sample notes
/// linking via path strings in frontmatter.
fn make_vault() -> tempfile::TempDir {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path();

    write(
        &root.join(".muninn/types/task.md"),
        r#"---
name: task
fields:
  title: {type: string, required: true}
  status: {type: enum, values: [active, done]}
  project: {type: string}
  parent: {type: string}
  deadline: {type: date}
computed:
  is_open: "status = 'active'"
---
A task.
"#,
    );

    write(
        &root.join(".muninn/types/project.md"),
        r#"---
name: project
fields:
  title: {type: string, required: true}
  status: {type: enum, values: [active, archived]}
---
A project.
"#,
    );

    write(
        &root.join("proj-alpha.md"),
        r#"---
title: Alpha
type: project
status: active
---
"#,
    );

    write(
        &root.join("proj-beta.md"),
        r#"---
title: Beta
type: project
status: archived
---
"#,
    );

    write(
        &root.join("task-a.md"),
        r#"---
title: Task A
type: task
status: active
project: proj-alpha.md
deadline: 2026-05-01
---
"#,
    );

    write(
        &root.join("task-b.md"),
        r#"---
title: Task B
type: task
status: active
project: proj-alpha.md
parent: task-a.md
---
"#,
    );

    write(
        &root.join("task-c.md"),
        r#"---
title: Task C
type: task
status: done
project: proj-beta.md
parent: task-a.md
---
"#,
    );

    tmp
}

#[test]
fn cross_type_inner_join() {
    let tmp = make_vault();
    let vault = Vault::open(tmp.path()).unwrap();

    let rs = vault
        .query(
            "SELECT t.title, p.title, p.status \
             FROM task t JOIN project p ON t.project = p.path \
             ORDER BY t.title ASC",
        )
        .unwrap();

    assert_eq!(rs.rows.len(), 3, "3 tasks, each with a matching project");
    // task-a → Alpha/active
    assert!(matches!(&rs.rows[0].cells[0], Value::String(s) if s == "Task A"));
    assert!(matches!(&rs.rows[0].cells[1], Value::String(s) if s == "Alpha"));
    // task-c → Beta/archived
    assert!(matches!(&rs.rows[2].cells[1], Value::String(s) if s == "Beta"));
}

#[test]
fn left_join_keeps_rows_without_match() {
    let tmp = make_vault();
    let vault = Vault::open(tmp.path()).unwrap();

    // Swap the join so we LEFT JOIN tasks onto projects to find projects with
    // no tasks — proj-alpha has tasks, proj-beta has one done task.
    let rs = vault
        .query(
            "SELECT t.title FROM task t \
             LEFT JOIN project p ON t.project = p.path \
             WHERE p.title IS NULL",
        )
        .unwrap();
    assert_eq!(rs.rows.len(), 0, "every task points at a real project");

    // Corrupt one task's project and re-run.
    std::fs::write(
        tmp.path().join("task-orphan.md"),
        "---\ntitle: Orphan\ntype: task\nstatus: active\nproject: missing.md\n---\n",
    )
    .unwrap();
    let vault = Vault::open(tmp.path()).unwrap();
    let rs = vault
        .query(
            "SELECT t.title FROM task t \
             LEFT JOIN project p ON t.project = p.path \
             WHERE p.title IS NULL",
        )
        .unwrap();
    assert_eq!(rs.rows.len(), 1);
    assert!(matches!(&rs.rows[0].cells[0], Value::String(s) if s == "Orphan"));
}

#[test]
fn self_join_on_parent_task() {
    let tmp = make_vault();
    let vault = Vault::open(tmp.path()).unwrap();

    let rs = vault
        .query(
            "SELECT a.title, b.title \
             FROM task a JOIN task b ON a.parent = b.path \
             ORDER BY a.title",
        )
        .unwrap();

    // task-b and task-c both parent on task-a.
    assert_eq!(rs.rows.len(), 2);
    assert!(matches!(&rs.rows[0].cells[0], Value::String(s) if s == "Task B"));
    assert!(matches!(&rs.rows[0].cells[1], Value::String(s) if s == "Task A"));
    assert!(matches!(&rs.rows[1].cells[0], Value::String(s) if s == "Task C"));
    assert!(matches!(&rs.rows[1].cells[1], Value::String(s) if s == "Task A"));
}

#[test]
fn unqualified_reference_resolves_to_primary() {
    let tmp = make_vault();
    let vault = Vault::open(tmp.path()).unwrap();

    // Bare `title` resolves to alias `a` (primary).
    let rs = vault
        .query(
            "SELECT title FROM task a JOIN task b ON a.parent = b.path \
             ORDER BY title",
        )
        .unwrap();

    assert_eq!(rs.rows.len(), 2);
    assert!(matches!(&rs.rows[0].cells[0], Value::String(s) if s == "Task B"));
}

#[test]
fn rejects_duplicate_alias() {
    let tmp = make_vault();
    let vault = Vault::open(tmp.path()).unwrap();

    let err = vault
        .query("SELECT * FROM task t JOIN task t ON t.parent = t.path")
        .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("duplicate") || msg.contains("alias"),
        "got: {msg}"
    );
}

#[test]
fn computed_field_resolves_in_select() {
    let tmp = make_vault();
    let vault = Vault::open(tmp.path()).unwrap();

    let rs = vault
        .query("SELECT title, is_open FROM task ORDER BY title")
        .unwrap();

    assert_eq!(rs.rows.len(), 3);
    // Task A active -> true
    assert!(matches!(&rs.rows[0].cells[1], Value::Bool(true)));
    // Task C done -> false
    assert!(matches!(&rs.rows[2].cells[1], Value::Bool(false)));
}

#[test]
fn computed_field_filters_in_where() {
    let tmp = make_vault();
    let vault = Vault::open(tmp.path()).unwrap();

    let rs = vault
        .query("SELECT title FROM task WHERE is_open ORDER BY title")
        .unwrap();
    assert_eq!(rs.rows.len(), 2, "Task A + Task B are active");
}

#[test]
fn qualified_computed_field_in_join() {
    let tmp = make_vault();
    let vault = Vault::open(tmp.path()).unwrap();

    let rs = vault
        .query(
            "SELECT a.title FROM task a JOIN task b ON a.parent = b.path \
             WHERE b.is_open \
             ORDER BY a.title",
        )
        .unwrap();

    // Parent is task-a (active). Children: task-b (active), task-c (done).
    // Both match because parent is active.
    assert_eq!(rs.rows.len(), 2);
}
