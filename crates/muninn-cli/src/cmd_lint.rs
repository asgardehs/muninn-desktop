use std::path::Path;

use anyhow::{Context, Result};
use clap::Args;
use colored::Colorize;

use muninn_core::grammar::GrammarChecker;
use muninn_core::vault::{NoteFilter, Vault};

#[derive(Args)]
pub struct LintArgs {
    /// Path to a specific note (lints all if omitted)
    path: Option<String>,

    /// Filter by type (lint all notes of this type)
    #[arg(long, short = 't')]
    r#type: Option<String>,
}

pub fn run(args: LintArgs, vault_path: &Path, json: bool) -> Result<()> {
    let vault = Vault::open(vault_path).context("failed to open vault")?;

    let dict_path = vault_path.join(".muninn/dictionary.txt");
    let dict_opt = if dict_path.exists() {
        Some(dict_path.as_path())
    } else {
        None
    };
    let checker = GrammarChecker::new(dict_opt);

    let paths: Vec<std::path::PathBuf> = if let Some(ref path_str) = args.path {
        vec![std::path::PathBuf::from(path_str)]
    } else {
        let mut filter = NoteFilter::new();
        if let Some(ref t) = args.r#type {
            filter = filter.with_type(t);
        }
        let notes = vault.list_notes(&filter).context("failed to list notes")?;
        notes.into_iter().map(|n| n.path).collect()
    };

    let mut all_diagnostics: Vec<serde_json::Value> = Vec::new();
    let mut total_issues = 0;

    for path in &paths {
        let note = match vault.read_note(path) {
            Ok(n) => n,
            Err(e) => {
                eprintln!("warning: skipping {}: {}", path.display(), e);
                continue;
            }
        };

        let diagnostics = checker.check(&note.body);

        if diagnostics.is_empty() {
            continue;
        }

        if json {
            for d in &diagnostics {
                all_diagnostics.push(serde_json::json!({
                    "path": path.display().to_string(),
                    "start": d.span.start,
                    "end": d.span.end,
                    "message": d.message,
                    "suggestions": d.suggestions,
                    "severity": format!("{:?}", d.severity),
                    "rule": d.rule,
                }));
            }
        } else {
            println!("{}", path.display().to_string().bold());
            for d in &diagnostics {
                let severity_str = match d.severity {
                    muninn_core::grammar::DiagnosticSeverity::Error => "error".red().bold(),
                    muninn_core::grammar::DiagnosticSeverity::Warning => "warn".yellow().bold(),
                };
                print!("  {} [{}..{}] {}", severity_str, d.span.start, d.span.end, d.message);
                if !d.suggestions.is_empty() {
                    print!(" (suggest: {})", d.suggestions.join(", "));
                }
                println!();
            }
        }

        total_issues += diagnostics.len();
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&all_diagnostics)?);
        return Ok(());
    }

    if total_issues == 0 {
        println!("{}", "No grammar or spelling issues found.".green());
    } else {
        println!();
        println!("{} issue(s) in {} file(s)", total_issues, paths.len());
    }

    Ok(())
}
