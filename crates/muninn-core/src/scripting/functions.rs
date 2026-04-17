//! Vault API exposed to Rhai scripts.
//!
//! All functions here are read-only by design. Mutation of the vault stays
//! in the CLI / API / UI — scripts are for viewing and computing.

use std::path::Path;
use std::sync::{Arc, Mutex};

use rhai::{Array, Dynamic, Engine, EvalAltResult, Map};

use crate::vault::{NoteFilter, Vault};

use super::bridge::{note_to_map, result_set_to_array};

type FnResult = Result<Dynamic, Box<EvalAltResult>>;

pub fn register_all(engine: &mut Engine, vault: Arc<Vault>, output: Arc<Mutex<String>>) {
    register_query(engine, Arc::clone(&vault));
    register_search(engine, Arc::clone(&vault));
    register_note(engine, Arc::clone(&vault));
    register_notes(engine, Arc::clone(&vault));
    register_backlinks(engine, Arc::clone(&vault));
    register_tags(engine, Arc::clone(&vault));
    register_types(engine, Arc::clone(&vault));
    register_runestone_stub(engine);

    register_link(engine);
    register_table(engine, Arc::clone(&output));
    register_list(engine, Arc::clone(&output));
    register_json(engine);
}

fn register_query(engine: &mut Engine, vault: Arc<Vault>) {
    let v = Arc::clone(&vault);
    engine.register_fn("query", move |sql: &str| -> FnResult {
        let rs = v.query(sql).map_err(rhai_err)?;
        Ok(Dynamic::from(result_set_to_array(&rs)))
    });
}

fn register_search(engine: &mut Engine, vault: Arc<Vault>) {
    let v = Arc::clone(&vault);
    engine.register_fn("search", move |text: &str| -> FnResult {
        let results = v.search(text, None).map_err(rhai_err)?;
        let arr: Array = results
            .into_iter()
            .map(|r| {
                let mut m = Map::new();
                m.insert("path".into(), Dynamic::from(r.path.display().to_string()));
                m.insert("title".into(), Dynamic::from(r.title));
                m.insert("score".into(), Dynamic::from(r.score as f64));
                Dynamic::from(m)
            })
            .collect();
        Ok(Dynamic::from(arr))
    });
}

fn register_note(engine: &mut Engine, vault: Arc<Vault>) {
    let v = Arc::clone(&vault);
    engine.register_fn("note", move |path: &str| -> FnResult {
        let note = v.read_note(Path::new(path)).map_err(rhai_err)?;
        Ok(Dynamic::from(note_to_map(&note)))
    });
}

fn register_notes(engine: &mut Engine, vault: Arc<Vault>) {
    // `notes(filter_map)` — accepts a Rhai Map with optional `type` / `tag`
    // keys. Empty map (or missing keys) lists everything.
    let v = Arc::clone(&vault);
    engine.register_fn("notes", move |filter: Map| -> FnResult {
        let mut nf = NoteFilter::new();
        if let Some(t) = filter.get("type").and_then(|d| d.clone().into_string().ok()) {
            nf = nf.with_type(&t);
        }
        if let Some(t) = filter.get("tag").and_then(|d| d.clone().into_string().ok()) {
            nf = nf.with_tag(&t);
        }
        let list = v.list_notes(&nf).map_err(rhai_err)?;
        let arr: Array = list
            .into_iter()
            .map(|s| {
                let mut m = Map::new();
                m.insert("path".into(), Dynamic::from(s.path.display().to_string()));
                m.insert("title".into(), Dynamic::from(s.title));
                if let Some(t) = s.note_type {
                    m.insert("type".into(), Dynamic::from(t));
                }
                let tags: Array = s.tags.into_iter().map(Dynamic::from).collect();
                m.insert("tags".into(), Dynamic::from(tags));
                Dynamic::from(m)
            })
            .collect();
        Ok(Dynamic::from(arr))
    });

    // No-argument overload: `notes()` → all notes.
    let v2 = Arc::clone(&vault);
    engine.register_fn("notes", move || -> FnResult {
        let list = v2.list_notes(&NoteFilter::new()).map_err(rhai_err)?;
        let arr: Array = list
            .into_iter()
            .map(|s| {
                let mut m = Map::new();
                m.insert("path".into(), Dynamic::from(s.path.display().to_string()));
                m.insert("title".into(), Dynamic::from(s.title));
                if let Some(t) = s.note_type {
                    m.insert("type".into(), Dynamic::from(t));
                }
                Dynamic::from(m)
            })
            .collect();
        Ok(Dynamic::from(arr))
    });
}

fn register_backlinks(engine: &mut Engine, vault: Arc<Vault>) {
    let v = Arc::clone(&vault);
    engine.register_fn("backlinks", move |path: &str| -> FnResult {
        let paths = v.backlinks(Path::new(path));
        let arr: Array = paths
            .into_iter()
            .map(|p| {
                let mut m = Map::new();
                m.insert("path".into(), Dynamic::from(p.display().to_string()));
                Dynamic::from(m)
            })
            .collect();
        Ok(Dynamic::from(arr))
    });
}

