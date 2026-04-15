use std::collections::HashMap;

use super::types::TypeDef;

/// Resolve type inheritance for all loaded types.
/// Each type that declares `extends` gets its parent's fields merged in.
/// Fields declared in the child override parent fields.
/// Detects cycles and missing parents.
pub fn resolve_inheritance(types: &mut HashMap<String, TypeDef>) -> Result<(), String> {
    // Build a dependency order via topological sort.
    let order = topological_sort(types)?;

    // Process in order so parents are resolved before children.
    for name in order {
        let parent_name = {
            let td = types.get(&name).unwrap();
            td.extends.clone()
        };

        if let Some(ref parent) = parent_name {
            let parent_fields = {
                let parent_td = types.get(parent).ok_or_else(|| {
                    format!("type {:?} extends {:?}, which does not exist", name, parent)
                })?;
                parent_td.effective_fields().clone()
            };

            let td = types.get_mut(&name).unwrap();
            let mut resolved = parent_fields;
            // Child fields override parent fields.
            for (k, v) in td.fields.iter() {
                resolved.insert(k.clone(), v.clone());
            }
            td.resolved_fields = Some(resolved);
        }
    }

    Ok(())
}

/// Topological sort of types by their `extends` dependency.
/// Returns names in dependency order (parents before children).
fn topological_sort(types: &HashMap<String, TypeDef>) -> Result<Vec<String>, String> {
    #[derive(Clone, Copy, PartialEq)]
    enum State {
        Unvisited,
        Visiting,
        Visited,
    }

    let mut state: HashMap<String, State> = types.keys().map(|k| (k.clone(), State::Unvisited)).collect();
    let mut order = Vec::with_capacity(types.len());

    fn visit(
        name: &str,
        types: &HashMap<String, TypeDef>,
        state: &mut HashMap<String, State>,
        order: &mut Vec<String>,
    ) -> Result<(), String> {
        match state.get(name) {
            Some(State::Visited) => return Ok(()),
            Some(State::Visiting) => {
                return Err(format!("inheritance cycle detected involving type {:?}", name));
            }
            _ => {}
        }

        state.insert(name.to_string(), State::Visiting);

        if let Some(td) = types.get(name)
            && let Some(ref parent) = td.extends
                && types.contains_key(parent) {
                    visit(parent, types, state, order)?;
                }
                // If parent doesn't exist, we'll catch it during resolution.

        state.insert(name.to_string(), State::Visited);
        order.push(name.to_string());
        Ok(())
    }

    let names: Vec<String> = types.keys().cloned().collect();
    for name in &names {
        if state[name] == State::Unvisited {
            visit(name, types, &mut state, &mut order)?;
        }
    }

    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::FieldDef;
    use indexmap::IndexMap;

    fn make_type(name: &str, extends: Option<&str>, fields: Vec<(&str, FieldDef)>) -> TypeDef {
        let mut field_map = IndexMap::new();
        for (k, v) in fields {
            field_map.insert(k.to_string(), v);
        }
        TypeDef {
            name: name.to_string(),
            description: None,
            extends: extends.map(|s| s.to_string()),
            fields: field_map,
            strict: None,
            r#match: None,
            path_pattern: None,
            display_name_key: None,
            resolved_fields: None,
            body: String::new(),
        }
    }

    fn string_field(required: bool) -> FieldDef {
        FieldDef {
            field_type: "string".to_string(),
            required,
            ..Default::default()
        }
    }

    #[test]
    fn simple_inheritance() {
        let mut types = HashMap::new();
        types.insert(
            "base".to_string(),
            make_type("base", None, vec![("title", string_field(true))]),
        );
        types.insert(
            "child".to_string(),
            make_type("child", Some("base"), vec![("status", string_field(false))]),
        );

        resolve_inheritance(&mut types).unwrap();

        let child = &types["child"];
        let eff = child.effective_fields();
        assert!(eff.contains_key("title"));
        assert!(eff.contains_key("status"));
        assert!(eff["title"].required);
    }

    #[test]
    fn child_overrides_parent_field() {
        let mut types = HashMap::new();
        types.insert(
            "base".to_string(),
            make_type("base", None, vec![("title", string_field(true))]),
        );
        types.insert(
            "child".to_string(),
            make_type("child", Some("base"), vec![("title", string_field(false))]),
        );

        resolve_inheritance(&mut types).unwrap();

        let child = &types["child"];
        assert!(!child.effective_fields()["title"].required);
    }

    #[test]
    fn detect_cycle() {
        let mut types = HashMap::new();
        types.insert("a".to_string(), make_type("a", Some("b"), vec![]));
        types.insert("b".to_string(), make_type("b", Some("a"), vec![]));

        let err = resolve_inheritance(&mut types).unwrap_err();
        assert!(err.contains("cycle"));
    }

    #[test]
    fn no_inheritance() {
        let mut types = HashMap::new();
        types.insert(
            "note".to_string(),
            make_type("note", None, vec![("title", string_field(true))]),
        );

        resolve_inheritance(&mut types).unwrap();

        let note = &types["note"];
        assert!(note.resolved_fields.is_none());
        assert!(note.effective_fields().contains_key("title"));
    }
}
