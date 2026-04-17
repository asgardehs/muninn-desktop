//! Integration tests for the HTTP API. Uses `tower::ServiceExt::oneshot`
//! so requests go through the real axum Router without a listening
//! socket — fast, no port conflicts, and covers routing + extraction +
//! serialization end-to-end.

use std::path::Path;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use muninn_core::api::{AppState, router};
use muninn_core::vault::Vault;
use tower::ServiceExt;

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, contents).unwrap();
}

fn make_vault() -> tempfile::TempDir {
    let tmp = tempfile::Builder::new()
        .prefix("muninn-api-")
        .tempdir()
        .unwrap();
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
        &root.join(".muninn/runestones/active.yaml"),
        r#"name: Active
source:
  types: [task]
  filter: "status = 'active'"
columns:
  - field: title
  - field: priority
    sort: desc
"#,
    );

    write(
        &root.join("task-a.md"),
        "---\ntitle: Task A\ntype: task\nstatus: active\npriority: 3\n---\nbody A\n",
    );
    write(
        &root.join("task-b.md"),
        "---\ntitle: Task B\ntype: task\nstatus: done\npriority: 1\n---\nbody B\n",
    );

    tmp
}

fn make_app(tmp: &tempfile::TempDir) -> Router {
    let vault = Vault::open(tmp.path()).unwrap();
    router(AppState::new(Arc::new(vault)))
}

async fn body_json(response: axum::http::Response<Body>) -> serde_json::Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("bad json: {e}; bytes: {:?}", bytes))
}

async fn get(app: &Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let resp = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let json = body_json(resp).await;
    (status, json)
}

async fn json_request(
    app: &Router,
    method: &str,
    uri: &str,
    body: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let json = body_json(resp).await;
    (status, json)
}

