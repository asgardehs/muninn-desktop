//! Integration tests for the Runestones module: load/save, evaluate, and
//! cell writeback.

use std::path::Path;

use muninn_core::query::Value;
use muninn_core::runestones;
use muninn_core::vault::Vault;

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, contents).unwrap();
}

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
  priority: {type: integer}
computed:
  is_open: "status = 'active'"
---
A task.
"#,
    );

    write(
        &root.join(".muninn/runestones/active-work.yaml"),
        r#"name: Active Work
description: Active tasks, highest priority first
source:
  types: [task]
  filter: "status = 'active'"
columns:
  - field: title
    width: 200
  - field: priority
    sort: desc
  - field: title_upper
    computed: "UPPER(title)"
"#,
    );

    write(
        &root.join("task-a.md"),
        "---\ntitle: Task A\ntype: task\nstatus: active\npriority: 3\n---\n",
    );
    write(
        &root.join("task-b.md"),
        "---\ntitle: Task B\ntype: task\nstatus: active\npriority: 1\n---\n",
    );
    write(
        &root.join("task-c.md"),
        "---\ntitle: Task C\ntype: task\nstatus: done\npriority: 5\n---\n",
    );

    tmp
}

#[test]
fn load_all_picks_up_yaml_files() {
    let tmp = make_vault();
    let all = runestones::load_all(tmp.path()).unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].name, "Active Work");
    assert_eq!(all[0].source.types, vec!["task"]);
}

#[test]
fn load_by_name_matches_name_field() {
    let tmp = make_vault();
    let rs = runestones::load_by_name(tmp.path(), "Active Work").unwrap();
    assert_eq!(rs.columns.len(), 3);
}

#[test]
fn load_by_name_falls_back_to_filename_stem() {
    let tmp = make_vault();
    let rs = runestones::load_by_name(tmp.path(), "active-work").unwrap();
    assert_eq!(rs.name, "Active Work");
}

#[test]
fn evaluate_filters_and_sorts() {
    let tmp = make_vault();
    let vault = Vault::open(tmp.path()).unwrap();
    let rs = runestones::load_by_name(tmp.path(), "Active Work").unwrap();
    let view = runestones::evaluate(&vault, &rs).unwrap();

    assert_eq!(view.rows.len(), 2, "Task C is done, filtered out");
    // priority DESC puts Task A (3) before Task B (1).
    assert!(matches!(&view.rows[0].cells[0], Value::String(s) if s == "Task A"));
    assert!(matches!(&view.rows[1].cells[0], Value::String(s) if s == "Task B"));
}

#[test]
fn evaluate_exposes_computed_column() {
    let tmp = make_vault();
    let vault = Vault::open(tmp.path()).unwrap();
    let rs = runestones::load_by_name(tmp.path(), "Active Work").unwrap();
    let view = runestones::evaluate(&vault, &rs).unwrap();

    // Third column is the computed `title_upper` → UPPER(title).
    assert!(matches!(&view.rows[0].cells[2], Value::String(s) if s == "TASK A"));
}

#[test]
fn save_then_load_round_trips() {
    let tmp = make_vault();
    let mut rs = runestones::load_by_name(tmp.path(), "Active Work").unwrap();
    rs.description = Some("edited".to_string());
    let saved_path = runestones::save(tmp.path(), &rs).unwrap();
    assert!(saved_path.ends_with("active-work.yaml"));

    let reloaded = runestones::load_by_name(tmp.path(), "Active Work").unwrap();
    assert_eq!(reloaded.description.as_deref(), Some("edited"));
}

#[test]
fn update_cell_writes_frontmatter() {
    let tmp = make_vault();
    let rs = runestones::load_by_name(tmp.path(), "Active Work").unwrap();

    runestones::update_cell(
        tmp.path(),
        &rs,
        Path::new("task-a.md"),
        "priority",
        &Value::Integer(9),
    )
    .unwrap();

    let vault = Vault::open(tmp.path()).unwrap();
    let view = runestones::evaluate(&vault, &rs).unwrap();
    // Task A now has priority 9, still sorted first DESC.
    assert!(matches!(&view.rows[0].cells[0], Value::String(s) if s == "Task A"));
    assert!(matches!(&view.rows[0].cells[1], Value::Integer(9)));
}

#[test]
fn update_cell_rejects_computed_column() {
    let tmp = make_vault();
    let rs = runestones::load_by_name(tmp.path(), "Active Work").unwrap();

    let err = runestones::update_cell(
        tmp.path(),
        &rs,
        Path::new("task-a.md"),
        "title_upper",
        &Value::String("X".to_string()),
    )
    .unwrap_err();
    assert!(matches!(err, runestones::CellWriteError::ReadOnly(_)));
}
