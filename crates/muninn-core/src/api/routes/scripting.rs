use std::path::PathBuf;

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::post,
};
use serde::Deserialize;

use crate::api::{AppError, AppState};
use crate::scripting::{RenderErrorBehavior, ScriptEngine};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/render/*path", post(render))
        .route("/run", post(run_script))
}

#[derive(Deserialize, Default)]
pub struct RenderBody {
    /// How to handle a failing script block. Defaults to `abort`.
    #[serde(default)]
    pub on_error: RenderMode,
}

#[derive(Deserialize, Default, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum RenderMode {
    #[default]
    Abort,
    ReplaceBlock,
}

impl From<RenderMode> for RenderErrorBehavior {
    fn from(m: RenderMode) -> Self {
        match m {
            RenderMode::Abort => RenderErrorBehavior::Abort,
            RenderMode::ReplaceBlock => RenderErrorBehavior::ReplaceBlock,
        }
    }
}

async fn render(
    State(state): State<AppState>,
    Path(path): Path<String>,
    body: Option<Json<RenderBody>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rel = PathBuf::from(path);
    let mode: RenderErrorBehavior = body.map(|Json(b)| b.on_error).unwrap_or_default().into();
    let vault = state.vault.clone();

    let rendered = tokio::task::spawn_blocking(move || -> Result<String, AppError> {
        let note = vault.read_note(&rel)?;
        let engine = ScriptEngine::new(vault.clone());
        engine
            .render(&note.body, mode)
            .map_err(|e| AppError::BadRequest(format!("render: {e}")))
    })
    .await??;

    Ok(Json(serde_json::json!({ "rendered": rendered })))
}

#[derive(Deserialize)]
pub struct RunBody {
    pub code: String,
}

async fn run_script(
    State(state): State<AppState>,
    Json(body): Json<RunBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let code = body.code;
    let vault = state.vault.clone();

    let output = tokio::task::spawn_blocking(move || -> Result<String, AppError> {
        let engine = ScriptEngine::new(vault);
        let out = engine
            .run(&code)
            .map_err(|e| AppError::BadRequest(format!("script: {e}")))?;
        Ok(out.text)
    })
    .await??;

    Ok(Json(serde_json::json!({ "output": output })))
}
