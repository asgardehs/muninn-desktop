//! Load and save Runestone definitions from `.muninn/runestones/`.

use std::path::{Path, PathBuf};

use thiserror::Error;

use super::runestone::Runestone;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("runestone not found: {0}")]
    NotFound(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid runestone YAML: {path}: {source}")]
    InvalidYaml {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("serialize error: {0}")]
    Serialize(#[from] serde_yaml::Error),
}

/// Directory where Runestone YAMLs live, relative to the vault root.
pub const RUNESTONES_DIR: &str = ".muninn/runestones";

pub fn runestones_dir(vault_root: &Path) -> PathBuf {
    vault_root.join(RUNESTONES_DIR)
}

pub fn load_all(vault_root: &Path) -> Result<Vec<Runestone>, StorageError> {
    let dir = runestones_dir(vault_root);
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("yaml")
            && path.extension().and_then(|e| e.to_str()) != Some("yml")
        {
            continue;
        }
        if !entry.file_type()?.is_file() {
            continue;
        }
        let content = std::fs::read_to_string(&path)?;
        let rs: Runestone =
            serde_yaml::from_str(&content).map_err(|e| StorageError::InvalidYaml {
                path: path.clone(),
                source: e,
            })?;
        out.push(rs);
    }

    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Load a Runestone by name. Matches on the `name:` field first, then on the
/// stem of the YAML filename as a fallback.
pub fn load_by_name(vault_root: &Path, name: &str) -> Result<Runestone, StorageError> {
    let all = load_all(vault_root)?;
    if let Some(rs) = all.iter().find(|r| r.name == name) {
        return Ok(rs.clone());
    }

    let dir = runestones_dir(vault_root);
    for ext in ["yaml", "yml"] {
        let candidate = dir.join(format!("{name}.{ext}"));
        if candidate.exists() {
            let content = std::fs::read_to_string(&candidate)?;
            let rs: Runestone =
                serde_yaml::from_str(&content).map_err(|e| StorageError::InvalidYaml {
                    path: candidate.clone(),
                    source: e,
                })?;
            return Ok(rs);
        }
    }

    Err(StorageError::NotFound(name.to_string()))
}

/// Persist a Runestone to `<vault>/.muninn/runestones/<slug>.yaml`, using a
/// slugified name for the filename.
pub fn save(vault_root: &Path, runestone: &Runestone) -> Result<PathBuf, StorageError> {
    let dir = runestones_dir(vault_root);
    std::fs::create_dir_all(&dir)?;

    let slug = slug::slugify(&runestone.name);
    let path = dir.join(format!("{slug}.yaml"));
    let yaml = serde_yaml::to_string(runestone)?;
    std::fs::write(&path, yaml)?;
    Ok(path)
}
