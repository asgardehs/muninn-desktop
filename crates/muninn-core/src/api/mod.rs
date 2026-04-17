//! HTTP API for programmatic access to a vault.
//!
//! Internal to the Asgard ecosystem — used by Huginn, Odin, automation
//! scripts, and the Tauri desktop shell (which embeds the server on a local
//! port). No auth, no rate limiting, no versioned stability guarantees
//! beyond what the team needs.
//!
//! Built on `axum` over Tokio. The router is constructed from a shared
//! `AppState` so callers can wrap the `Router` with their own middleware
//! (CORS, tracing, auth shims) before handing it to `axum::serve` or the
//! Tauri shell.

use std::sync::Arc;

use axum::Router;
use tower_http::cors::CorsLayer;

use crate::vault::Vault;

pub mod error;
pub mod json;
pub mod routes;

pub use error::AppError;

/// Shared state every handler receives. `Arc<Vault>` keeps the vault
/// pinned for the duration of the server; handlers that mutate it hold
/// the vault's internal locks on a blocking task (see `spawn_blocking`
/// usage in individual routes).
#[derive(Clone)]
pub struct AppState {
    pub vault: Arc<Vault>,
}

impl AppState {
    pub fn new(vault: Arc<Vault>) -> Self {
        Self { vault }
    }
}

/// Build the full `/api/*` router. Callers can nest this under another
/// path, attach middleware, or merge it into a larger Tauri-hosted
/// `Router`.
pub fn router(state: AppState) -> Router {
    Router::new()
        .nest("/api", routes::routes())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
