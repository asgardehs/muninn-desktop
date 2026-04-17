use std::collections::HashMap;

use regex::Regex;

use super::config::MdbaseConfig;
use super::types::{FieldDef, StrictMode, TypeDef};

/// A single validation issue.
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub code: String,
    pub message: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Severity {
    Error,
    Warning,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}: {}", self.code, self.field, self.message)
    }
}

/// Validate a frontmatter map against a type definition.
pub fn validate_record(
    frontmatter: &HashMap<String, serde_yaml::Value>,
    td: &TypeDef,
    _cfg: Option<&MdbaseConfig>,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let fields = td.effective_fields();

    // Check required fields.
    for (name, field) in fields {
        if !field.required {
            continue;
        }
        match frontmatter.get(name) {
            None => {
                errors.push(ValidationError {
                    field: name.clone(),
                    code: "missing_required".to_string(),
                    message: format!("required field {:?} is missing", name),
                    severity: Severity::Error,
                });
            }
            Some(serde_yaml::Value::Null) => {
                errors.push(ValidationError {
                    field: name.clone(),
                    code: "missing_required".to_string(),
                    message: format!("required field {:?} is null", name),
                    severity: Severity::Error,
                });
            }
            _ => {}
        }
    }

    // Check each frontmatter field.
    for (name, val) in frontmatter {
        // Skip null values — already handled by required check.
        if val.is_null() {
            continue;
        }

        if let Some(field) = fields.get(name) {
            let field_errors = validate_field(name, val, field);
            errors.extend(field_errors);
        } else {
            // Unknown field — check strict mode.
            match td.strict {
                Some(StrictMode::Forbid) => {
                    errors.push(ValidationError {
                        field: name.clone(),
                        code: "unknown_field".to_string(),
                        message: format!("unknown field {:?} in type {:?}", name, td.name),
                        severity: Severity::Error,
                    });
                }
                Some(StrictMode::Warn) => {
                    errors.push(ValidationError {
                        field: name.clone(),
                        code: "unknown_field".to_string(),
                        message: format!("unknown field {:?} in type {:?}", name, td.name),
                        severity: Severity::Warning,
                    });
                }
                None => {}
            }
        }
    }

    errors
}

/// Validate a single value against a field definition.
pub fn validate_field(
    name: &str,
    value: &serde_yaml::Value,
    field: &FieldDef,
) -> Vec<ValidationError> {
    if value.is_null() {
        return Vec::new();
    }

    match field.field_type.as_str() {
        "string" => validate_string(name, value, field),
        "integer" => validate_integer(name, value, field),
        "number" => validate_number(name, value, field),
        "boolean" => validate_boolean(name, value),
        "date" => validate_date(name, value),
        "datetime" => validate_datetime(name, value),
        "time" => validate_time(name, value),
        "enum" => validate_enum(name, value, field),
        "list" => validate_list(name, value, field),
        "object" => validate_object(name, value, field),
        "link" => validate_string(name, value, field), // Links are strings at the data level
        "any" => Vec::new(),
        _ => vec![ValidationError {
            field: name.to_string(),
            code: "invalid_type".to_string(),
            message: format!("unknown field type {:?}", field.field_type),
            severity: Severity::Error,
        }],
    }
}

fn validate_string(
    name: &str,
    value: &serde_yaml::Value,
    field: &FieldDef,
) -> Vec<ValidationError> {
    let s = match value.as_str() {
        Some(s) => s,
        None => {
            return vec![ValidationError {
                field: name.to_string(),
                code: "type_mismatch".to_string(),
                message: format!("expected string, got {:?}", value_type_name(value)),
                severity: Severity::Error,
            }];
        }
    };

    let mut errors = Vec::new();

    if let Some(min) = field.min_length
        && s.len() < min
    {
        errors.push(ValidationError {
            field: name.to_string(),
            code: "constraint_violation".to_string(),
            message: format!("string length {} below minimum {}", s.len(), min),
            severity: Severity::Error,
        });
    }

    if let Some(max) = field.max_length
        && s.len() > max
    {
        errors.push(ValidationError {
            field: name.to_string(),
            code: "constraint_violation".to_string(),
            message: format!("string length {} above maximum {}", s.len(), max),
            severity: Severity::Error,
        });
    }

    if let Some(ref pattern) = field.pattern
        && let Ok(re) = Regex::new(pattern)
        && !re.is_match(s)
    {
        errors.push(ValidationError {
            field: name.to_string(),
            code: "constraint_violation".to_string(),
            message: format!("value {:?} does not match pattern {:?}", s, pattern),
            severity: Severity::Error,
        });
    }

    errors
}