#[tokio::test]
async fn types_list_and_get() {
    let tmp = make_vault();
    let app = make_app(&tmp);

    let (s, body) = get(&app, "/api/types").await;
    assert_eq!(s, StatusCode::OK);
    assert!(
        body["types"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "task")
    );

    let (s, body) = get(&app, "/api/types/task").await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(body["name"], "task");
    assert!(body["computed"]["is_open"].is_string());

    let (s, _) = get(&app, "/api/types/missing").await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn notes_list_filters_by_type() {
    let tmp = make_vault();
    let app = make_app(&tmp);

    let (s, body) = get(&app, "/api/notes?type=task").await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(body["notes"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn notes_read_write_delete() {
    let tmp = make_vault();
    let app = make_app(&tmp);

    // Read existing.
    let (s, body) = get(&app, "/api/notes/task-a.md").await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(body["title"], "Task A");

    // Full replace.
    let (s, _) = json_request(
        &app,
        "PUT",
        "/api/notes/task-a.md",
        serde_json::json!({
            "frontmatter": { "title": "Task A", "type": "task", "status": "done", "priority": 9 },
            "body": "rewritten\n",
        }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);

    let (_, body) = get(&app, "/api/notes/task-a.md").await;
    assert_eq!(body["frontmatter"]["priority"], 9);
    assert_eq!(body["body"].as_str().unwrap().trim(), "rewritten");

    // Delete.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/notes/task-a.md")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let (s, _) = get(&app, "/api/notes/task-a.md").await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn notes_create() {
    let tmp = make_vault();
    let app = make_app(&tmp);

    let (s, body) = json_request(
        &app,
        "POST",
        "/api/notes",
        serde_json::json!({
            "title": "Fresh Note",
            "type": "task",
            "fields": { "status": "active", "priority": 2 }
        }),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED);
    assert!(body["path"].as_str().unwrap().ends_with(".md"));
}

#[tokio::test]
async fn query_endpoint_returns_rows() {
    let tmp = make_vault();
    let app = make_app(&tmp);

    let (s, body) = json_request(
        &app,
        "POST",
        "/api/query",
        serde_json::json!({ "sql": "SELECT title, is_open FROM task ORDER BY title" }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let rows = body["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["cells"]["is_open"], true);
}

#[tokio::test]
async fn query_rejects_bad_sql() {
    let tmp = make_vault();
    let app = make_app(&tmp);

    let (s, body) = json_request(
        &app,
        "POST",
        "/api/query",
        serde_json::json!({ "sql": "DELETE FROM note" }),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "bad_request");
}

#[tokio::test]
async fn runestones_list_and_eval() {
    let tmp = make_vault();
    let app = make_app(&tmp);

    let (s, body) = get(&app, "/api/runestones").await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(body["runestones"][0]["name"], "Active");

    let (s, body) = get(&app, "/api/runestones/Active").await;
    assert_eq!(s, StatusCode::OK);
    let rows = body["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1, "only Task A is active");
    assert_eq!(rows[0]["cells"]["title"], "Task A");
}

#[tokio::test]
async fn runestone_cell_writeback() {
    let tmp = make_vault();
    let app = make_app(&tmp);

    let (s, _) = json_request(
        &app,
        "PUT",
        "/api/runestones/Active/rows/task-a.md",
        serde_json::json!({ "column": "priority", "value": 42 }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);

    let (_, body) = get(&app, "/api/notes/task-a.md").await;
    assert_eq!(body["frontmatter"]["priority"], 42);
}

#[tokio::test]
async fn search_endpoint() {
    let tmp = make_vault();
    let app = make_app(&tmp);

    let (s, body) = get(&app, "/api/search?q=Task%20A").await;
    assert_eq!(s, StatusCode::OK);
    let results = body["results"].as_array().unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0]["title"], "Task A");
}

#[tokio::test]
async fn links_graph_and_backlinks() {
    let tmp = make_vault();
    // Inject a wikilink so there's something to report.
    std::fs::write(
        tmp.path().join("task-a.md"),
        "---\ntitle: Task A\ntype: task\nstatus: active\n---\nSee [[task-b]]\n",
    )
    .unwrap();

    let app = make_app(&tmp);

    let (s, body) = get(&app, "/api/links/graph").await;
    assert_eq!(s, StatusCode::OK);
    let edges = body["edges"].as_array().unwrap();
    assert!(edges.iter().any(|e| e["to"] == "task-b"));

    let (s, body) = get(&app, "/api/links/backlinks/task-b").await;
    assert_eq!(s, StatusCode::OK);
    let bl = body["backlinks"].as_array().unwrap();
    assert!(
        bl.iter()
            .any(|p| p.as_str().unwrap().ends_with("task-a.md"))
    );
}

#[tokio::test]
async fn scripting_run_and_render() {
    let tmp = make_vault();
    let app = make_app(&tmp);

    let (s, body) = json_request(
        &app,
        "POST",
        "/api/run",
        serde_json::json!({ "code": r#"print("hello");"# }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(body["output"].as_str().unwrap().trim(), "hello");

    // Render against a note body with a muninn code block.
    std::fs::write(
        tmp.path().join("task-a.md"),
        "---\ntitle: Task A\ntype: task\nstatus: active\n---\n\n```muninn\nprint(\"rendered\");\n```\n",
    )
    .unwrap();

    let (s, body) =
        json_request(&app, "POST", "/api/render/task-a.md", serde_json::json!({})).await;
    assert_eq!(s, StatusCode::OK);
    assert!(body["rendered"].as_str().unwrap().contains("rendered"));
}

#[tokio::test]
async fn validate_single_note() {
    let tmp = make_vault();
    // Break a note so validation has something to report.
    std::fs::write(
        tmp.path().join("task-a.md"),
        "---\ntype: task\nstatus: active\n---\n",
    )
    .unwrap();

    let app = make_app(&tmp);

    let (s, body) = json_request(
        &app,
        "POST",
        "/api/validate",
        serde_json::json!({ "path": "task-a.md" }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let results = body["results"].as_array().unwrap();
    assert_eq!(results.len(), 1);
    let errs = results[0]["errors"].as_array().unwrap();
    assert!(errs.iter().any(|e| e["field"] == "title"));
}
