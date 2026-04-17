use std::path::PathBuf;
use std::sync::Arc;

use muninn_core::scripting::ScriptEngine;
use muninn_core::vault::Vault;

fn test_vault_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("testdata/test-vault")
}

fn engine() -> ScriptEngine {
    let vault = Arc::new(Vault::open(test_vault_path()).unwrap());
    ScriptEngine::new(vault)
}

#[test]
fn print_captures_output() {
    let out = engine().run(r#"print("hi");"#).unwrap();
    assert_eq!(out.text.trim(), "hi");
}

#[test]
fn query_returns_rows() {
    let script = r#"
        let rows = query("SELECT title FROM note");
        print(`rows=${rows.len()}`);
    "#;
    let out = engine().run(script).unwrap();
    assert!(out.text.starts_with("rows="));
    let n: usize = out.text.trim().trim_start_matches("rows=").parse().unwrap();
    assert!(n >= 3);
}

#[test]
fn note_reads_single() {
    let script = r#"
        let n = note("projects/plant-ops.md");
        print(n.title);
    "#;
    let out = engine().run(script).unwrap();
    assert_eq!(out.text.trim(), "Plant Operations");
}

#[test]
fn table_renders_markdown() {
    // Rhai `Map` keys are alphabetized, so headers will be `n, name` here.
    let script = r#"
        let rows = [#{ name: "a", n: 1 }, #{ name: "b", n: 2 }];
        table(rows);
    "#;
    let out = engine().run(script).unwrap();
    assert!(out.text.contains("| n | name |"));
    assert!(out.text.contains("| --- | --- |"));
    assert!(out.text.contains("| 1 | a |"));
    assert!(out.text.contains("| 2 | b |"));
}

#[test]
fn list_renders_bullets() {
    let out = engine().run(r#"list(["a", "b", "c"]);"#).unwrap();
    assert_eq!(out.text, "- a\n- b\n- c\n");
}

#[test]
fn link_wikilink_format() {
    let out = engine()
        .run(r#"print(link("projects/plant-ops.md"));"#)
        .unwrap();
    assert_eq!(out.text.trim(), "[[projects/plant-ops]]");
}

#[test]
fn json_serializes_map() {
    let out = engine()
        .run(r#"print(json(#{ a: 1, b: "x" }));"#)
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(out.text.trim()).unwrap();
    assert_eq!(parsed["a"], 1);
    assert_eq!(parsed["b"], "x");
}

#[test]
fn tags_function_returns_counts() {
    let script = r#"
        let ts = tags();
        print(`count=${ts.len()}`);
    "#;
    let out = engine().run(script).unwrap();
    assert!(out.text.contains("count="));
}

#[test]
fn runaway_loop_hits_operation_limit() {
    let script = r#"
        let i = 0;
        while true { i += 1; }
    "#;
    let err = engine().run(script).unwrap_err();
    let msg = err.to_string().to_lowercase();
    // Rhai's max_operations error or our classification — either wording is fine.
    assert!(
        msg.contains("operation") || msg.contains("eval") || msg.contains("timeout"),
        "unexpected error: {msg}"
    );
}

#[test]
fn runestone_stub_errors_with_phase_5_message() {
    let err = engine().run(r#"runestone("foo");"#).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("Phase 5"), "got: {msg}");
}

#[test]
fn note_missing_returns_error() {
    let err = engine()
        .run(r#"note("does-not-exist.md");"#)
        .unwrap_err();
    let msg = err.to_string().to_lowercase();
    assert!(msg.contains("not found") || msg.contains("no such"));
}

#[test]
fn search_returns_results_array() {
    let script = r#"
        let r = search("operations");
        print(`found=${r.len()}`);
    "#;
    let out = engine().run(script).unwrap();
    let n: usize = out
        .text
        .trim()
        .trim_start_matches("found=")
        .parse()
        .unwrap();
    assert!(n >= 1);
}

#[test]
fn backlinks_returns_paths() {
    let script = r#"
        let b = backlinks("projects/plant-ops.md");
        print(`backlinks=${b.len()}`);
    "#;
    let out = engine().run(script).unwrap();
    let n: usize = out
        .text
        .trim()
        .trim_start_matches("backlinks=")
        .parse()
        .unwrap();
    assert!(n >= 1);
}

#[test]
fn types_function_lists_defined_types() {
    let script = r#"
        let t = types();
        print(`types=${t.len()}`);
    "#;
    let out = engine().run(script).unwrap();
    let n: usize = out
        .text
        .trim()
        .trim_start_matches("types=")
        .parse()
        .unwrap();
    assert!(n >= 1);
}
