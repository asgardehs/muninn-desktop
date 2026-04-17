use std::path::Path;

use anyhow::{Context, Result, anyhow};
use clap::{Args, Subcommand};
use tabled::builder::Builder;

use muninn_core::runestones;
use muninn_core::vault::Vault;

#[derive(Subcommand)]
pub enum RunestoneCommand {
    /// List Runestones saved in the vault.
    List,
    /// Show a Runestone's definition (YAML).
    Show(NameArgs),
    /// Evaluate a Runestone and print its rows.
    Eval(EvalArgs),
}

#[derive(Args)]
pub struct NameArgs {
    /// Runestone name (matches the `name:` field or the YAML file stem).
    pub name: String,
}

#[derive(Args)]
pub struct EvalArgs {
    /// Runestone name (matches the `name:` field or the YAML file stem).
    pub name: String,

    /// Include the source note path as the first column.
    #[arg(long)]
    pub with_path: bool,
}

pub fn run(cmd: RunestoneCommand, vault_path: &Path, json: bool) -> Result<()> {
    match cmd {
        RunestoneCommand::List => run_list(vault_path, json),
        RunestoneCommand::Show(args) => run_show(&args.name, vault_path, json),
        RunestoneCommand::Eval(args) => run_eval(args, vault_path, json),
    }
}

fn run_list(vault_path: &Path, json: bool) -> Result<()> {
    let all = runestones::load_all(vault_path).context("failed to load runestones")?;

    if json {
        let summaries: Vec<serde_json::Value> = all
            .iter()
            .map(|rs| {
                let mut obj = serde_json::Map::new();
                obj.insert(
                    "name".to_string(),
                    serde_json::Value::String(rs.name.clone()),
                );
                if let Some(d) = &rs.description {
                    obj.insert(
                        "description".to_string(),
                        serde_json::Value::String(d.clone()),
                    );
                }
                obj.insert(
                    "types".to_string(),
                    serde_json::Value::Array(
                        rs.source
                            .types
                            .iter()
                            .map(|t| serde_json::Value::String(t.clone()))
                            .collect(),
                    ),
                );
                obj.insert(
                    "columns".to_string(),
                    serde_json::Value::Number((rs.columns.len() as u64).into()),
                );
                serde_json::Value::Object(obj)
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&summaries)?);
        return Ok(());
    }

    if all.is_empty() {
        println!("(no runestones)");
        return Ok(());
    }

    let mut builder = Builder::default();
    builder.push_record(["name", "types", "columns", "description"]);
    for rs in &all {
        builder.push_record([
            rs.name.clone(),
            rs.source.types.join(","),
            rs.columns.len().to_string(),
            rs.description.clone().unwrap_or_default(),
        ]);
    }
    println!("{}", builder.build());
    Ok(())
}

fn run_show(name: &str, vault_path: &Path, json: bool) -> Result<()> {
    let rs = runestones::load_by_name(vault_path, name)
        .with_context(|| format!("failed to load runestone '{name}'"))?;
    if json {
        println!("{}", serde_json::to_string_pretty(&rs)?);
    } else {
        let yaml = serde_yaml::to_string(&rs).map_err(|e| anyhow!(e))?;
        println!("{yaml}");
    }
    Ok(())
}

fn run_eval(args: EvalArgs, vault_path: &Path, json: bool) -> Result<()> {
    let vault = Vault::open(vault_path).context("failed to open vault")?;
    let rs = runestones::load_by_name(vault_path, &args.name)
        .with_context(|| format!("failed to load runestone '{}'", args.name))?;
    let view = runestones::evaluate(&vault, &rs).context("runestone evaluation failed")?;

    if json {
        let rows: Vec<serde_json::Value> = view
            .rows
            .iter()
            .map(|row| {
                let mut obj = serde_json::Map::new();
                obj.insert(
                    "path".to_string(),
                    serde_json::Value::String(row.path.display().to_string()),
                );
                for (col, val) in view.columns.iter().zip(row.cells.iter()) {
                    obj.insert(col.field.clone(), val.to_json());
                }
                serde_json::Value::Object(obj)
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&rows)?);
        return Ok(());
    }

    if view.rows.is_empty() {
        println!("(no rows)");
        return Ok(());
    }

    let mut builder = Builder::default();

    let mut header: Vec<String> = Vec::new();
    if args.with_path {
        header.push("path".to_string());
    }
    header.extend(view.columns.iter().map(|c| c.display().to_string()));
    builder.push_record(header);

    for row in &view.rows {
        let mut record: Vec<String> = Vec::new();
        if args.with_path {
            record.push(row.path.display().to_string());
        }
        for cell in &row.cells {
            record.push(cell.to_string());
        }
        builder.push_record(record);
    }

    println!("{}", builder.build());
    Ok(())
}
