use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};

use crate::api::{AppError, AppState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/types", get(list_types))
        .route("/types/:name", get(get_type))
}

async fn list_types(State(state): State<AppState>) -> Result<Json<serde_json::Value>, AppError> {
    let names: Vec<&String> = state.vault.types().keys().collect();
    Ok(Json(serde_json::json!({
        "types": names,
    })))
}

async fn get_type(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let td = state
        .vault
        .types()
        .get(&name)
        .ok_or_else(|| AppError::NotFound(format!("type {name} not found")))?;
    let body = serde_json::to_value(td).map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(body))
}
