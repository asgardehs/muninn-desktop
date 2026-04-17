//! Render a markdown note by evaluating its inline `muninn` code blocks.
//!
//! A fenced block whose language tag is `muninn` is extracted, evaluated via
//! a fresh `ScriptEngine::run`, and its captured output replaces the block
//! in place. Blocks with other language tags (including `muninn-error`) are
//! left untouched.

use regex::Regex;
use std::sync::OnceLock;

use super::{RenderErrorBehavior, ScriptEngine, ScriptError};

/// Matches a fenced block whose opening fence is ``` ```muninn ``` (optionally
/// followed by whitespace), content of any length, closing fence on its own
/// line. Non-greedy so consecutive blocks don't merge.
fn block_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?ms)^```muninn[^\S\n]*\n(.*?)\n```[^\S\n]*$").unwrap()
    })
}

impl ScriptEngine {
    /// Evaluate every `muninn` code block in `source` and splice each block's
    /// captured output back into the text in place of the original block.
    pub fn render(
        &self,
        source: &str,
        on_error: RenderErrorBehavior,
    ) -> Result<String, ScriptError> {
        let re = block_re();
        let mut out = String::with_capacity(source.len());
        let mut cursor = 0;

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let script = cap.get(1).unwrap().as_str();

            out.push_str(&source[cursor..full.start()]);
            cursor = full.end();

            match self.run(script) {
                Ok(output) => {
                    out.push_str(output.text.trim_end_matches('\n'));
                }
                Err(e) => match on_error {
                    RenderErrorBehavior::Abort => return Err(e),
                    RenderErrorBehavior::ReplaceBlock => {
                        out.push_str("```muninn-error\n");
                        out.push_str(&e.to_string());
                        out.push_str("\n```");
                    }
                },
            }
        }

        out.push_str(&source[cursor..]);
        Ok(out)
    }
}
