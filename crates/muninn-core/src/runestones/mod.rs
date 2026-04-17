//! Runestones — relational document views over the vault.
//!
//! A Runestone is a saved, named view stored in `.muninn/runestones/*.yaml`.
//! It targets a type and pins which columns to show, how to filter, and how
//! to sort — essentially a spreadsheet definition whose rows are notes and
//! whose columns are frontmatter fields. Evaluating a Runestone compiles it
//! to SQL and delegates to the `query` module; writing back to a cell
//! updates the owning note's frontmatter in place.

pub mod runestone;
pub mod storage;
pub mod view;
pub mod writeback;

pub use runestone::{ColumnDef, Runestone, RunestoneOrderBy, RunestoneSource, SortDirection};
pub use storage::{RUNESTONES_DIR, StorageError, load_all, load_by_name, save};
pub use view::{RunestoneView, ViewError, evaluate};
pub use writeback::{CellWriteError, update_cell};
