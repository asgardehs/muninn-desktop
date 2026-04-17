use std::path::PathBuf;

use axum::{Json, Router, extract::State, routing::post};
use serde::Deserialize;

use crate::api::{AppError, AppState};
use crate::mdbase::validate::{Severity, ValidationError};

use super::vault_op;

pub fn routes() -> Router<AppState> {
    Router::new().route("/validate", post(validate))
}

#[derive(Deserialize, Default)]
pub struct ValidateBody {
    /// If `Some`, validate only this note. Otherwise validate the whole
    /// vault and return every error found.
    pub path: Option<String>,
}

async fn validate(
    State(state): State<AppState>,
    body: Option<Json<ValidateBody>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let target = body.and_then(|Json(b)| b.path);

    let results = vault_op(&state, move |v| {
        if let Some(p) = target {
            let path = PathBuf::from(p);
            let errs = v.validate(&path)?;
            Ok(vec![(path, errs)])
        } else {
            Ok(v.validate_all()?)
        }
    })
    .await?;

    let rows: Vec<serde_json::Value> = results
        .into_iter()
        .map(|(path, errs)| {
            serde_json::json!({
                "path": path.display().to_string(),
                "errors": errs.iter().map(encode_error).collect::<Vec<_>>(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "results": rows })))
}

fn encode_error(e: &ValidationError) -> serde_json::Value {
    serde_json::json!({
        "field": e.field,
        "code": e.code,
        "message": e.message,
        "severity": match e.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        },
    })
}