fn validate_integer(
    name: &str,
    value: &serde_yaml::Value,
    field: &FieldDef,
) -> Vec<ValidationError> {
    let n = match value {
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i as f64
            } else if let Some(f) = n.as_f64() {
                if f.fract() != 0.0 {
                    return vec![ValidationError {
                        field: name.to_string(),
                        code: "type_mismatch".to_string(),
                        message: format!("expected integer, got float {}", f),
                        severity: Severity::Error,
                    }];
                }
                f
            } else {
                return vec![type_mismatch(name, "integer", value)];
            }
        }
        _ => return vec![type_mismatch(name, "integer", value)],
    };

    validate_numeric_constraints(name, n, field)
}

fn validate_number(
    name: &str,
    value: &serde_yaml::Value,
    field: &FieldDef,
) -> Vec<ValidationError> {
    let n = match value {
        serde_yaml::Value::Number(n) => n.as_f64().unwrap_or(0.0),
        _ => return vec![type_mismatch(name, "number", value)],
    };

    validate_numeric_constraints(name, n, field)
}

fn validate_numeric_constraints(name: &str, n: f64, field: &FieldDef) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if let Some(min) = field.min
        && n < min
    {
        errors.push(ValidationError {
            field: name.to_string(),
            code: "constraint_violation".to_string(),
            message: format!("value {} below minimum {}", n, min),
            severity: Severity::Error,
        });
    }

    if let Some(max) = field.max
        && n > max
    {
        errors.push(ValidationError {
            field: name.to_string(),
            code: "constraint_violation".to_string(),
            message: format!("value {} above maximum {}", n, max),
            severity: Severity::Error,
        });
    }

    errors
}

fn validate_boolean(name: &str, value: &serde_yaml::Value) -> Vec<ValidationError> {
    if value.as_bool().is_none() {
        vec![type_mismatch(name, "boolean", value)]
    } else {
        Vec::new()
    }
}

fn validate_date(name: &str, value: &serde_yaml::Value) -> Vec<ValidationError> {
    let s = match value.as_str() {
        Some(s) => s,
        None => return vec![type_mismatch(name, "date (string)", value)],
    };

    if chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").is_err() {
        vec![ValidationError {
            field: name.to_string(),
            code: "type_mismatch".to_string(),
            message: format!("invalid date format {:?}, expected YYYY-MM-DD", s),
            severity: Severity::Error,
        }]
    } else {
        Vec::new()
    }
}

fn validate_datetime(name: &str, value: &serde_yaml::Value) -> Vec<ValidationError> {
    let s = match value.as_str() {
        Some(s) => s,
        None => return vec![type_mismatch(name, "datetime (string)", value)],
    };

    // Try RFC3339 first, then ISO8601 without timezone.
    if chrono::DateTime::parse_from_rfc3339(s).is_ok() {
        return Vec::new();
    }
    if chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").is_ok() {
        return Vec::new();
    }

    vec![ValidationError {
        field: name.to_string(),
        code: "type_mismatch".to_string(),
        message: format!("invalid datetime format {:?}", s),
        severity: Severity::Error,
    }]
}

fn validate_time(name: &str, value: &serde_yaml::Value) -> Vec<ValidationError> {
    let s = match value.as_str() {
        Some(s) => s,
        None => return vec![type_mismatch(name, "time (string)", value)],
    };

    if chrono::NaiveTime::parse_from_str(s, "%H:%M:%S").is_ok()
        || chrono::NaiveTime::parse_from_str(s, "%H:%M").is_ok()
    {
        Vec::new()
    } else {
        vec![ValidationError {
            field: name.to_string(),
            code: "type_mismatch".to_string(),
            message: format!("invalid time format {:?}, expected HH:MM or HH:MM:SS", s),
            severity: Severity::Error,
        }]
    }
}

fn validate_enum(name: &str, value: &serde_yaml::Value, field: &FieldDef) -> Vec<ValidationError> {
    let s = match value.as_str() {
        Some(s) => s,
        None => {
            return vec![ValidationError {
                field: name.to_string(),
                code: "type_mismatch".to_string(),
                message: format!("expected string for enum, got {:?}", value_type_name(value)),
                severity: Severity::Error,
            }];
        }
    };

    if let Some(ref values) = field.values {
        let matches = values.iter().any(|v| v.eq_ignore_ascii_case(s));
        if !matches {
            return vec![ValidationError {
                field: name.to_string(),
                code: "constraint_violation".to_string(),
                message: format!("value {:?} not in allowed values {:?}", s, values),
                severity: Severity::Error,
            }];
        }
    }

    Vec::new()
}

