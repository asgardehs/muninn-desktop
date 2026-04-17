//! Common HTTP error type. Handlers return `Result<T, AppError>`; this
//! module maps the error to a JSON body with an HTTP status code.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

use crate::query::{EvalError, ParseError};
use crate::runestones::{CellWriteError, StorageError, ViewError};
use crate::vault::VaultError;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Conflict(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl AppError {
    fn status(&self) -> StatusCode {
        match self {
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn code(&self) -> &'static str {
        match self {
            AppError::NotFound(_) => "not_found",
            AppError::BadRequest(_) => "bad_request",
            AppError::Conflict(_) => "conflict",
            AppError::Internal(_) => "internal",
        }
    }
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
    code: &'static str,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = Json(ErrorBody {
            error: self.to_string(),
            code: self.code(),
        });
        (self.status(), body).into_response()
    }
}

impl From<VaultError> for AppError {
    fn from(e: VaultError) -> Self {
        match e {
            VaultError::NoteNotFound(p) => {
                AppError::NotFound(format!("note not found: {}", p.display()))
            }
            VaultError::PathNotFound(p) => {
                AppError::NotFound(format!("path not found: {}", p.display()))
            }
            VaultError::QueryParse(pe) => AppError::BadRequest(format!("query parse: {pe}")),
            VaultError::QueryEval(ev) => map_eval(ev),
            VaultError::Mdbase(m) => AppError::BadRequest(m),
            VaultError::Parse(p) => AppError::BadRequest(format!("parse: {p}")),
            VaultError::Io(e) => AppError::Internal(format!("io: {e}")),
            VaultError::Walk(e) => AppError::Internal(format!("walk: {e}")),
        }
    }
}

fn map_eval(e: EvalError) -> AppError {
    match e {
        EvalError::UnknownType(t) => AppError::BadRequest(format!("unknown type: {t}")),
        EvalError::UnknownAlias(a) => AppError::BadRequest(format!("unknown alias: {a}")),
        EvalError::UnknownFunction(f) => AppError::BadRequest(format!("unknown function: {f}")),
        EvalError::TypeMismatch(m) => AppError::BadRequest(format!("type mismatch: {m}")),
        EvalError::ResultTooLarge(n) => AppError::BadRequest(format!("result too large (>{n})")),
        EvalError::ComputedTooDeep(n) => {
            AppError::BadRequest(format!("computed recursion too deep: {n}"))
        }
        EvalError::ComputedParse { field, source } => {
            AppError::BadRequest(format!("computed field {field} parse: {source}"))
        }
        EvalError::Io(e) => AppError::Internal(format!("io: {e}")),
        EvalError::Parse(p) => AppError::BadRequest(format!("parse: {p}")),
        EvalError::Walk(e) => AppError::Internal(format!("walk: {e}")),
    }
}

impl From<ParseError> for AppError {
    fn from(e: ParseError) -> Self {
        AppError::BadRequest(format!("query parse: {e}"))
    }
}

impl From<EvalError> for AppError {
    fn from(e: EvalError) -> Self {
        map_eval(e)
    }
}

impl From<StorageError> for AppError {
    fn from(e: StorageError) -> Self {
        match e {
            StorageError::NotFound(n) => AppError::NotFound(format!("runestone not found: {n}")),
            StorageError::Io(e) => AppError::Internal(format!("io: {e}")),
            StorageError::InvalidYaml { path, source } => {
                AppError::BadRequest(format!("invalid runestone {}: {source}", path.display()))
            }
            StorageError::Serialize(e) => AppError::Internal(format!("yaml serialize: {e}")),
        }
    }
}

impl From<ViewError> for AppError {
    fn from(e: ViewError) -> Self {
        match e {
            ViewError::UnsupportedMultiType { .. } | ViewError::NoSourceType { .. } => {
                AppError::BadRequest(e.to_string())
            }
            ViewError::DuplicateColumn(_) | ViewError::InvalidIdentifier(_) => {
                AppError::BadRequest(e.to_string())
            }
            ViewError::Query(v) => AppError::from(v),
        }
    }
}

impl From<CellWriteError> for AppError {
    fn from(e: CellWriteError) -> Self {
        match e {
            CellWriteError::UnknownColumn(_) | CellWriteError::ReadOnly(_) => {
                AppError::BadRequest(e.to_string())
            }
            CellWriteError::NoteNotFound(p) => {
                AppError::NotFound(format!("note not found: {}", p.display()))
            }
            CellWriteError::Io(e) => AppError::Internal(format!("io: {e}")),
            CellWriteError::InvalidYaml(e) => AppError::BadRequest(format!("invalid yaml: {e}")),
            CellWriteError::Serialize(e) => AppError::Internal(format!("serialize: {e}")),
        }
    }
}

impl From<tokio::task::JoinError> for AppError {
    fn from(e: tokio::task::JoinError) -> Self {
        AppError::Internal(format!("task join: {e}"))
    }
}
