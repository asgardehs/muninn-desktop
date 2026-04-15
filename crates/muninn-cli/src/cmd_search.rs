use std::path::Path;

use anyhow::Result;
use clap::Args;

use crate::cmd_note;

#[derive(Args)]
pub struct SearchArgs {
    /// Search query (words joined with spaces)
    #[arg(required = true, num_args = 1..)]
    query: Vec<String>,

    /// Maximum number of results
    #[arg(long, default_value = "10")]
    limit: usize,

    /// Filter by type
    #[arg(long, short = 't')]
    r#type: Option<String>,

    /// Filter by tag
    #[arg(long)]
    tag: Option<String>,
}

pub fn run(args: SearchArgs, vault_path: &Path, json: bool) -> Result<()> {
    // Delegate to the note search handler.
    cmd_note::run_search(
        cmd_note::SearchArgs {
            query: args.query,
            limit: args.limit,
            r#type: args.r#type,
            tag: args.tag,
        },
        vault_path,
        json,
    )
}
