use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use clap::Subcommand;
use tabled::{Table, Tabled};

use muninn_core::vault::{NoteFilter, Vault};

#[derive(Subcommand)]
pub enum NoteCommand {
    /// Create a new note
    New(NewArgs),
    /// List notes
    List(ListArgs),
    /// Search notes
    Search(SearchArgs),
    /// Show backlinks to a note
    Backlinks(BacklinksArgs),
}

#[derive(clap::Args)]
pub struct NewArgs {
    /// Note title (words joined with spaces)
    #[arg(required = true, num_args = 1..)]
    title: Vec<String>,

    /// Note type
    #[arg(long, short = 't')]
    r#type: Option<String>,

    /// Tags (comma-separated)
    #[arg(long)]
    tags: Option<String>,

    /// Extra fields as key=value pairs
    #[arg(long = "field", short = 'f', value_parser = parse_field)]
    fields: Vec<(String, String)>,
}

#[derive(clap::Args)]
pub struct ListArgs {
    /// Filter by type
    #[arg(long, short = 't')]
    r#type: Option<String>,

    /// Filter by tag
    #[arg(long)]
    tag: Option<String>,

    /// Filter by title substring
    #[arg(long)]
    title: Option<String>,
}

#[derive(clap::Args)]
pub struct SearchArgs {
    /// Search query (words joined with spaces)
    #[arg(required = true, num_args = 1..)]
    pub query: Vec<String>,

    /// Maximum number of results
    #[arg(long, default_value = "10")]
    pub limit: usize,

    /// Filter by type
    #[arg(long, short = 't')]
    pub r#type: Option<String>,

    /// Filter by tag
    #[arg(long)]
    pub tag: Option<String>,
}

#[derive(clap::Args)]
pub struct BacklinksArgs {
    /// Path to note (relative to vault root)
    path: String,
}

fn rel_path(path: &Path, vault_path: &Path) -> String {
    path.strip_prefix(vault_path)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn parse_field(s: &str) -> Result<(String, String), String> {
    let pos = s.find('=').ok_or_else(|| format!("expected KEY=VALUE, got {:?}", s))?;
    Ok((s[..pos].to_string(), s[pos + 1..].to_string()))
}

pub fn run(cmd: NoteCommand, vault_path: &Path, json: bool) -> Result<()> {
    match cmd {
        NoteCommand::New(args) => run_new(args, vault_path),
        NoteCommand::List(args) => run_list(args, vault_path, json),
        NoteCommand::Search(args) => run_search(args, vault_path, json),
        NoteCommand::Backlinks(args) => run_backlinks(args, vault_path),
    }
}

fn run_new(args: NewArgs, vault_path: &Path) -> Result<()> {
    let vault = Vault::open(vault_path).context("failed to open vault")?;
    let title = args.title.join(" ");

    let mut fields: HashMap<String, serde_yaml::Value> = HashMap::new();

    if let Some(tags) = args.tags {
        let tag_list: Vec<serde_yaml::Value> = tags
            .split(',')
            .map(|t| serde_yaml::Value::String(t.trim().to_string()))
            .collect();
        fields.insert("tags".to_string(), serde_yaml::Value::Sequence(tag_list));
    }

    for (key, value) in args.fields {
        fields.insert(key, serde_yaml::Value::String(value));
    }

    let path = vault
        .create_note(&title, args.r#type.as_deref(), fields)
        .context("failed to create note")?;

    let rel = path
        .strip_prefix(vault_path)
        .unwrap_or(&path);
    println!("Created {}", rel.display());

    Ok(())
}

#[derive(Tabled)]
struct NoteRow {
    #[tabled(rename = "Path")]
    path: String,
    #[tabled(rename = "Title")]
    title: String,
    #[tabled(rename = "Type")]
    note_type: String,
}

fn run_list(args: ListArgs, vault_path: &Path, json: bool) -> Result<()> {
    let vault = Vault::open(vault_path).context("failed to open vault")?;

    let mut filter = NoteFilter::new();
    if let Some(ref t) = args.r#type {
        filter = filter.with_type(t);
    }
    if let Some(ref tag) = args.tag {
        filter = filter.with_tag(tag);
    }
    if let Some(ref title) = args.title {
        filter = filter.with_title(title);
    }

    let notes = vault.list_notes(&filter).context("failed to list notes")?;

    if json {
        let items: Vec<serde_json::Value> = notes
            .iter()
            .map(|n| {
                serde_json::json!({
                    "path": rel_path(&n.path, vault_path),
                    "title": n.title,
                    "type": n.note_type,
                    "tags": n.tags,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&items)?);
        return Ok(());
    }

    if notes.is_empty() {
        println!("No notes found.");
        return Ok(());
    }

    let rows: Vec<NoteRow> = notes
        .iter()
        .map(|n| NoteRow {
            path: rel_path(&n.path, vault_path),
            title: n.title.clone(),
            note_type: n.note_type.clone().unwrap_or_default(),
        })
        .collect();

    println!("{}", Table::new(rows));

    Ok(())
}

pub fn run_search(args: SearchArgs, vault_path: &Path, json: bool) -> Result<()> {
    let vault = Vault::open(vault_path).context("failed to open vault")?;
    let query = args.query.join(" ");

    let filter = {
        let mut f = NoteFilter::new();
        if let Some(ref t) = args.r#type {
            f = f.with_type(t);
        }
        if let Some(ref tag) = args.tag {
            f = f.with_tag(tag);
        }
        f
    };

    let results = vault
        .search(&query, Some(&filter))
        .context("search failed")?;

    let results: Vec<_> = results.into_iter().take(args.limit).collect();

    if json {
        let items: Vec<serde_json::Value> = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "path": rel_path(&r.path, vault_path),
                    "title": r.title,
                    "score": r.score,
                    "snippet": r.snippet,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&items)?);
        return Ok(());
    }

    if results.is_empty() {
        println!("No results found.");
        return Ok(());
    }

    for (i, result) in results.iter().enumerate() {
        if i > 0 {
            println!("---");
        }
        println!(
            "[{}] {} (score: {})",
            rel_path(&result.path, vault_path),
            result.title,
            result.score
        );
        if let Some(ref snippet) = result.snippet {
            println!("  {}", snippet);
        }
    }

    Ok(())
}

fn run_backlinks(args: BacklinksArgs, vault_path: &Path) -> Result<()> {
    let vault = Vault::open(vault_path).context("failed to open vault")?;
    let path = std::path::Path::new(&args.path);

    let backlinks = vault.backlinks(path);

    if backlinks.is_empty() {
        println!("No notes link to {}.", args.path);
        return Ok(());
    }

    println!("Notes linking to {}:", args.path);
    for link in &backlinks {
        println!("  {}", link.display());
    }

    Ok(())
}