fn register_tags(engine: &mut Engine, vault: Arc<Vault>) {
    let v = Arc::clone(&vault);
    engine.register_fn("tags", move || -> FnResult {
        let tags = v.collect_tags().map_err(rhai_err)?;
        let arr: Array = tags
            .into_iter()
            .map(|t| {
                let mut m = Map::new();
                m.insert("tag".into(), Dynamic::from(t.tag));
                m.insert("count".into(), Dynamic::from(t.count as i64));
                Dynamic::from(m)
            })
            .collect();
        Ok(Dynamic::from(arr))
    });
}

fn register_types(engine: &mut Engine, vault: Arc<Vault>) {
    let v = Arc::clone(&vault);
    engine.register_fn("types", move || -> FnResult {
        let types = v.types();
        let arr: Array = types
            .values()
            .map(|t| {
                let mut m = Map::new();
                m.insert("name".into(), Dynamic::from(t.name.clone()));
                if let Some(ref d) = t.description {
                    m.insert("description".into(), Dynamic::from(d.clone()));
                }
                if let Some(ref e) = t.extends {
                    m.insert("extends".into(), Dynamic::from(e.clone()));
                }
                Dynamic::from(m)
            })
            .collect();
        Ok(Dynamic::from(arr))
    });
}

fn register_runestone_stub(engine: &mut Engine) {
    engine.register_fn("runestone", |_name: &str| -> FnResult {
        Err(Box::new(EvalAltResult::ErrorRuntime(
            "runestone() is not available yet — Runestones ship in Phase 5".into(),
            rhai::Position::NONE,
        )))
    });
}

fn register_link(engine: &mut Engine) {
    // `link("path/to/note.md")` → "[[path/to/note]]" (strip .md suffix).
    engine.register_fn("link", |path: &str| {
        let stem = path.strip_suffix(".md").unwrap_or(path);
        format!("[[{stem}]]")
    });
}

fn register_table(engine: &mut Engine, output: Arc<Mutex<String>>) {
    let out = Arc::clone(&output);
    engine.register_fn("table", move |rows: Array| -> FnResult {
        let rendered = render_markdown_table(&rows);
        let mut buf = out.lock().unwrap();
        if !buf.is_empty() && !buf.ends_with('\n') {
            buf.push('\n');
        }
        buf.push_str(&rendered);
        buf.push('\n');
        Ok(Dynamic::UNIT)
    });
}

fn register_list(engine: &mut Engine, output: Arc<Mutex<String>>) {
    let out = Arc::clone(&output);
    engine.register_fn("list", move |items: Array| -> FnResult {
        let mut buf = out.lock().unwrap();
        for item in items {
            buf.push_str("- ");
            buf.push_str(&dynamic_to_display(&item));
            buf.push('\n');
        }
        Ok(Dynamic::UNIT)
    });
}

fn register_json(engine: &mut Engine) {
    engine.register_fn("json", |value: Dynamic| -> FnResult {
        let v = dynamic_to_json(&value);
        match serde_json::to_string(&v) {
            Ok(s) => Ok(Dynamic::from(s)),
            Err(e) => Err(Box::new(EvalAltResult::ErrorRuntime(
                format!("json: {e}").into(),
                rhai::Position::NONE,
            ))),
        }
    });
}

fn render_markdown_table(rows: &Array) -> String {
    if rows.is_empty() {
        return String::new();
    }

    let header: Vec<String> = match rows[0].as_map_ref() {
        Ok(m) => m.keys().map(|k| k.to_string()).collect(),
        Err(_) => return String::new(),
    };

    if header.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    out.push_str("| ");
    out.push_str(&header.join(" | "));
    out.push_str(" |\n|");
    for _ in &header {
        out.push_str(" --- |");
    }
    out.push('\n');

    for row in rows {
        let map = match row.as_map_ref() {
            Ok(m) => m,
            Err(_) => continue,
        };
        out.push('|');
        for col in &header {
            out.push(' ');
            if let Some(v) = map.get(col.as_str()) {
                out.push_str(&dynamic_to_display(v));
            }
            out.push_str(" |");
        }
        out.push('\n');
    }
    out
}

fn dynamic_to_display(d: &Dynamic) -> String {
    if d.is_unit() {
        return String::new();
    }
    if let Some(s) = d.clone().into_string().ok() {
        return s;
    }
    d.to_string()
}

fn dynamic_to_json(d: &Dynamic) -> serde_json::Value {
    if d.is_unit() {
        return serde_json::Value::Null;
    }
    if let Some(b) = d.clone().try_cast::<bool>() {
        return serde_json::Value::Bool(b);
    }
    if let Some(i) = d.clone().try_cast::<i64>() {
        return serde_json::Value::Number(i.into());
    }
    if let Some(f) = d.clone().try_cast::<f64>() {
        return serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null);
    }
    if let Some(s) = d.clone().try_cast::<String>() {
        return serde_json::Value::String(s);
    }
    if let Some(arr) = d.clone().try_cast::<Array>() {
        return serde_json::Value::Array(arr.iter().map(dynamic_to_json).collect());
    }
    if let Some(map) = d.clone().try_cast::<Map>() {
        let mut out = serde_json::Map::new();
        for (k, v) in map {
            out.insert(k.to_string(), dynamic_to_json(&v));
        }
        return serde_json::Value::Object(out);
    }
    serde_json::Value::String(d.to_string())
}

fn rhai_err<E: std::fmt::Display>(e: E) -> Box<EvalAltResult> {
    Box::new(EvalAltResult::ErrorRuntime(
        e.to_string().into(),
        rhai::Position::NONE,
    ))
}
