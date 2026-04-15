use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct QueryArgs {
    /// SQL query string
    query: String,
}

pub fn run(_args: QueryArgs) -> Result<()> {
    eprintln!("Query engine not yet implemented. Coming in Phase 3.");
    std::process::exit(1);
}
