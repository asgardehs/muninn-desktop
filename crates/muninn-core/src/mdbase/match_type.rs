use std::collections::HashMap;
use std::path::Path;

use super::config::MdbaseConfig;
use super::types::TypeDef;

/// Determine which types apply to a file based on its path and frontmatter.
/// Returns matched types: explicit declarations first, then match rule results.
pub fn match_types<'a>(
    file_path: &Path,
    frontmatter: &HashMap<String, serde_yaml::Value>,
    types: &'a HashMap<String, TypeDef>,
    cfg: Option<&MdbaseConfig>,
) -> Vec<&'a TypeDef> {
    let type_keys = if let Some(cfg) = cfg {
        if cfg.settings.explicit_type_keys.is_empty() {
            vec!["type".to_string(), "types".to_string()]
        } else {
            cfg.settings.explicit_type_keys.clone()
        }
    } else {
        vec!["type".to_string(), "types".to_string()]
    };

    // Check explicit type declaration.
    let explicit = resolve_explicit_types(frontmatter, &type_keys, types);
    if !explicit.is_empty() {
        return explicit;
    }

    // Fall back to match rules.
    resolve_match_rules(file_path, frontmatter, types)
}

fn resolve_explicit_types<'a>(
    frontmatter: &HashMap<String, serde_yaml::Value>,
    type_keys: &[String],
    types: &'a HashMap<String, TypeDef>,
) -> Vec<&'a TypeDef> {
    for key in type_keys {
        let val = match frontmatter.get(key) {
            Some(v) if !v.is_null() => v,
            _ => continue,
        };

        let names: Vec<String> = match val {
            serde_yaml::Value::String(s) => vec![s.clone()],
            serde_yaml::Value::Sequence(seq) => seq
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
            _ => continue,
        };

        let matched: Vec<&TypeDef> = names
            .iter()
            .filter_map(|name| {
                let normalized = name.to_lowercase().trim().to_string();
                types.get(&normalized)
            })
            .collect();

        if !matched.is_empty() {
            return matched;
        }
    }

    Vec::new()
}

fn resolve_match_rules<'a>(
    file_path: &Path,
    frontmatter: &HashMap<String, serde_yaml::Value>,
    types: &'a HashMap<String, TypeDef>,
) -> Vec<&'a TypeDef> {
    let mut matched = Vec::new();

    for td in types.values() {
        let rule = match &td.r#match {
            Some(rule) => rule,
            None => continue,
        };

        if matches_rule(file_path, frontmatter, rule) {
            matched.push(td);
        }
    }

    matched
}

fn matches_rule(
    file_path: &Path,
    frontmatter: &HashMap<String, serde_yaml::Value>,
    rule: &super::types::MatchRule,
) -> bool {
    // Check path_glob.
    if let Some(ref pattern) = rule.path_glob {
        let path_str = file_path.to_str().unwrap_or("");
        if let Ok(glob) = glob::Pattern::new(pattern) {
            if !glob.matches(path_str) {
                return false;
            }
        } else {
            return false;
        }
    }

    // Check fields_present.
    if let Some(ref fields) = rule.fields_present {
        for field in fields {
            if !frontmatter.contains_key(field) {
                return false;
            }
        }
    }

    // Check where conditions.
    if let Some(ref where_conds) = rule.r#where {
        for (field, cond) in where_conds {
            let val = match frontmatter.get(field) {
                Some(v) => v,
                None => return false,
            };

            if !matches_condition(val, cond) {
                return false;
            }
        }
    }

    true
}

fn matches_condition(val: &serde_yaml::Value, cond: &super::types::WhereCond) -> bool {
    if let Some(ref eq) = cond.eq
        && val != eq
    {
        return false;
    }

    if let Some(ref ne) = cond.ne
        && val == ne
    {
        return false;
    }

    if let Some(ref contains) = cond.contains {
        match val.as_str() {
            Some(s) => {
                if !s.contains(contains.as_str()) {
                    return false;
                }
            }
            None => return false,
        }
    }

    if let Some(ref starts) = cond.starts_with {
        match val.as_str() {
            Some(s) => {
                if !s.starts_with(starts.as_str()) {
                    return false;
                }
            }
            None => return false,
        }
    }

    if let Some(ref in_values) = cond.r#in
        && !in_values.contains(val)
    {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::super::types::MatchRule;
    use super::*;

    fn make_type_with_match(name: &str, rule: MatchRule) -> TypeDef {
        TypeDef {
            name: name.to_string(),
            r#match: Some(rule),
            ..Default::default()
        }
    }

    fn fm(pairs: Vec<(&str, serde_yaml::Value)>) -> HashMap<String, serde_yaml::Value> {
        pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
    }

    #[test]
    fn explicit_type_match() {
        let mut types = HashMap::new();
        types.insert(
            "note".to_string(),
            TypeDef {
                name: "note".to_string(),
                ..Default::default()
            },
        );

        let frontmatter = fm(vec![(
            "type",
            serde_yaml::Value::String("note".to_string()),
        )]);
        let matched = match_types(Path::new("test.md"), &frontmatter, &types, None);
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].name, "note");
    }

    #[test]
    fn fields_present_match() {
        let mut types = HashMap::new();
        types.insert(
            "journal".to_string(),
            make_type_with_match(
                "journal",
                MatchRule {
                    path_glob: None,
                    fields_present: Some(vec!["mood".to_string(), "weather".to_string()]),
                    r#where: None,
                },
            ),
        );

        let frontmatter = fm(vec![
            ("mood", serde_yaml::Value::String("good".to_string())),
            ("weather", serde_yaml::Value::String("sunny".to_string())),
        ]);
        let matched = match_types(Path::new("test.md"), &frontmatter, &types, None);
        assert_eq!(matched.len(), 1);
    }

    #[test]
    fn fields_present_no_match() {
        let mut types = HashMap::new();
        types.insert(
            "journal".to_string(),
            make_type_with_match(
                "journal",
                MatchRule {
                    path_glob: None,
                    fields_present: Some(vec!["mood".to_string()]),
                    r#where: None,
                },
            ),
        );

        let matched = match_types(Path::new("test.md"), &HashMap::new(), &types, None);
        assert!(matched.is_empty());
    }

    #[test]
    fn path_glob_match() {
        let mut types = HashMap::new();
        types.insert(
            "journal".to_string(),
            make_type_with_match(
                "journal",
                MatchRule {
                    path_glob: Some("journal/*.md".to_string()),
                    fields_present: None,
                    r#where: None,
                },
            ),
        );

        let matched = match_types(
            Path::new("journal/2026-04-15.md"),
            &HashMap::new(),
            &types,
            None,
        );
        assert_eq!(matched.len(), 1);
    }
}
