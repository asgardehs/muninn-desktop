use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct BackfillArgs {
    /// Type to backfill
    #[arg(long)]
    r#type: String,

    /// Field to backfill
    #[arg(long)]
    field: String,

    /// Show what would change without writing
    #[arg(long)]
    dry_run: bool,
}

pub fn run(_args: BackfillArgs) -> Result<()> {
    eprintln!("Backfill not yet implemented. Coming in Phase 10.");
    std::process::exit(1);
}
