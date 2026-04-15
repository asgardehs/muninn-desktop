use std::path::Path;

use anyhow::{Context, Result};
use clap::Args;
use colored::Colorize;

use muninn_core::vault::Vault;
use muninn_core::mdbase::validate::Severity;

#[derive(Args)]
pub struct ValidateArgs {
    /// Path to a specific note (validates all if omitted)
    path: Option<String>,
}

pub fn run(args: ValidateArgs, vault_path: &Path, json: bool) -> Result<()> {
    let vault = Vault::open(vault_path).context("failed to open vault")?;

    let results = if let Some(ref path_str) = args.path {
        let path = std::path::Path::new(path_str);
        let errors = vault.validate(path).context("validation failed")?;
        if errors.is_empty() {
            vec![]
        } else {
            vec![(path.to_path_buf(), errors)]
        }
    } else {
        vault.validate_all().context("validation failed")?
    };

    if json {
        let items: Vec<serde_json::Value> = results
            .iter()
            .flat_map(|(path, errors)| {
                errors.iter().map(move |e| {
                    serde_json::json!({
                        "path": path.display().to_string(),
                        "field": e.field,
                        "code": e.code,
                        "message": e.message,
                        "severity": format!("{:?}", e.severity),
                    })
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&items)?);
        if items.iter().any(|i| i["severity"] == "Error") {
            std::process::exit(1);
        }
        return Ok(());
    }

    if results.is_empty() {
        if args.path.is_some() {
            println!("{}", "Validation passed.".green());
        } else {
            println!("{}", "All notes valid.".green());
        }
        return Ok(());
    }

    let mut error_count = 0;
    let mut warn_count = 0;

    for (path, errors) in &results {
        println!("{}", path.display().to_string().bold());
        for e in errors {
            match e.severity {
                Severity::Error => {
                    println!("  {} {}: {}", "error".red().bold(), e.field, e.message);
                    error_count += 1;
                }
                Severity::Warning => {
                    println!("  {} {}: {}", "warn".yellow().bold(), e.field, e.message);
                    warn_count += 1;
                }
            }
        }
    }

    println!();
    println!(
        "{} error(s), {} warning(s) in {} file(s)",
        error_count, warn_count, results.len()
    );

    if error_count > 0 {
        std::process::exit(1);
    }

    Ok(())
}
