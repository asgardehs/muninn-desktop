use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Args;

use muninn_core::scripting::{RenderErrorBehavior, ScriptEngine};
use muninn_core::vault::Vault;

#[derive(Args)]
pub struct RenderArgs {
    /// Note to render
    note: PathBuf,

    /// Continue rendering when a `muninn` block fails, replacing the block
    /// with an inline `muninn-error` fence instead of aborting.
    #[arg(long)]
    continue_on_error: bool,
}

pub fn run(args: RenderArgs, vault_path: &Path, _json: bool) -> Result<()> {
    let vault = Arc::new(Vault::open(vault_path).context("failed to open vault")?);
    let engine = ScriptEngine::new(vault);

    let source = std::fs::read_to_string(&args.note)
        .with_context(|| format!("reading {}", args.note.display()))?;

    let behavior = if args.continue_on_error {
        RenderErrorBehavior::ReplaceBlock
    } else {
        RenderErrorBehavior::Abort
    };

    let rendered = engine
        .render(&source, behavior)
        .with_context(|| format!("rendering {}", args.note.display()))?;
    print!("{rendered}");
    Ok(())
}
