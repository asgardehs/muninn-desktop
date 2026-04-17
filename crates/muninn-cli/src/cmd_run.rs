use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Args;

use muninn_core::scripting::ScriptEngine;
use muninn_core::vault::Vault;

#[derive(Args)]
pub struct RunArgs {
    /// Path to a .rhai script file
    script: PathBuf,
}

pub fn run(args: RunArgs, vault_path: &Path, _json: bool) -> Result<()> {
    let vault = Arc::new(Vault::open(vault_path).context("failed to open vault")?);
    let engine = ScriptEngine::new(vault);
    let output = engine
        .run_file(&args.script)
        .with_context(|| format!("running {}", args.script.display()))?;
    print!("{}", output.text);
    Ok(())
}
