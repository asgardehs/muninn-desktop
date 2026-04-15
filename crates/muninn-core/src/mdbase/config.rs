use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("yaml parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

/// Top-level mdbase collection configuration from `.muninn/config.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MdbaseConfig {
    #[serde(default = "default_spec_version")]
    pub spec_version: String,
    #[serde(default)]
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub settings: Settings,
}

fn default_spec_version() -> String {
    "0.2.0".to_string()
}

/// Collection-level settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub explicit_type_keys: Vec<String>,
    #[serde(default)]
    pub grammar: GrammarSettings,
}

/// Grammar/spell-checking settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammarSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub skip_types: Vec<String>,
    #[serde(default)]
    pub disabled_rules: Vec<String>,
}

fn default_true() -> bool {
    true
}

fn default_language() -> String {
    "en-US".to_string()
}

impl Default for GrammarSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            language: "en-US".to_string(),
            skip_types: Vec::new(),
            disabled_rules: Vec::new(),
        }
    }
}

/// Load config from the `.muninn/` directory.
pub fn load_config(muninn_dir: &Path) -> Result<MdbaseConfig, ConfigError> {
    let config_path = muninn_dir.join("config.yaml");
    let content = std::fs::read_to_string(config_path)?;
    let config: MdbaseConfig = serde_yaml::from_str(&content)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_config() {
        let yaml = "name: test-vault\n";
        let cfg: MdbaseConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cfg.name, "test-vault");
        assert_eq!(cfg.spec_version, "0.2.0");
    }

    #[test]
    fn parse_full_config() {
        let yaml = r#"
spec_version: "0.2.1"
name: my-vault
description: A test vault
settings:
  explicit_type_keys:
    - type
    - kind
  grammar:
    enabled: true
    language: en-US
    disabled_rules:
      - PASSIVE_VOICE
"#;
        let cfg: MdbaseConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cfg.spec_version, "0.2.1");
        assert_eq!(cfg.settings.explicit_type_keys, vec!["type", "kind"]);
        assert_eq!(cfg.settings.grammar.disabled_rules, vec!["PASSIVE_VOICE"]);
    }
}
