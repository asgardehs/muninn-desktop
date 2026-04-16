//! SQL query engine over vault frontmatter.
//!
//! Parses a subset of SQL (SELECT/WHERE/GROUP BY/HAVING/ORDER BY/LIMIT/OFFSET)
//! via `sqlparser-rs` and evaluates it against notes in a [`Vault`].
//!
//! Phase 3: foundation — SELECT, WHERE, ORDER BY, LIMIT/OFFSET, `FROM <type>`,
//! and the synthetic `FROM note` source that walks every note regardless of
//! type. GROUP BY/HAVING, built-in functions, and computed fields arrive in
//! the second Phase 3 commit.
//!
//! JOINs are deferred to Phase 5 — cross-type joins depend on link-field
//! semantics that Runestones introduces.

pub mod ast;
pub mod eval;
pub mod parser;
pub mod value;

pub use ast::{Expr, MuninnQuery, OrderBy, Projection, SortOrder};
pub use eval::{EvalError, QueryResultSet, execute};
pub use parser::{ParseError, parse_query};
pub use value::Value;

/// Synthetic "any type" source: `FROM note` selects from all notes in the vault,
/// bypassing type matching.
pub const ANY_TYPE_SOURCE: &str = "note";

/// Maximum number of result rows a single query may return.
pub const MAX_RESULT_ROWS: usize = 10_000;

/// Maximum expression nesting depth.
pub const MAX_EXPR_DEPTH: usize = 32;
