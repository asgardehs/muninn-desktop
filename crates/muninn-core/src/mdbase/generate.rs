use indexmap::IndexMap;

use super::types::TypeDef;

/// Recognized generated field strategies.
const VALID_STRATEGIES: &[&str] = &[
    "now",
    "now_on_write",
    "uuid",
    "uuid_short",
    "slug",
    "counter",
];

pub fn is_valid_strategy(s: &str) -> bool {
    VALID_STRATEGIES.contains(&s)
}

/// Apply generated field values to frontmatter.
/// `is_new` indicates whether this is a newly created note (affects which strategies fire).
pub fn apply_generated(
    frontmatter: &mut IndexMap<String, serde_yaml::Value>,
    td: &TypeDef,
    is_new: bool,
) {
    let now = chrono::Utc::now();
    let fields = td.effective_fields();

    for (name, field) in fields {
        let strategy = match &field.generated {
            Some(s) => s.as_str(),
            None => continue,
        };

        match strategy {
            "now" => {
                frontmatter.insert(name.clone(), serde_yaml::Value::String(now.to_rfc3339()));
            }
            "now_on_write" => {
                frontmatter.insert(name.clone(), serde_yaml::Value::String(now.to_rfc3339()));
            }
            "uuid" => {
                if is_new && !frontmatter.contains_key(name) {
                    frontmatter.insert(
                        name.clone(),
                        serde_yaml::Value::String(uuid::Uuid::new_v4().to_string()),
                    );
                }
            }
            "uuid_short" => {
                if is_new && !frontmatter.contains_key(name) {
                    let id = uuid::Uuid::new_v4().to_string();
                    frontmatter
                        .insert(name.clone(), serde_yaml::Value::String(id[..8].to_string()));
                }
            }
            "slug" => {
                if is_new && !frontmatter.contains_key(name) {
                    // Derive slug from title if present.
                    if let Some(serde_yaml::Value::String(title)) = frontmatter.get("title") {
                        frontmatter.insert(
                            name.clone(),
                            serde_yaml::Value::String(slug::slugify(title)),
                        );
                    }
                }
            }
            "counter" => {
                // Counter requires external state — skip in Phase 1.
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::FieldDef;
    use super::*;

    fn make_type(fields: Vec<(&str, FieldDef)>) -> TypeDef {
        let mut field_map = IndexMap::new();
        for (k, v) in fields {
            field_map.insert(k.to_string(), v);
        }
        TypeDef {
            name: "test".to_string(),
            fields: field_map,
            ..Default::default()
        }
    }

    #[test]
    fn uuid_generated_on_new() {
        let td = make_type(vec![(
            "id",
            FieldDef {
                generated: Some("uuid".to_string()),
                ..Default::default()
            },
        )]);

        let mut fm = IndexMap::new();
        apply_generated(&mut fm, &td, true);
        assert!(fm.contains_key("id"));
        let id = fm["id"].as_str().unwrap();
        assert_eq!(id.len(), 36); // UUID format
    }

    #[test]
    fn uuid_not_generated_on_update() {
        let td = make_type(vec![(
            "id",
            FieldDef {
                generated: Some("uuid".to_string()),
                ..Default::default()
            },
        )]);

        let mut fm = IndexMap::new();
        apply_generated(&mut fm, &td, false);
        assert!(!fm.contains_key("id"));
    }

    #[test]
    fn uuid_not_overwritten() {
        let td = make_type(vec![(
            "id",
            FieldDef {
                generated: Some("uuid".to_string()),
                ..Default::default()
            },
        )]);

        let mut fm = IndexMap::new();
        fm.insert(
            "id".to_string(),
            serde_yaml::Value::String("existing".to_string()),
        );
        apply_generated(&mut fm, &td, true);
        assert_eq!(fm["id"].as_str().unwrap(), "existing");
    }

    #[test]
    fn slug_from_title() {
        let td = make_type(vec![(
            "slug",
            FieldDef {
                generated: Some("slug".to_string()),
                ..Default::default()
            },
        )]);

        let mut fm = IndexMap::new();
        fm.insert(
            "title".to_string(),
            serde_yaml::Value::String("My Test Note".to_string()),
        );
        apply_generated(&mut fm, &td, true);
        assert_eq!(fm["slug"].as_str().unwrap(), "my-test-note");
    }

    #[test]
    fn now_on_write_always_updates() {
        let td = make_type(vec![(
            "updated",
            FieldDef {
                generated: Some("now_on_write".to_string()),
                ..Default::default()
            },
        )]);

        let mut fm = IndexMap::new();
        fm.insert(
            "updated".to_string(),
            serde_yaml::Value::String("old".to_string()),
        );
        apply_generated(&mut fm, &td, false);
        assert_ne!(fm["updated"].as_str().unwrap(), "old");
    }
}
