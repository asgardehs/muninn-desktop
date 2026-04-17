use std::path::PathBuf;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use indexmap::IndexMap;
use serde::Deserialize;

use crate::api::{AppError, AppState, json::yaml_to_json};
use crate::vault::NoteFilter;

use super::vault_op;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/notes", get(list_notes).post(create_note))
        .route(
            "/notes/*path",
            get(read_note).put(update_note).delete(delete_note),
        )
}

#[derive(Deserialize)]
pub struct ListParams {
    pub r#type: Option<String>,
    pub tag: Option<String>,
    pub title: Option<String>,
}

async fn list_notes(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let filter = NoteFilter {
        note_type: params.r#type,
        tag: params.tag,
        title_contains: params.title,
        field_filters: std::collections::HashMap::new(),
    };
    let notes = vault_op(&state, move |v| Ok(v.list_notes(&filter)?)).await?;

    let rows: Vec<serde_json::Value> = notes
        .into_iter()
        .map(|n| {
            serde_json::json!({
                "path": n.path.display().to_string(),
                "title": n.title,
                "type": n.note_type,
                "tags": n.tags,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "notes": rows })))
}

async fn read_note(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rel = PathBuf::from(path);
    let note = vault_op(&state, move |v| Ok(v.read_note(&rel)?)).await?;

    let mut fm_json = serde_json::Map::new();
    for (k, v) in &note.frontmatter {
        fm_json.insert(k.clone(), yaml_to_json(v));
    }

    Ok(Json(serde_json::json!({
        "path": note.path.strip_prefix(state.vault.root()).unwrap_or(&note.path).display().to_string(),
        "title": note.title,
        "frontmatter": fm_json,
        "body": note.body,
        "tags": note.tags,
    })))
}

#[derive(Deserialize)]
pub struct CreateNoteBody {
    pub title: String,
    pub r#type: Option<String>,
    #[serde(default)]
    pub fields: serde_json::Map<String, serde_json::Value>,
}

async fn create_note(
    State(state): State<AppState>,
    Json(body): Json<CreateNoteBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let title = body.title.clone();
    let type_name = body.r#type.clone();
    let fields_map: std::collections::HashMap<String, serde_yaml::Value> = body
        .fields
        .into_iter()
        .map(|(k, v)| (k, crate::api::json::json_to_yaml(&v)))
        .collect();

    let path = vault_op(&state, move |v| {
        Ok(v.create_note(&title, type_name.as_deref(), fields_map)?)
    })
    .await?;

    let rel = path
        .strip_prefix(state.vault.root())
        .unwrap_or(&path)
        .display()
        .to_string();
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "path": rel })),
    ))
}

#[derive(Deserialize)]
pub struct UpdateNoteBody {
    #[serde(default)]
    pub frontmatter: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    pub body: String,
}

async fn update_note(
    State(state): State<AppState>,
    Path(path): Path<String>,
    Json(body): Json<UpdateNoteBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rel = PathBuf::from(path);
    let fm_value = serde_json::Value::Object(body.frontmatter);
    let fm_yaml = crate::api::json::json_to_yaml(&fm_value);
    let body_text = body.body;

    let fm_map: IndexMap<String, serde_yaml::Value> = match fm_yaml {
        serde_yaml::Value::Mapping(m) => m
            .into_iter()
            .filter_map(|(k, v)| match k {
                serde_yaml::Value::String(s) => Some((s, v)),
                _ => None,
            })
            .collect(),
        _ => {
            return Err(AppError::BadRequest(
                "frontmatter must be an object".to_string(),
            ));
        }
    };

    let abs = vault_op(
        &state,
        move |v| Ok(v.write_note(&rel, &fm_map, &body_text)?),
    )
    .await?;
    let rel_out = abs
        .strip_prefix(state.vault.root())
        .unwrap_or(&abs)
        .display()
        .to_string();
    Ok(Json(serde_json::json!({ "path": rel_out })))
}

async fn delete_note(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let rel = PathBuf::from(path);
    vault_op(&state, move |v| Ok(v.delete_note(&rel)?)).await?;
    Ok(StatusCode::NO_CONTENT)
}
