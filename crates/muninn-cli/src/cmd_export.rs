use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct ExportArgs {
    /// Path to note to export
    path: String,

    /// Output format (pdf, html, docx, latex, epub)
    #[arg(long, default_value = "pdf")]
    format: String,
}

pub fn run(_args: ExportArgs) -> Result<()> {
    eprintln!("Export not yet implemented. Coming in Phase 6.");
    std::process::exit(1);
}
