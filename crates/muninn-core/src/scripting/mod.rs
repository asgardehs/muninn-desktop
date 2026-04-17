//! Embedded Rhai scripting engine with a read-only vault API.
//!
//! Scripts evaluate SQL queries, read notes, and emit formatted output
//! (`print`, `table`, `list`). They cannot mutate the vault — mutation is
//! reserved for the CLI, API, and UI.
//!
//! Phase 4 part 1 covers the engine, vault API, and `ScriptEngine::run`.
//! Part 2 adds note rendering (`muninn` code blocks), `.muninn/scripts/`
//! imports, and the `muninn run` / `muninn render` CLI commands.

pub mod bridge;
pub mod engine;
pub mod functions;

pub use engine::{ScriptEngine, ScriptError, ScriptOutput};

/// Controls how `render` reacts when a `muninn` code block fails to evaluate.
///
/// Different callers want different defaults — CLI and HTTP callers want
/// abort-on-error so failures don't go unnoticed, while the live preview
/// (Phase 7) benefits from leaving other blocks intact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderErrorBehavior {
    /// First failing block short-circuits `render` with an error.
    Abort,
    /// Replace the failing block in-place with a `muninn-error` fenced block
    /// so the rest of the note still renders.
    ReplaceBlock,
}

/// Hardcoded resource limits for Phase 4. Configurable via `MdbaseConfig` in
/// a later phase if real users want to override them.
pub const MAX_OPERATIONS: u64 = 1_000_000;
pub const MAX_CALL_LEVELS: usize = 64;
pub const TIMEOUT_SECS: u64 = 5;
