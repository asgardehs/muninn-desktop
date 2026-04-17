use std::path::PathBuf;

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, put},
};
use serde::Deserialize;

use crate::api::{AppError, AppState, json::json_to_yaml};
use crate::query::Value;
use crate::runestones;

use super::vault_op;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/runestones", get(list_runestones))
        .route("/runestones/:name", get(eval_runestone))
        .route("/runestones/:name/rows/*path", put(update_runestone_cell))
}

async fn list_runestones(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let all = vault_op(&state, |v| Ok(runestones::load_all(v.root())?)).await?;

    let summaries: Vec<serde_json::Value> = all
        .into_iter()
        .map(|rs| {
            serde_json::json!({
                "name": rs.name,
                "description": rs.description,
                "types": rs.source.types,
                "columns": rs.columns.len(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "runestones": summaries })))
}

async fn eval_runestone(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let view = vault_op(&state, move |v| {
        let rs = runestones::load_by_name(v.root(), &name)?;
        Ok(runestones::evaluate(v, &rs)?)
    })
    .await?;

    let columns: Vec<serde_json::Value> = view
        .columns
        .iter()
        .map(|c| {
            serde_json::json!({
                "field": c.field,
                "header": c.header,
                "width": c.width,
                "computed": c.computed,
                "hidden": c.hidden,
            })
        })
        .collect();

    let rows: Vec<serde_json::Value> = view
        .rows
        .iter()
        .map(|row| {
            let mut cells = serde_json::Map::new();
            for (col, val) in view.columns.iter().zip(row.cells.iter()) {
                cells.insert(col.field.clone(), val.to_json());
            }
            serde_json::json!({
                "path": row.path.display().to_string(),
                "cells": cells,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "name": view.name,
        "description": view.description,
        "group_by": view.group_by,
        "columns": columns,
        "rows": rows,
    })))
}

#[derive(Deserialize)]
pub struct CellUpdate {
    pub column: String,
    pub value: serde_json::Value,
}

async fn update_runestone_cell(
    State(state): State<AppState>,
    Path((name, path)): Path<(String, String)>,
    Json(body): Json<CellUpdate>,
) -> Result<Json<serde_json::Value>, AppError> {
    let note_path = PathBuf::from(path);
    let column = body.column.clone();
    let yaml_val = json_to_yaml(&body.value);
    let new_val = Value::from_yaml(&yaml_val);

    vault_op(&state, move |v| {
        let rs = runestones::load_by_name(v.root(), &name)?;
        runestones::update_cell(v.root(), &rs, &note_path, &column, &new_val)?;
        Ok(())
    })
    .await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}