fn validate_list(name: &str, value: &serde_yaml::Value, field: &FieldDef) -> Vec<ValidationError> {
    let list = match value.as_sequence() {
        Some(list) => list,
        None => return vec![type_mismatch(name, "list", value)],
    };

    let mut errors = Vec::new();

    if let Some(min) = field.min_items
        && list.len() < min
    {
        errors.push(ValidationError {
            field: name.to_string(),
            code: "constraint_violation".to_string(),
            message: format!("list length {} below minimum {}", list.len(), min),
            severity: Severity::Error,
        });
    }

    if let Some(max) = field.max_items
        && list.len() > max
    {
        errors.push(ValidationError {
            field: name.to_string(),
            code: "constraint_violation".to_string(),
            message: format!("list length {} above maximum {}", list.len(), max),
            severity: Severity::Error,
        });
    }

    if let Some(ref items_def) = field.items {
        for (i, item) in list.iter().enumerate() {
            let item_name = format!("{}[{}]", name, i);
            let item_errors = validate_field(&item_name, item, items_def);
            errors.extend(item_errors);
        }
    }

    errors
}

fn validate_object(
    name: &str,
    value: &serde_yaml::Value,
    field: &FieldDef,
) -> Vec<ValidationError> {
    let map = match value.as_mapping() {
        Some(map) => map,
        None => return vec![type_mismatch(name, "object", value)],
    };

    let mut errors = Vec::new();

    if let Some(ref sub_fields) = field.fields {
        // Check required sub-fields.
        for (sub_name, sub_field) in sub_fields {
            if sub_field.required {
                let key = serde_yaml::Value::String(sub_name.clone());
                match map.get(&key) {
                    None | Some(serde_yaml::Value::Null) => {
                        errors.push(ValidationError {
                            field: format!("{}.{}", name, sub_name),
                            code: "missing_required".to_string(),
                            message: format!("required field {:?} is missing", sub_name),
                            severity: Severity::Error,
                        });
                    }
                    _ => {}
                }
            }
        }

        // Validate each sub-field value.
        for (key, val) in map {
            if let Some(key_str) = key.as_str()
                && let Some(sub_field) = sub_fields.get(key_str)
            {
                let sub_name = format!("{}.{}", name, key_str);
                let sub_errors = validate_field(&sub_name, val, sub_field);
                errors.extend(sub_errors);
            }
        }
    }

    errors
}

fn type_mismatch(name: &str, expected: &str, value: &serde_yaml::Value) -> ValidationError {
    ValidationError {
        field: name.to_string(),
        code: "type_mismatch".to_string(),
        message: format!("expected {}, got {:?}", expected, value_type_name(value)),
        severity: Severity::Error,
    }
}

