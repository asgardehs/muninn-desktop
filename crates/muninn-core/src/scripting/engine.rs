//! Public scripting entry point.
//!
//! `ScriptEngine` wraps an `Arc<Vault>` and builds fresh `rhai::Engine`
//! instances per run so each run gets its own output buffer and timeout
//! clock. Building an engine is cheap (no VM warmup, all stack-allocated
//! data structures), so per-run construction is acceptable overhead.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rhai::{Engine, module_resolvers::FileModuleResolver};
use thiserror::Error;

use crate::vault::Vault;

use super::{MAX_CALL_LEVELS, MAX_OPERATIONS, TIMEOUT_SECS, functions};

#[derive(Debug, Error)]
pub enum ScriptError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("eval error: {0}")]
    Eval(String),
    #[error("script timeout exceeded {0}s")]
    Timeout(u64),
}

/// Accumulated stdout-like output from a script run. Populated by `print`,
/// `table`, and `list` built-ins.
#[derive(Debug, Default, Clone)]
pub struct ScriptOutput {
    pub text: String,
}

pub struct ScriptEngine {
    vault: Arc<Vault>,
    scripts_dir: Option<PathBuf>,
}

impl ScriptEngine {
    pub fn new(vault: Arc<Vault>) -> Self {
        let scripts_dir = vault.root().join(".muninn/scripts");
        let scripts_dir = if scripts_dir.exists() {
            Some(scripts_dir)
        } else {
            None
        };
        Self {
            vault,
            scripts_dir,
        }
    }

    /// Override the module resolver directory (used in tests).
    pub fn with_scripts_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.scripts_dir = Some(dir.into());
        self
    }

    pub fn run(&self, source: &str) -> Result<ScriptOutput, ScriptError> {
        let output = Arc::new(Mutex::new(String::new()));
        let engine = self.build_engine(Arc::clone(&output));
        let result = engine.run(source);
        let text = output.lock().unwrap().clone();
        match result {
            Ok(()) => Ok(ScriptOutput { text }),
            Err(e) => classify_error(e, text),
        }
    }

    pub fn run_file(&self, path: &Path) -> Result<ScriptOutput, ScriptError> {
        let source = std::fs::read_to_string(path)?;
        self.run(&source)
    }

    fn build_engine(&self, output: Arc<Mutex<String>>) -> Engine {
        let mut engine = Engine::new();
        engine.set_max_operations(MAX_OPERATIONS);
        engine.set_max_call_levels(MAX_CALL_LEVELS);

        // Timeout — the progress callback fires after every operation. Returning
        // `Some(..)` terminates the script; we tag the value so `classify_error`
        // can surface it as `ScriptError::Timeout`.
        let start = Instant::now();
        let timeout = Duration::from_secs(TIMEOUT_SECS);
        engine.on_progress(move |_ops| {
            if start.elapsed() > timeout {
                Some(rhai::Dynamic::from(TIMEOUT_MARKER.to_string()))
            } else {
                None
            }
        });

        // print() appends to the shared output buffer.
        {
            let out = Arc::clone(&output);
            engine.on_print(move |s| {
                let mut buf = out.lock().unwrap();
                buf.push_str(s);
                buf.push('\n');
            });
        }

        if let Some(ref dir) = self.scripts_dir {
            engine.set_module_resolver(FileModuleResolver::new_with_path(dir.clone()));
        }

        functions::register_all(&mut engine, Arc::clone(&self.vault), Arc::clone(&output));

        engine
    }
}

const TIMEOUT_MARKER: &str = "__muninn_script_timeout__";

fn classify_error(
    e: Box<rhai::EvalAltResult>,
    _captured: String,
) -> Result<ScriptOutput, ScriptError> {
    // Rhai's timeout surfaces as `Terminated(Dynamic, Position)`. Check the
    // carried value against our marker to convert it into a typed error.
    if let rhai::EvalAltResult::ErrorTerminated(ref val, _) = *e
        && val.clone().into_string().ok().as_deref() == Some(TIMEOUT_MARKER)
    {
        return Err(ScriptError::Timeout(TIMEOUT_SECS));
    }
    if matches!(
        *e,
        rhai::EvalAltResult::ErrorParsing(_, _)
    ) {
        return Err(ScriptError::Parse(e.to_string()));
    }
    Err(ScriptError::Eval(e.to_string()))
}
