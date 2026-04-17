//! Per-resource route modules, each exposing a `routes() -> Router` that
//! the top-level `api::router` nests under `/api`.

use axum::Router;

use super::AppState;

pub mod links;
pub mod notes;
pub mod query;
pub mod runestones;
pub mod scripting;
pub mod search;
pub mod types;
pub mod validate;

pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(notes::routes())
        .merge(search::routes())
        .merge(query::routes())
        .merge(runestones::routes())
        .merge(types::routes())
        .merge(links::routes())
        .merge(scripting::routes())
        .merge(validate::routes())
}

/// Helper shared across route modules: run a synchronous Vault operation on
/// the blocking thread pool so the async runtime can keep servicing other
/// requests while walkdir / fs reads happen. Propagates both the
/// `spawn_blocking` join failure and any domain error from the closure.
pub(super) async fn vault_op<T, F>(state: &AppState, f: F) -> Result<T, super::AppError>
where
    T: Send + 'static,
    F: FnOnce(&crate::vault::Vault) -> Result<T, super::AppError> + Send + 'static,
{
    let vault = state.vault.clone();
    tokio::task::spawn_blocking(move || f(&vault)).await?
}
