use crate::parser_rust::TestFailure;
use anyhow::Result;
use pilot_oracle::OracleStore;
use std::fs;

pub struct ContextBuilder {
    store: OracleStore,
    root: std::path::PathBuf,
}

#[derive(Debug)]
pub struct HealContext {
    pub failure: TestFailure,
    pub source_snippet: String,
    pub related_signatures: Vec<String>,
}

impl ContextBuilder {
    pub fn new(store: OracleStore, root: std::path::PathBuf) -> Self {
        Self { store, root }
    }

    pub fn build_context(&self, failure: &TestFailure) -> Result<HealContext> {
        // 1. Read source file
        let file_path = self.root.join(&failure.file_path);
        let source = if file_path.exists() {
            fs::read_to_string(&file_path)?
        } else {
            String::new()
        };

        // 2. Extract snippet around the failing line (if known)
        let snippet = if let Some(line) = failure.line {
            let lines: Vec<&str> = source.lines().collect();
            let start = (line as usize).saturating_sub(5);
            let end = (line as usize + 10).min(lines.len());
            lines[start..end].join("\n")
        } else {
            // Take first 50 lines if no line number
            source.lines().take(50).collect::<Vec<_>>().join("\n")
        };

        // 3. Query store for related function signatures
        let related_signatures = self
            .store
            .get_related_signatures(&failure.file_path, failure.line);

        Ok(HealContext {
            failure: failure.clone(),
            source_snippet: snippet,
            related_signatures,
        })
    }
}
