use std::path::Path;

use anyhow::Result;

pub fn run(vault_path: &Path) -> Result<()> {
    let muninn_dir = vault_path.join(".muninn");
    let types_dir = muninn_dir.join("types");
    let config_path = muninn_dir.join("config.yaml");

    if config_path.exists() {
        println!("Vault already initialized at {}", vault_path.display());
        return Ok(());
    }

    std::fs::create_dir_all(&types_dir)?;

    // Write default config.
    std::fs::write(
        &config_path,
        r#"spec_version: "0.2.0"
name: muninn
settings:
  explicit_type_keys:
    - type
  grammar:
    enabled: true
    language: en-US
"#,
    )?;

    // Write default note type.
    std::fs::write(
        types_dir.join("note.md"),
        r#"---
name: note
description: A basic note
fields:
  title:
    type: string
    required: true
  tags:
    type: list
    items:
      type: string
  status:
    type: enum
    values:
      - active
      - done
      - archived
---
The default note type. All notes should have a title.
"#,
    )?;

    println!("Initialized vault at {}", vault_path.display());
    println!("  config: {}", config_path.display());
    println!("  types:  {}", types_dir.display());

    Ok(())
}
