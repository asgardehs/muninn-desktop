use std::path::Path;

use anyhow::{Context, Result};
use clap::Args;
use tabled::builder::Builder;

use muninn_core::vault::Vault;

#[derive(Args)]
pub struct QueryArgs {
    /// SQL query string
    query: String,

    /// Include the source path as the first column
    #[arg(long)]
    with_path: bool,
}

pub fn run(args: QueryArgs, vault_path: &Path, json: bool) -> Result<()> {
    let vault = Vault::open(vault_path).context("failed to open vault")?;
    let rs = vault.query(&args.query).context("query failed")?;

    if json {
        let rows: Vec<serde_json::Value> = rs
            .rows
            .iter()
            .map(|row| {
                let mut obj = serde_json::Map::new();
                obj.insert(
                    "path".to_string(),
                    serde_json::Value::String(row.path.display().to_string()),
                );
                for (col, val) in rs.columns.iter().zip(row.cells.iter()) {
                    obj.insert(col.clone(), val.to_json());
                }
                serde_json::Value::Object(obj)
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&rows)?);
        return Ok(());
    }

    if rs.rows.is_empty() {
        println!("(no rows)");
        return Ok(());
    }

    let mut builder = Builder::default();

    let mut header: Vec<String> = Vec::new();
    if args.with_path {
        header.push("path".to_string());
    }
    header.extend(rs.columns.iter().cloned());
    builder.push_record(header);

    for row in &rs.rows {
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
