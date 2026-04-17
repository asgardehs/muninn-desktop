use axum::{Json, Router, extract::State, routing::post};
use serde::Deserialize;

use crate::api::{AppError, AppState};

use super::vault_op;

#[derive(Deserialize)]
pub struct QueryBody {
    pub sql: String,
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/query", post(run_query))
}

async fn run_query(
    State(state): State<AppState>,
    Json(body): Json<QueryBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let sql = body.sql.clone();
    let rs = vault_op(&state, move |v| Ok(v.query(&sql)?)).await?;

    let rows: Vec<serde_json::Value> = rs
        .rows
        .into_iter()
        .map(|row| {
            let mut obj = serde_json::Map::new();
            obj.insert(
                "path".to_string(),
                serde_json::Value::String(row.path.display().to_string()),
            );
            let mut cells = serde_json::Map::new();
            for (col, val) in rs.columns.iter().zip(row.cells.iter()) {
                cells.insert(col.clone(), val.to_json());
            }
            obj.insert("cells".to_string(), serde_json::Value::Object(cells));
            serde_json::Value::Object(obj)
        })
        .collect();

    Ok(Json(serde_json::json!({
        "columns": rs.columns,
        "rows": rows,
    })))
}
