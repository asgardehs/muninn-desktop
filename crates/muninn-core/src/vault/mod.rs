mod list;
mod search;
mod tags;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::RwLock;
use thiserror::Error;

use crate::markdown::{self, Note};
use crate::mdbase::config::MdbaseConfig;
use crate::mdbase::types::TypeDef;
use crate::mdbase::validate::ValidationError;
use crate::wikilink::WikilinkIndex;

pub use list::{NoteFilter, NoteSummary};
pub use search::{SearchResult, search_notes};
pub use tags::TagCount;

#[derive(Debug, Error)]
pub enum VaultError {
    #[error("vault path does not exist: {0}")]
    PathNotFound(PathBuf),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("note not found: {0}")]
    NoteNotFound(PathBuf),
    #[error("parse error: {0}")]
    Parse(#[from] markdown::ParseError),
    #[error("mdbase error: {0}")]
    Mdbase(String),
    #[error("walk error: {0}")]
    Walk(#[from] walkdir::Error),
    #[error("query parse error: {0}")]
    QueryParse(#[from] crate::query::ParseError),
    #[error("query eval error: {0}")]
    QueryEval(#[from] crate::query::EvalError),
}

pub type Result<T> = std::result::Result<T, VaultError>;

pub struct RenameResult {
    pub new_path: PathBuf,
    pub links_updated: usize,
}

pub struct Vault {
    root: PathBuf,
    config: Option<MdbaseConfig>,
    types: HashMap<String, TypeDef>,
    wikilinks: Arc<RwLock<WikilinkIndex>>,
}

impl Vault {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        if !root.exists() {
            return Err(VaultError::PathNotFound(root));
        }

        let muninn_dir = root.join(".muninn");

        let config = if muninn_dir.join("config.yaml").exists() {
            crate::mdbase::config::load_config(&muninn_dir).ok()
        } else {
            None
        };

        let types = if muninn_dir.join("types").exists() {
            crate::mdbase::loader::load_types(&muninn_dir.join("types")).unwrap_or_default()
        } else {
            HashMap::new()
        };

        let wikilinks = Arc::new(RwLock::new(WikilinkIndex::new()));

        let mut vault = Vault {
            root,
            config,
            types,
            wikilinks,
        };

        vault.build_wikilink_index();

        Ok(vault)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn config(&self) -> Option<&MdbaseConfig> {
        self.config.as_ref()
    }

    pub fn types(&self) -> &HashMap<String, TypeDef> {
        &self.types
    }

    pub fn read_note(&self, path: &Path) -> Result<Note> {
        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };

        if !full_path.exists() {
            return Err(VaultError::NoteNotFound(full_path));
        }

        let content = std::fs::read_to_string(&full_path)?;
        let note = markdown::parse_document(&full_path, &content)?;
        Ok(note)
    }

    pub fn create_note(
        &self,
        title: &str,
        type_name: Option<&str>,
        fields: HashMap<String, serde_yaml::Value>,
    ) -> Result<PathBuf> {
        let slug = slug::slugify(title);
        let filename = format!("{}.md", slug);
        let path = self.root.join(&filename);

        let mut frontmatter = indexmap::IndexMap::new();
        frontmatter.insert(
            "title".to_string(),
            serde_yaml::Value::String(title.to_string()),
        );

        if let Some(tn) = type_name {
            frontmatter.insert(
                "type".to_string(),
                serde_yaml::Value::String(tn.to_string()),
            );
        }

        for (k, v) in fields {
            frontmatter.insert(k, v);
        }

        // Apply generated fields if we have a type.
        if let Some(tn) = type_name
            && let Some(td) = self.types.get(tn)
        {
            crate::mdbase::generate::apply_generated(&mut frontmatter, td, true);
        }

        let yaml =
            serde_yaml::to_string(&frontmatter).map_err(|e| VaultError::Mdbase(e.to_string()))?;

        let content = format!("---\n{}---\n", yaml);
        std::fs::write(&path, content)?;

        // Update wikilink index (new note has no links yet, but register it).
        Ok(path)
    }

    pub fn validate(&self, path: &Path) -> Result<Vec<ValidationError>> {
        let note = self.read_note(path)?;
        let matched = crate::mdbase::match_type::match_types(
            path,
            &note.frontmatter,
            &self.types,
            self.config.as_ref(),
        );

        let mut all_errors = Vec::new();
        for td in matched {
            let errors = crate::mdbase::validate::validate_record(
                &note.frontmatter,
                td,
                self.config.as_ref(),
            );
            all_errors.extend(errors);
        }

        Ok(all_errors)
    }

    pub fn validate_all(&self) -> Result<Vec<(PathBuf, Vec<ValidationError>)>> {
        let notes = self.list_note_paths()?;
        let mut results = Vec::new();

        for path in notes {
            let errors = self.validate(&path)?;
            if !errors.is_empty() {
                results.push((path, errors));
            }
        }

        Ok(results)
    }

    pub fn search(&self, query: &str, filter: Option<&NoteFilter>) -> Result<Vec<SearchResult>> {
        let notes = self.read_all_notes()?;
        let filtered: Vec<Note> = if let Some(f) = filter {
            notes
                .into_iter()
                .filter(|n| f.matches(&n.frontmatter, &n.title, &n.tags))
                .collect()
        } else {
            notes
        };
        Ok(search_notes(&filtered, query))
    }

    pub fn list_notes(&self, filter: &NoteFilter) -> Result<Vec<NoteSummary>> {
        let notes = self.read_all_notes()?;
        let summaries = notes
            .into_iter()
            .filter(|n| filter.matches(&n.frontmatter, &n.title, &n.tags))
            .map(|n| NoteSummary {
                path: n.path,
                title: n.title,
                note_type: n
                    .frontmatter
                    .get("type")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                tags: n.tags,
            })
            .collect();
        Ok(summaries)
    }

    pub fn collect_tags(&self) -> Result<Vec<TagCount>> {
        let notes = self.read_all_notes()?;
        let tag_lists: Vec<Vec<String>> = notes.iter().map(|n| n.tags.clone()).collect();
        Ok(tags::collect_tags(&tag_lists))
    }

    pub fn query(&self, sql: &str) -> Result<crate::query::QueryResultSet> {
        let q = crate::query::parse_query(sql)?;
        let rs = crate::query::execute(&self.root, &self.types, self.config.as_ref(), &q)?;
        Ok(rs)
    }

    /// Full replace: write `{frontmatter}` + `{body}` to the given note
    /// path, creating the file if it doesn't exist. Callers should
    /// generally use `create_note` for first-time creation (it applies
    /// generated-field strategies) and reserve this for updates.
    pub fn write_note(
        &self,
        path: &Path,
        frontmatter: &indexmap::IndexMap<String, serde_yaml::Value>,
        body: &str,
    ) -> Result<PathBuf> {
        let abs = self.resolve(path);
        if let Some(parent) = abs.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let fm_yaml = if frontmatter.is_empty() {
            String::new()
        } else {
            serde_yaml::to_string(frontmatter).map_err(|e| VaultError::Mdbase(e.to_string()))?
        };
        let content = if fm_yaml.is_empty() {
            body.to_string()
        } else {
            format!("---\n{fm_yaml}---\n{body}")
        };
        std::fs::write(&abs, content)?;

        // Refresh wikilink index for this file so /api/links reflects the
        // write immediately.
        if let Ok(text) = std::fs::read_to_string(&abs) {
            let links = crate::wikilink::extract(&text);
            let rel = self.relative_path(&abs);
            self.wikilinks.write().update(rel, links);
        }

        Ok(abs)
    }

    /// Delete a note by relative or absolute path. Removes the file and
    /// evicts it from the wikilink index.
    pub fn delete_note(&self, path: &Path) -> Result<()> {
        let abs = self.resolve(path);
        if !abs.exists() {
            return Err(VaultError::NoteNotFound(abs));
        }
        std::fs::remove_file(&abs)?;
        let rel = self.relative_path(&abs);
        self.wikilinks.write().remove(&rel);
        Ok(())
    }

    fn resolve(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        }
    }

    pub fn backlinks(&self, path: &Path) -> Vec<PathBuf> {
        let index = self.wikilinks.read();
        let rel_path = self.relative_path(path);
        index.backlinks_for(&rel_path)
    }

    /// Full wikilink graph as `(source, target_strings)` pairs. Targets are
    /// the raw `[[link]]` target strings — unresolved and may not correspond
    /// to existing notes.
    pub fn link_graph(&self) -> Vec<(PathBuf, Vec<String>)> {
        let index = self.wikilinks.read();
        index
            .sources()
            .into_iter()
            .map(|src| {
                let links = index.forward_links(src);
                let targets: Vec<String> = links.iter().map(|l| l.target.clone()).collect();
                (src.clone(), targets)
            })
            .collect()
    }

    pub fn rename_note(&self, from: &Path, new_title: &str) -> Result<RenameResult> {
        let full_from = if from.is_absolute() {
            from.to_path_buf()
        } else {
            self.root.join(from)
        };

        if !full_from.exists() {
            return Err(VaultError::NoteNotFound(full_from));
        }

        let new_slug = slug::slugify(new_title);
        let new_filename = format!("{}.md", new_slug);
        let new_path = full_from.parent().unwrap_or(&self.root).join(&new_filename);

        std::fs::rename(&full_from, &new_path)?;

        // Update wikilinks that point to the old note.
        let old_target = self.relative_path(&full_from);
        let old_name = old_target
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let new_name = new_slug.as_str();

        let links_updated = self.update_wikilink_references(old_name, new_name)?;

        // Rebuild index for the renamed note.
        let content = std::fs::read_to_string(&new_path)?;
        let links = crate::wikilink::extract(&content);
        let mut index = self.wikilinks.write();
        index.remove(&old_target);
        index.update(self.relative_path(&new_path), links);

        Ok(RenameResult {
            new_path,
            links_updated,
        })
    }

    fn read_all_notes(&self) -> Result<Vec<Note>> {
        let paths = self.list_note_paths()?;
        let mut notes = Vec::new();
        for path in paths {
            match self.read_note(&path) {
                Ok(note) => notes.push(note),
                Err(_) => continue, // Skip unparseable notes.
            }
        }
        Ok(notes)
    }

    fn build_wikilink_index(&mut self) {
        let paths = match self.list_note_paths() {
            Ok(p) => p,
            Err(_) => return,
        };

        let mut index = self.wikilinks.write();
        for path in paths {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let links = crate::wikilink::extract(&content);
                let rel = self.relative_path(&path);
                index.update(rel, links);
            }
        }
    }

    fn relative_path(&self, path: &Path) -> PathBuf {
        path.strip_prefix(&self.root).unwrap_or(path).to_path_buf()
    }

    fn list_note_paths(&self) -> Result<Vec<PathBuf>> {
        let mut paths = Vec::new();
        for entry in walkdir::WalkDir::new(&self.root)
            .into_iter()
            .filter_entry(|e| {
                if e.depth() == 0 {
                    return true;
                }
                let name = e.file_name().to_str().unwrap_or("");
                // Skip .muninn and _attachments directories.
                !name.starts_with('.') && name != "_attachments"
            })
        {
            let entry = entry?;
            if entry.file_type().is_file() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("md") {
                    // Skip _index.md files — they're folder metadata, not regular notes.
                    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if file_name != "_index.md" {
                        paths.push(path.to_path_buf());
                    }
                }
            }
        }
        Ok(paths)
    }

    fn update_wikilink_references(&self, old_name: &str, new_name: &str) -> Result<usize> {
        let mut count = 0;
        let pattern = format!("[[{}]]", old_name);
        let replacement = format!("[[{}]]", new_name);

        let paths = self.list_note_paths()?;
        for path in paths {
            let content = std::fs::read_to_string(&path)?;
            if content.contains(&pattern) {
                let updated = content.replace(&pattern, &replacement);
                std::fs::write(&path, updated)?;
                count += 1;
            }
        }

        Ok(count)
    }
}
