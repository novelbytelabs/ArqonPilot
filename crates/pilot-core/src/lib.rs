use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoContext {
    pub root: PathBuf,
}

impl RepoContext {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandReport {
    pub command: String,
    pub success: bool,
    pub summary: String,
}

impl CommandReport {
    pub fn ok(command: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            success: true,
            summary: summary.into(),
        }
    }

    pub fn err(command: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            success: false,
            summary: summary.into(),
        }
    }
}
