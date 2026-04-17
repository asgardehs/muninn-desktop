//! End-to-end smoke test: bind the real router on a random port and make an
//! HTTP request via `reqwest`. Covers the serve/listen path that
//! `tower::ServiceExt::oneshot` skips.

use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use muninn_core::api::{AppState, router};
use muninn_core::vault::Vault;
use tokio::net::TcpListener;

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, contents).unwrap();
}

#[tokio::test]
async fn serves_types_over_tcp() {
    let tmp = tempfile::Builder::new()
        .prefix("muninn-e2e-")
        .tempdir()
        .unwrap();

    write(
        &tmp.path().join(".muninn/types/note.md"),
        "---\nname: note\nfields:\n  title: {type: string, required: true}\n---\nA note.\n",
    );

    let vault = Vault::open(tmp.path()).unwrap();
    let app = router(AppState::new(Arc::new(vault)));

    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Give the server a moment to start listening.
    tokio::task::yield_now().await;

    let url = format!("http://{addr}/api/types");
    let body: serde_json::Value = reqwest::get(&url).await.unwrap().json().await.unwrap();
    assert!(
        body["types"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "note"),
        "unexpected body: {body}"
    );

    server.abort();
}
