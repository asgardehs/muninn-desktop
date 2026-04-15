use std::path::PathBuf;

use crate::markdown::Note;

/// A search result with relevance score.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub path: PathBuf,
    pub title: String,
    pub score: i32,
    pub snippet: Option<String>,
}

/// Score and rank notes against a search query.
/// Scoring model (from Go version):
/// - Title match: 3 points per query word
/// - Tag match: 2 points per query word
/// - Body match: 1 point per query word
pub fn search_notes(notes: &[Note], query: &str) -> Vec<SearchResult> {
    let words: Vec<String> = query
        .split_whitespace()
        .map(|w| w.to_lowercase())
        .filter(|w| !w.is_empty())
        .collect();

    if words.is_empty() {
        return Vec::new();
    }

    let mut results: Vec<SearchResult> = notes
        .iter()
        .filter_map(|note| {
            let score = score_note(note, &words);
            if score > 0 {
                let snippet = find_snippet(note, &words);
                Some(SearchResult {
                    path: note.path.clone(),
                    title: note.title.clone(),
                    score,
                    snippet,
                })
            } else {
                None
            }
        })
        .collect();

    results.sort_by(|a, b| b.score.cmp(&a.score));
    results
}

fn score_note(note: &Note, words: &[String]) -> i32 {
    let mut score = 0i32;
    let title_lower = note.title.to_lowercase();
    let body_lower = note.body.to_lowercase();

    for word in words {
        // Title match: 3 points.
        if title_lower.contains(word.as_str()) {
            score += 3;
        }

        // Tag match: 2 points.
        for tag in &note.tags {
            if tag.to_lowercase().contains(word.as_str()) {
                score += 2;
                break;
            }
        }

        // Body match: 1 point.
        if body_lower.contains(word.as_str()) {
            score += 1;
        }
    }

    score
}

fn find_snippet(note: &Note, words: &[String]) -> Option<String> {
    let body_lower = note.body.to_lowercase();

    // Find the first matching word position.
    let mut first_pos = None;
    for word in words {
        if let Some(pos) = body_lower.find(word.as_str()) {
            first_pos = Some(match first_pos {
                Some(existing) if existing < pos => existing,
                _ => pos,
            });
        }
    }

    let pos = first_pos?;

    // Extract a snippet around the match.
    let start = pos.saturating_sub(40);
    let end = (pos + 80).min(note.body.len());

    // Align to word boundaries.
    let snippet = &note.body[start..end];
    let snippet = snippet.trim();

    Some(if start > 0 {
        format!("...{}", snippet)
    } else {
        snippet.to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_note(title: &str, body: &str, tags: Vec<&str>) -> Note {
        Note {
            path: PathBuf::from(format!("{}.md", title.to_lowercase().replace(' ', "-"))),
            title: title.to_string(),
            frontmatter: HashMap::new(),
            raw_frontmatter: String::new(),
            body: body.to_string(),
            tags: tags.into_iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn title_match_scores_highest() {
        let notes = vec![
            make_note("Rust Guide", "A guide about programming.", vec![]),
            make_note("Programming", "This mentions rust in the body.", vec![]),
        ];

        let results = search_notes(&notes, "rust");
        assert_eq!(results.len(), 2);
        // Title match (3) + body match (1) vs body match (1) only
        assert_eq!(results[0].title, "Rust Guide");
        assert!(results[0].score > results[1].score);
    }

    #[test]
    fn tag_match_scores_medium() {
        let notes = vec![
            make_note("Note A", "Nothing relevant.", vec!["rust"]),
            make_note("Note B", "This mentions rust.", vec![]),
        ];

        let results = search_notes(&notes, "rust");
        assert_eq!(results.len(), 2);
        // Tag match (2) vs body match (1)
        assert!(results[0].score >= results[1].score);
    }

    #[test]
    fn empty_query_returns_empty() {
        let notes = vec![make_note("Test", "body", vec![])];
        assert!(search_notes(&notes, "").is_empty());
        assert!(search_notes(&notes, "   ").is_empty());
    }

    #[test]
    fn no_match_filtered_out() {
        let notes = vec![make_note("Test", "body text", vec![])];
        let results = search_notes(&notes, "nonexistent");
        assert!(results.is_empty());
    }

    #[test]
    fn case_insensitive() {
        let notes = vec![make_note("UPPER", "BODY TEXT", vec!["TAG"])];
        let results = search_notes(&notes, "upper body tag");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].score, 3 + 1 + 2); // title + body + tag for "upper"; body for "body"; tag for "tag"
    }
}
