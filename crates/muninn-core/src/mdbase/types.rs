use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// A single mdbase type definition loaded from `.muninn/types/*.md`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDef {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extends: Option<String>,
    #[serde(default)]
    pub fields: IndexMap<String, FieldDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strict: Option<StrictMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#match: Option<MatchRule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_pattern: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name_key: Option<String>,

    /// Resolved fields after inheritance — populated by `inherit::resolve_inheritance`.
    #[serde(skip)]
    pub resolved_fields: Option<IndexMap<String, FieldDef>>,

    /// The raw markdown body below the frontmatter (documentation).
    #[serde(skip)]
    pub body: String,
}

impl TypeDef {
    /// Returns resolved fields if available, otherwise declared fields.
    pub fn effective_fields(&self) -> &IndexMap<String, FieldDef> {
        self.resolved_fields.as_ref().unwrap_or(&self.fields)
    }
}

/// Field definition within a type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDef {
    #[serde(rename = "type", default = "default_field_type_string")]
    pub field_type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_yaml::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub deprecated: bool,
    #[serde(default)]
    pub unique: bool,

    // String constraints.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "minLength")]
    pub min_length: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "maxLength")]
    pub max_length: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,

    // Numeric constraints.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,

    // Enum values.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,

    // List constraints.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<FieldDef>>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "minItems")]
    pub min_items: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "maxItems")]
    pub max_items: Option<usize>,

    // Object sub-fields.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields: Option<IndexMap<String, FieldDef>>,

    // Link constraints.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<LinkTarget>,
    #[serde(default)]
    pub validate_exists: bool,
}

fn default_field_type_string() -> String {
    "string".to_string()
}

impl Default for FieldDef {
    fn default() -> Self {
        Self {
            field_type: "string".to_string(),
            required: false,
            default: None,
            generated: None,
            description: None,
            deprecated: false,
            unique: false,
            min_length: None,
            max_length: None,
            pattern: None,
            min: None,
            max: None,
            values: None,
            items: None,
            min_items: None,
            max_items: None,
            fields: None,
            target: None,
            validate_exists: false,
        }
    }
}

/// Link target — can be a single type name or a list.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LinkTarget {
    Single(String),
    Multiple(Vec<String>),
}

impl LinkTarget {
    pub fn target_types(&self) -> Vec<&str> {
        match self {
            LinkTarget::Single(s) => vec![s.as_str()],
            LinkTarget::Multiple(v) => v.iter().map(|s| s.as_str()).collect(),
        }
    }
}

/// Strict mode for unknown fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StrictMode {
    Forbid,
    Warn,
}

/// Match rule for automatic type matching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchRule {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_glob: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields_present: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#where: Option<IndexMap<String, WhereCond>>,
}

/// A condition in a match rule's where clause.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhereCond {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eq: Option<serde_yaml::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ne: Option<serde_yaml::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contains: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starts_with: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#in: Option<Vec<serde_yaml::Value>>,
}

/// Valid field type strings.
pub const VALID_FIELD_TYPES: &[&str] = &[
    "string", "integer", "number", "boolean", "date", "datetime", "time",
    "enum", "list", "object", "link", "any",
];

pub fn is_valid_field_type(t: &str) -> bool {
    VALID_FIELD_TYPES.contains(&t)
}
