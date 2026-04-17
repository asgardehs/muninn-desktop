use std::ops::Range;
use std::path::Path;

use harper_core::linting::{LintGroup, LintGroupConfig, Linter, Suggestion};
use harper_core::parsers::Markdown;
use harper_core::{Document, FstDictionary};

use super::dictionary::Dictionary;

/// Severity of a grammar diagnostic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DiagnosticSeverity {
    /// Spelling error.
    Error,
    /// Grammar suggestion.
    Warning,
}

/// A grammar or spelling diagnostic.
#[derive(Debug, Clone)]
pub struct GrammarDiagnostic {
    /// Character span in the checked text.
    pub span: Range<usize>,
    pub message: String,
    pub suggestions: Vec<String>,
    pub severity: DiagnosticSeverity,
    pub rule: String,
}

/// Grammar and spell checker wrapping harper-core.
pub struct GrammarChecker {
    custom_dict: Dictionary,
}

impl GrammarChecker {
    /// Create a new grammar checker, optionally loading a vault dictionary.
    pub fn new(vault_dict_path: Option<&Path>) -> Self {
        let custom_dict = match vault_dict_path {
            Some(path) => Dictionary::load(path).unwrap_or_default(),
            None => Dictionary::default(),
        };

        Self { custom_dict }
    }

    /// Check note body content for grammar and spelling issues.
    /// Frontmatter and code fences should be stripped before calling this.
    pub fn check(&self, content: &str) -> Vec<GrammarDiagnostic> {
        let body = strip_non_prose(content);

        let dict = FstDictionary::curated();
        let document = Document::new(&body, &Markdown::default(), &dict);
        let mut linter = LintGroup::new(LintGroupConfig::default(), dict);
        let lints = linter.lint(&document);

        let source_chars: Vec<char> = body.chars().collect();
        let mut diagnostics = Vec::new();

        for lint in lints {
            let start = lint.span.start;
            let end = lint.span.end;

            // Extract the flagged word from the char-indexed source.
            let word: String = source_chars[start..end].iter().collect();

            // Skip if word is in custom dictionary.
            if self.custom_dict.contains(&word) {
                continue;
            }

            let suggestions: Vec<String> = lint
                .suggestions
                .iter()
                .filter_map(|s| match s {
                    Suggestion::ReplaceWith(chars) => {
                        let replacement: String = chars.iter().collect();
                        if replacement.is_empty() {
                            None
                        } else {
                            Some(replacement)
                        }
                    }
                    _ => None,
                })
                .collect();

            let severity = DiagnosticSeverity::Warning;

            diagnostics.push(GrammarDiagnostic {
                span: start..end,
                message: lint.message,
                suggestions,
                severity,
                rule: format!("{:?}", lint.lint_kind),
            });
        }

        diagnostics
    }

    /// Add a word to the custom dictionary.
    pub fn add_to_dictionary(
        &mut self,
        word: &str,
    ) -> Result<(), super::dictionary::DictionaryError> {
        self.custom_dict.add(word)
    }
}

/// Strip code fences from content, leaving prose and blank lines for fenced content.
fn strip_non_prose(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let mut in_code_fence = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code_fence = !in_code_fence;
            result.push('\n');
            continue;
        }
        if in_code_fence {
            result.push('\n');
            continue;
        }
        result.push_str(line);
        result.push('\n');
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_code_fences() {
        let input = "Hello world.\n\n```rust\nfn main() {}\n```\n\nMore text.\n";
        let stripped = strip_non_prose(input);
        assert!(stripped.contains("Hello world."));
        assert!(stripped.contains("More text."));
        assert!(!stripped.contains("fn main()"));
    }

    #[test]
    fn checker_creation() {
        let checker = GrammarChecker::new(None);
        // Just verify it doesn't panic.
        let _ = checker.check("This is a simple test sentence.");
    }
}
