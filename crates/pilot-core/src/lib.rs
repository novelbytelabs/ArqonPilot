use chrono::Utc;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoOutcome {
    pub repo: String,
    pub path: String,
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub timestamp: String,
    pub command: String,
    pub dry_run: bool,
    pub success: bool,
    pub summary: String,
    pub repo_count: usize,
    pub failures: usize,
    pub artifact_path: Option<String>,
}

pub fn write_repo_outcomes_artifact(
    command: &str,
    outcomes: &[RepoOutcome],
) -> std::io::Result<PathBuf> {
    let dir = default_reports_dir();
    std::fs::create_dir_all(&dir)?;
    let stamp = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let safe = command.replace('.', "_");
    let path = dir.join(format!("{}_{}.json", safe, stamp));
    let body =
        serde_json::to_string_pretty(outcomes).map_err(|e| std::io::Error::other(e.to_string()))?;
    std::fs::write(&path, body)?;
    Ok(path)
}

pub fn append_audit_event(mut event: AuditEvent) -> std::io::Result<PathBuf> {
    if event.timestamp.is_empty() {
        event.timestamp = Utc::now().to_rfc3339();
    }
    let dir = default_pilot_home();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("audit.jsonl");
    let line = serde_json::to_string(&event).map_err(|e| std::io::Error::other(e.to_string()))?;
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    writeln!(file, "{}", line)?;
    Ok(path)
}

fn default_reports_dir() -> PathBuf {
    default_pilot_home().join("reports")
}

fn default_pilot_home() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".pilot")
}
