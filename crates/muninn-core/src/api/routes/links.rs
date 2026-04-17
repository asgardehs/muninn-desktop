use std::path::PathBuf;

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};

use crate::api::{AppError, AppState};

use super::vault_op;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/links/backlinks/*path", get(backlinks))
        .route("/links/graph", get(graph))
}

async fn backlinks(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let target = PathBuf::from(path);
    let found = vault_op(&state, move |v| Ok(v.backlinks(&target))).await?;
    let paths: Vec<String> = found.into_iter().map(|p| p.display().to_string()).collect();
    Ok(Json(serde_json::json!({ "backlinks": paths })))
}

/// Full link graph as `{ nodes: [...], edges: [...] }`. Each node is a note
/// path; each edge is `{ from, to }` with the target normalized to the
/// `[[wikilink]]` stem (may be unresolved). Consumers pair this with
/// `/api/notes` to hydrate metadata.
async fn graph(State(state): State<AppState>) -> Result<Json<serde_json::Value>, AppError> {
    let edges = vault_op(&state, |v| Ok(v.link_graph())).await?;

    let mut nodes = std::collections::BTreeSet::new();
    let mut edge_list: Vec<serde_json::Value> = Vec::new();
    for (from, targets) in edges {
        let from_s = from.display().to_string();
        nodes.insert(from_s.clone());
        for target in targets {
            nodes.insert(target.clone());
            edge_list.push(serde_json::json!({ "from": from_s, "to": target }));
        }
    }

    Ok(Json(serde_json::json!({
        "nodes": nodes.into_iter().collect::<Vec<_>>(),
        "edges": edge_list,
    })))
}