fn value_type_name(value: &serde_yaml::Value) -> &'static str {
    match value {
        serde_yaml::Value::Null => "null",
        serde_yaml::Value::Bool(_) => "boolean",
        serde_yaml::Value::Number(_) => "number",
        serde_yaml::Value::String(_) => "string",
        serde_yaml::Value::Sequence(_) => "list",
        serde_yaml::Value::Mapping(_) => "object",
        serde_yaml::Value::Tagged(_) => "tagged",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;

    fn make_type(name: &str, fields: Vec<(&str, FieldDef)>) -> TypeDef {
        let mut field_map = IndexMap::new();
        for (k, v) in fields {
            field_map.insert(k.to_string(), v);
        }
        TypeDef {
            name: name.to_string(),
            fields: field_map,
            strict: None,
            ..default_typedef()
        }
    }

    fn default_typedef() -> TypeDef {
        TypeDef::default()
    }

    fn fm(pairs: Vec<(&str, serde_yaml::Value)>) -> HashMap<String, serde_yaml::Value> {
        pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
    }

    #[test]
    fn required_field_missing() {
        let td = make_type(
            "note",
            vec![(
                "title",
                FieldDef {
                    required: true,
                    ..Default::default()
                },
            )],
        );

        let errors = validate_record(&HashMap::new(), &td, None);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "missing_required");
    }

    #[test]
    fn required_field_present() {
        let td = make_type(
            "note",
            vec![(
                "title",
                FieldDef {
                    required: true,
                    ..Default::default()
                },
            )],
        );

        let frontmatter = fm(vec![(
            "title",
            serde_yaml::Value::String("Hello".to_string()),
        )]);
        let errors = validate_record(&frontmatter, &td, None);
        assert!(errors.is_empty());
    }

    #[test]
    fn string_min_length() {
        let td = make_type(
            "note",
            vec![(
                "title",
                FieldDef {
                    min_length: Some(5),
                    ..Default::default()
                },
            )],
        );

        let frontmatter = fm(vec![("title", serde_yaml::Value::String("Hi".to_string()))]);
        let errors = validate_record(&frontmatter, &td, None);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "constraint_violation");
    }

    #[test]
    fn enum_valid_value() {
        let td = make_type(
            "note",
            vec![(
                "status",
                FieldDef {
                    field_type: "enum".to_string(),
                    values: Some(vec!["active".to_string(), "done".to_string()]),
                    ..Default::default()
                },
            )],
        );

        let frontmatter = fm(vec![(
            "status",
            serde_yaml::Value::String("active".to_string()),
        )]);
        let errors = validate_record(&frontmatter, &td, None);
        assert!(errors.is_empty());
    }

    #[test]
    fn enum_invalid_value() {
        let td = make_type(
            "note",
            vec![(
                "status",
                FieldDef {
                    field_type: "enum".to_string(),
                    values: Some(vec!["active".to_string(), "done".to_string()]),
                    ..Default::default()
                },
            )],
        );

        let frontmatter = fm(vec![(
            "status",
            serde_yaml::Value::String("unknown".to_string()),
        )]);
        let errors = validate_record(&frontmatter, &td, None);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "constraint_violation");
    }

    #[test]
    fn strict_forbid_unknown_field() {
        let mut td = make_type("note", vec![("title", FieldDef::default())]);
        td.strict = Some(StrictMode::Forbid);

        let frontmatter = fm(vec![
            ("title", serde_yaml::Value::String("Hi".to_string())),
            ("unknown", serde_yaml::Value::String("wat".to_string())),
        ]);
        let errors = validate_record(&frontmatter, &td, None);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "unknown_field");
        assert_eq!(errors[0].severity, Severity::Error);
    }

    #[test]
    fn strict_warn_unknown_field() {
        let mut td = make_type("note", vec![("title", FieldDef::default())]);
        td.strict = Some(StrictMode::Warn);

        let frontmatter = fm(vec![
            ("title", serde_yaml::Value::String("Hi".to_string())),
            ("extra", serde_yaml::Value::String("ok".to_string())),
        ]);
        let errors = validate_record(&frontmatter, &td, None);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].severity, Severity::Warning);
    }

    #[test]
    fn list_validation() {
        let td = make_type(
            "note",
            vec![(
                "tags",
                FieldDef {
                    field_type: "list".to_string(),
                    items: Some(Box::new(FieldDef::default())), // string items
                    min_items: Some(1),
                    ..Default::default()
                },
            )],
        );

        let frontmatter = fm(vec![("tags", serde_yaml::Value::Sequence(vec![]))]);
        let errors = validate_record(&frontmatter, &td, None);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("below minimum"));
    }

    #[test]
    fn integer_type_mismatch() {
        let td = make_type(
            "note",
            vec![(
                "count",
                FieldDef {
                    field_type: "integer".to_string(),
                    ..Default::default()
                },
            )],
        );

        let frontmatter = fm(vec![(
            "count",
            serde_yaml::Value::String("not a number".to_string()),
        )]);
        let errors = validate_record(&frontmatter, &td, None);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "type_mismatch");
    }

    #[test]
    fn date_validation() {
        let td = make_type(
            "note",
            vec![(
                "due",
                FieldDef {
                    field_type: "date".to_string(),
                    ..Default::default()
                },
            )],
        );

        // Valid date.
        let frontmatter = fm(vec![(
            "due",
            serde_yaml::Value::String("2026-04-15".to_string()),
        )]);
        assert!(validate_record(&frontmatter, &td, None).is_empty());

        // Invalid date.
        let frontmatter = fm(vec![(
            "due",
            serde_yaml::Value::String("not-a-date".to_string()),
        )]);
        assert_eq!(validate_record(&frontmatter, &td, None).len(), 1);
    }
}
