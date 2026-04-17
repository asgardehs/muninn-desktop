use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::Deserialize;

use crate::api::{AppError, AppState};
use crate::vault::NoteFilter;

use super::vault_op;

#[derive(Deserialize)]
pub struct SearchParams {
    /// The query string (required).
    pub q: String,
    /// Restrict to a single type by name.
    pub r#type: Option<String>,
    /// Limit the number of ranked results returned (default 50).
    pub limit: Option<usize>,
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/search", get(search))
}

async fn search(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let q = params.q.clone();
    let type_filter = params.r#type.clone();
    let limit = params.limit.unwrap_or(50);

    let results = vault_op(&state, move |v| {
        let filter = type_filter.as_ref().map(|t| NoteFilter {
            note_type: Some(t.clone()),
            tag: None,
            title_contains: None,
            field_filters: std::collections::HashMap::new(),
        });
        Ok(v.search(&q, filter.as_ref())?)
    })
    .await?;

    let hits: Vec<serde_json::Value> = results
        .into_iter()
        .take(limit)
        .map(|r| {
            serde_json::json!({
                "path": r.path.display().to_string(),
                "title": r.title,
                "score": r.score,
                "snippet": r.snippet,
            })
        })
        .collect();

    Ok(Json(
        serde_json::json!({ "query": params.q, "results": hits }),
    ))
}
