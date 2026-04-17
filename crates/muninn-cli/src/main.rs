mod cmd_init;
mod cmd_note;
mod cmd_search;
mod cmd_type;
mod cmd_validate;
mod cmd_lint;
mod cmd_query;
mod cmd_run;
mod cmd_render;
mod cmd_export;
mod cmd_backfill;

use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "muninn", version, about = "Personal knowledge base and note management")]
struct Cli {
    /// Output as JSON where supported
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize a new vault
    Init,
    /// Note operations (new, list, search, backlinks)
    #[command(subcommand)]
    Note(cmd_note::NoteCommand),
    /// Search notes (alias for `note search`)
    Search(cmd_search::SearchArgs),
    /// Type management (list, show)
    #[command(subcommand)]
    Type(cmd_type::TypeCommand),
    /// Validate notes against type schemas
    Validate(cmd_validate::ValidateArgs),
    /// Grammar and spell check
    Lint(cmd_lint::LintArgs),
    /// Run SQL queries over notes
    Query(cmd_query::QueryArgs),
    /// Run a .rhai script against the vault
    Run(cmd_run::RunArgs),
    /// Render a note with its `muninn` script blocks evaluated
    Render(cmd_render::RenderArgs),
    /// Export notes to PDF, HTML, DOCX, etc. (not yet implemented)
    Export(cmd_export::ExportArgs),
    /// Backfill generated/default fields (not yet implemented)
    Backfill(cmd_backfill::BackfillArgs),
}

fn resolve_vault_path() -> PathBuf {
    if let Ok(path) = std::env::var("MUNINN_VAULT_PATH") {
        let p = PathBuf::from(path);
        return p.canonicalize().unwrap_or(p);
    }
    let data_dir = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
            PathBuf::from(home).join(".local/share")
        });
    data_dir.join("muninn")
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Init => cmd_init::run(&resolve_vault_path()),
        Command::Note(cmd) => {
            let vault_path = resolve_vault_path();
            cmd_note::run(cmd, &vault_path, cli.json)
        }
        Command::Search(args) => {
            let vault_path = resolve_vault_path();
            cmd_search::run(args, &vault_path, cli.json)
        }
        Command::Type(cmd) => {
            let vault_path = resolve_vault_path();
            cmd_type::run(cmd, &vault_path, cli.json)
        }
        Command::Validate(args) => {
            let vault_path = resolve_vault_path();
            cmd_validate::run(args, &vault_path, cli.json)
        }
        Command::Lint(args) => {
            let vault_path = resolve_vault_path();
            cmd_lint::run(args, &vault_path, cli.json)
        }
        Command::Query(args) => {
            let vault_path = resolve_vault_path();
            cmd_query::run(args, &vault_path, cli.json)
        }
        Command::Run(args) => {
            let vault_path = resolve_vault_path();
            cmd_run::run(args, &vault_path, cli.json)
        }
        Command::Render(args) => {
            let vault_path = resolve_vault_path();
            cmd_render::run(args, &vault_path, cli.json)
        }
        Command::Export(args) => cmd_export::run(args),
        Command::Backfill(args) => cmd_backfill::run(args),
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        process::exit(1);
    }
}
