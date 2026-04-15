use std::collections::HashMap;
use std::path::PathBuf;

use muninn_core::vault::{Vault, NoteFilter};

fn test_vault_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("testdata/test-vault")
}

#[test]
fn open_vault() {
    let vault = Vault::open(test_vault_path()).unwrap();
    assert!(vault.config().is_some());
    assert!(!vault.types().is_empty());
}

#[test]
fn read_note() {
    let vault = Vault::open(test_vault_path()).unwrap();
    let note = vault.read_note(std::path::Path::new("projects/plant-ops.md")).unwrap();
    assert_eq!(note.title, "Plant Operations");
    assert!(note.tags.contains(&"safety".to_string()));
}

#[test]
fn list_all_notes() {
    let vault = Vault::open(test_vault_path()).unwrap();
    let notes = vault.list_notes(&NoteFilter::new()).unwrap();
    // Should find plant-ops, 2026-04-15, osha-standards (not _index.md)
    assert!(notes.len() >= 3, "Expected at least 3 notes, got {}", notes.len());
}

#[test]
fn list_notes_filtered_by_type() {
    let vault = Vault::open(test_vault_path()).unwrap();
    let notes = vault.list_notes(&NoteFilter::new().with_type("journal")).unwrap();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].title, "April 15, 2026");
}

#[test]
fn list_notes_filtered_by_tag() {
    let vault = Vault::open(test_vault_path()).unwrap();
    let notes = vault.list_notes(&NoteFilter::new().with_tag("safety")).unwrap();
    assert!(notes.len() >= 2);
}

#[test]
fn search_notes() {
    let vault = Vault::open(test_vault_path()).unwrap();
    let results = vault.search("operations", None).unwrap();
    assert!(!results.is_empty());
    // "Plant Operations" should score highest (title match).
    assert_eq!(results[0].title, "Plant Operations");
}

#[test]
fn search_with_filter() {
    let vault = Vault::open(test_vault_path()).unwrap();
    let filter = NoteFilter::new().with_type("journal");
    let results = vault.search("muninn", Some(&filter)).unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn validate_note() {
    let vault = Vault::open(test_vault_path()).unwrap();
    let errors = vault.validate(std::path::Path::new("projects/plant-ops.md")).unwrap();
    // plant-ops has all required fields, should pass validation.
    assert!(errors.is_empty(), "Unexpected validation errors: {:?}", errors.iter().map(|e| e.to_string()).collect::<Vec<_>>());
}

#[test]
fn validate_all() {
    let vault = Vault::open(test_vault_path()).unwrap();
    let results = vault.validate_all().unwrap();
    // No validation errors expected in the test vault.
    for (path, errors) in &results {
        println!("{}: {:?}", path.display(), errors.iter().map(|e| e.to_string()).collect::<Vec<_>>());
    }
}

#[test]
fn collect_tags() {
    let vault = Vault::open(test_vault_path()).unwrap();
    let tags = vault.collect_tags().unwrap();
    assert!(!tags.is_empty());
    // "safety" should appear in multiple notes.
    let safety = tags.iter().find(|t| t.tag == "safety");
    assert!(safety.is_some());
    assert!(safety.unwrap().count >= 2);
}

#[test]
fn backlinks() {
    let vault = Vault::open(test_vault_path()).unwrap();
    let bl = vault.backlinks(std::path::Path::new("projects/plant-ops.md"));
    // osha-standards and journal entry both link to plant-ops.
    assert!(bl.len() >= 2, "Expected at least 2 backlinks, got {:?}", bl);
}

#[test]
fn wikilink_extraction() {
    let links = muninn_core::wikilink::extract("See [[note1]] and [[folder/]] and ![[image.png]].");
    assert_eq!(links.len(), 3);

    assert_eq!(links[0].target, "note1");
    assert!(!links[0].is_folder_link);
    assert!(!links[0].is_embed);

    assert_eq!(links[1].target, "folder");
    assert!(links[1].is_folder_link);

    assert_eq!(links[2].target, "image.png");
    assert!(links[2].is_embed);
}

#[test]
fn type_loading_with_inheritance() {
    let vault = Vault::open(test_vault_path()).unwrap();
    let types = vault.types();

    let journal = &types["journal"];
    let eff = journal.effective_fields();

    // Journal extends note, so it should have note's fields + its own.
    assert!(eff.contains_key("title"), "Missing inherited field 'title'");
    assert!(eff.contains_key("tags"), "Missing inherited field 'tags'");
    assert!(eff.contains_key("date"), "Missing own field 'date'");
    assert!(eff.contains_key("mood"), "Missing own field 'mood'");
}

#[test]
fn create_and_read_note() {
    let tmp = tempfile::TempDir::new().unwrap();
    let vault_dir = tmp.path().to_path_buf();

    // Create a minimal vault structure.
    std::fs::create_dir_all(vault_dir.join(".muninn/types")).unwrap();
    std::fs::write(
        vault_dir.join(".muninn/config.yaml"),
        "name: temp-vault\n",
    ).unwrap();

    let vault = Vault::open(&vault_dir).unwrap();
    let path = vault.create_note("My Test Note", None, HashMap::new()).unwrap();

    assert!(path.exists());
    let note = vault.read_note(&path).unwrap();
    assert_eq!(note.title, "My Test Note");
}

#[test]
fn grammar_checker_basic() {
    let checker = muninn_core::grammar::GrammarChecker::new(None);
    let diagnostics = checker.check("This is a correct sentence.");
    // A correct sentence should produce few or no diagnostics.
    // (Harper may still flag style issues, so we just verify it runs.)
    let _ = diagnostics;
}

#[test]
fn custom_dictionary() {
    let tmp = tempfile::TempDir::new().unwrap();
    let dict_path = tmp.path().join("dictionary.txt");
    std::fs::write(&dict_path, "muninn\nosha\n").unwrap();

    let checker = muninn_core::grammar::GrammarChecker::new(Some(&dict_path));
    // "muninn" is in the custom dictionary, so it shouldn't be flagged as misspelled.
    let diags = checker.check("The muninn system is running.");
    let muninn_flags: Vec<_> = diags.iter().filter(|d| d.message.to_lowercase().contains("muninn")).collect();
    assert!(muninn_flags.is_empty(), "muninn should not be flagged: {:?}", muninn_flags);
}
