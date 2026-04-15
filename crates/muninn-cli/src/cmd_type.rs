use std::path::Path;

use anyhow::{Context, Result};
use clap::Subcommand;
use colored::Colorize;
use tabled::{Table, Tabled};

use muninn_core::vault::Vault;

#[derive(Subcommand)]
pub enum TypeCommand {
    /// List all defined types
    List,
    /// Show details of a specific type
    Show(ShowArgs),
}

#[derive(clap::Args)]
pub struct ShowArgs {
    /// Type name
    name: String,
}

pub fn run(cmd: TypeCommand, vault_path: &Path, json: bool) -> Result<()> {
    match cmd {
        TypeCommand::List => run_list(vault_path, json),
        TypeCommand::Show(args) => run_show(args, vault_path, json),
    }
}

#[derive(Tabled)]
struct TypeRow {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Description")]
    description: String,
    #[tabled(rename = "Extends")]
    extends: String,
    #[tabled(rename = "Fields")]
    field_count: usize,
}

fn run_list(vault_path: &Path, json: bool) -> Result<()> {
    let vault = Vault::open(vault_path).context("failed to open vault")?;
    let types = vault.types();

    if json {
        let items: Vec<serde_json::Value> = types
            .values()
            .map(|td| {
                serde_json::json!({
                    "name": td.name,
                    "description": td.description,
                    "extends": td.extends,
                    "fields": td.effective_fields().keys().collect::<Vec<_>>(),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&items)?);
        return Ok(());
    }

    if types.is_empty() {
        println!("No types defined.");
        return Ok(());
    }

    let mut rows: Vec<TypeRow> = types
        .values()
        .map(|td| TypeRow {
            name: td.name.clone(),
            description: td.description.clone().unwrap_or_default(),
            extends: td.extends.clone().unwrap_or_default(),
            field_count: td.effective_fields().len(),
        })
        .collect();
    rows.sort_by(|a, b| a.name.cmp(&b.name));

    println!("{}", Table::new(rows));

    Ok(())
}

fn run_show(args: ShowArgs, vault_path: &Path, json: bool) -> Result<()> {
    let vault = Vault::open(vault_path).context("failed to open vault")?;

    let td = vault
        .types()
        .get(&args.name)
        .ok_or_else(|| anyhow::anyhow!("type {:?} not found", args.name))?;

    if json {
        let fields: Vec<serde_json::Value> = td
            .effective_fields()
            .iter()
            .map(|(name, field)| {
                serde_json::json!({
                    "name": name,
                    "type": field.field_type,
                    "required": field.required,
                    "default": field.default,
                    "generated": field.generated,
                    "description": field.description,
                    "deprecated": field.deprecated,
                })
            })
            .collect();

        let output = serde_json::json!({
            "name": td.name,
            "description": td.description,
            "extends": td.extends,
            "fields": fields,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Header.
    println!("{}", td.name.bold());
    if let Some(ref desc) = td.description {
        println!("  {}", desc);
    }
    if let Some(ref extends) = td.extends {
        println!("  extends: {}", extends.cyan());
    }
    println!();

    // Fields.
    let fields = td.effective_fields();
    if fields.is_empty() {
        println!("  No fields defined.");
    } else {
        println!("  {}", "Fields:".bold());
        for (name, field) in fields {
            let mut attrs = Vec::new();
            attrs.push(field.field_type.clone());
            if field.required {
                attrs.push("required".to_string());
            }
            if let Some(ref default) = field.default {
                attrs.push(format!("default={:?}", default));
            }
            if let Some(ref strategy) = field.generated {
                attrs.push(format!("generated={}", strategy));
            }
            if field.deprecated {
                attrs.push("deprecated".to_string());
            }
            if let Some(ref values) = field.values {
                attrs.push(format!("values={:?}", values));
            }

            let attr_str = attrs.join(", ");
            let name_str = if field.required {
                name.bold().to_string()
            } else {
                name.to_string()
            };
            println!("    {} ({})", name_str, attr_str.dimmed());

            if let Some(ref desc) = field.description {
                println!("      {}", desc);
            }
        }
    }

    // Body (type documentation).
    if !td.body.is_empty() {
        println!();
        println!("{}", td.body.trim());
    }

    Ok(())
}
