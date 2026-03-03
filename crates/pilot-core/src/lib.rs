use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use sha2::{Sha256, Digest};

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
    pub id: String,
    pub timestamp: String,
    pub scope_id: Option<String>,
    pub domain: String,
    pub action: String,
    pub dry_run: bool,
    pub success: bool,
    pub summary: String,
    pub repo_count: usize,
    pub failures: usize,
    pub artifact_path: Option<String>,
    pub repos: Vec<String>,
    pub details: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_hash: Option<String>,
}

impl AuditEvent {
    pub fn compute_content_hash(&self) -> String {
        let mut clone = self.clone();
        clone.content_hash = None;
        // Keep prev_hash intact to tightly bind the chain!
        let canon = serde_json::to_string(&clone).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(canon.as_bytes());
        format!("{:x}", hasher.finalize())
    }
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
    
    // Write the main artifact
    std::fs::write(&path, &body)?;

    // Compute and write the sidecar hash
    let mut hasher = Sha256::new();
    hasher.update(body.as_bytes());
    let hash = format!("{:x}", hasher.finalize());
    let filename = path.file_name().unwrap_or_default().to_string_lossy();
    let sidecar_path = dir.join(format!("{}_{}.json.sha256", safe, stamp));
    let sidecar_body = format!("{}  {}\n", hash, filename);
    std::fs::write(&sidecar_path, sidecar_body)?;

    Ok(path)
}

pub fn verify_artifact_hash(path: &PathBuf) -> bool {
    // Check if the sidecar exists
    let sidecar_path = PathBuf::from(format!("{}.sha256", path.display()));
    if !sidecar_path.exists() {
        return false;
    }

    // Read the expected hash from the sidecar
    let sidecar_content = match std::fs::read_to_string(&sidecar_path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    let expected_hash = match sidecar_content.split_whitespace().next() {
        Some(h) => h,
        None => return false,
    };

    // Compute the actual hash of the artifact
    let artifact_content = match std::fs::read(&path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    let mut hasher = Sha256::new();
    hasher.update(&artifact_content);
    let actual_hash = format!("{:x}", hasher.finalize());

    actual_hash == expected_hash
}

pub fn compute_file_hash(path: &std::path::Path) -> std::io::Result<String> {
    let content = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn append_audit_event(mut event: AuditEvent) -> std::io::Result<PathBuf> {
    if event.timestamp.is_empty() {
        event.timestamp = Utc::now().to_rfc3339();
    }
    let dir = default_pilot_home();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("audit.jsonl");

    // Extract prev_hash
    let prev_hash = get_last_audit_hash(&path).unwrap_or_else(|| "genesis".to_string());
    event.prev_hash = Some(prev_hash);
    event.content_hash = Some(event.compute_content_hash());

    let line = serde_json::to_string(&event).map_err(|e| std::io::Error::other(e.to_string()))?;
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    writeln!(file, "{}", line)?;
    Ok(path)
}

fn get_last_audit_hash(path: &PathBuf) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let last_line = content.lines().filter(|s| !s.trim().is_empty()).last()?;
    let event: AuditEvent = serde_json::from_str(last_line).ok()?;
    event.content_hash
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainVerification {
    pub is_valid: bool,
    pub audited_events: usize,
    pub errors: Vec<String>,
}

pub fn verify_audit_chain() -> ChainVerification {
    let path = default_pilot_home().join("audit.jsonl");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => {
            return ChainVerification {
                is_valid: true,
                audited_events: 0,
                errors: vec![],
            };
        }
    };

    let lines: Vec<&str> = content.lines().filter(|s| !s.trim().is_empty()).collect();
    let mut prev_expected_hash = "genesis".to_string();
    let mut errors = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let event: AuditEvent = match serde_json::from_str(line) {
            Ok(e) => e,
            Err(_) => {
                errors.push(format!("Line {}: Failed to parse JSON", i + 1));
                continue;
            }
        };

        let current_prev = event.prev_hash.clone().unwrap_or_default();
        if prev_expected_hash == "genesis" && current_prev.is_empty() {
            // Un-hashed legacy events are tolerated before hash chain starts.
            continue;
        }

        if current_prev != prev_expected_hash {
            errors.push(format!(
                "Line {}: Chain broken. Expected prev_hash '{}', but found '{}'",
                i + 1,
                prev_expected_hash,
                current_prev
            ));
        }

        let computed = event.compute_content_hash();
        let stated = event.content_hash.clone().unwrap_or_default();
        if computed != stated {
            errors.push(format!(
                "Line {}: Content hash mismatch. Computed '{}', but event says '{}'",
                i + 1,
                computed,
                stated
            ));
        }

        prev_expected_hash = computed;
    }

    ChainVerification {
        is_valid: errors.is_empty(),
        audited_events: lines.len(),
        errors,
    }
}

fn default_reports_dir() -> PathBuf {
    default_pilot_home().join("reports")
}

fn default_pilot_home() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".pilot")
}

pub fn query_audit_events(
    scope_id: Option<&str>,
    domain_filter: Option<&str>,
    action_filter: Option<&str>,
    limit: usize,
    offset: usize,
) -> Vec<AuditEvent> {
    let path = default_pilot_home().join("audit.jsonl");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let mut events: Vec<AuditEvent> = content
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .filter(|e: &AuditEvent| {
            let scope_ok = scope_id.map_or(true, |sid| e.scope_id.as_deref() == Some(sid));
            let domain_ok = domain_filter.map_or(true, |d| e.domain == d);
            let action_ok = action_filter.map_or(true, |a| e.action == a);
            scope_ok && domain_ok && action_ok
        })
        .collect();

    events.reverse(); // Most recent first
    events.into_iter().skip(offset).take(limit).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_audit_chain_tamper_simulation() {
        // Setup isolated HOME
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("HOME", temp_dir.path().to_str().unwrap());

        // Create 3 valid events
        for i in 0..3 {
            let event = AuditEvent {
                id: format!("evt-{}", i),
                timestamp: "".to_string(), // will be populated
                scope_id: None,
                domain: "test".to_string(),
                action: "mock".to_string(),
                dry_run: false,
                success: true,
                summary: format!("Event {}", i),
                repo_count: 0,
                failures: 0,
                artifact_path: None,
                repos: vec![],
                details: serde_json::json!({}),
                content_hash: None,
                prev_hash: None,
            };
            append_audit_event(event).unwrap();
        }

        // 1. Verify a healthy chain
        let healthy_res = verify_audit_chain();
        assert!(healthy_res.is_valid, "Chain should be valid initially");
        assert_eq!(healthy_res.audited_events, 3);

        // 2. Tamper Simulation: Corrupt the second event
        let audit_path = default_pilot_home().join("audit.jsonl");
        let mut lines: Vec<String> = fs::read_to_string(&audit_path)
            .unwrap()
            .lines()
            .map(|s| s.to_string())
            .collect();
        
        let mut tampered_event: AuditEvent = serde_json::from_str(&lines[1]).unwrap();
        tampered_event.success = false; // The malicious mutation
        lines[1] = serde_json::to_string(&tampered_event).unwrap();
        
        // Write the corrupted chain back
        fs::write(&audit_path, lines.join("\n") + "\n").unwrap();

        // 3. Verify the tampered chain fails
        let tampered_res = verify_audit_chain();
        assert!(!tampered_res.is_valid, "Chain should be invalid after tampering!");
        assert!(
            tampered_res.errors.iter().any(|e| e.contains("Content hash mismatch")),
            "Expected hash mismatch error for the tampered event"
        );
        assert!(
            tampered_res.errors.iter().any(|e| e.contains("Chain broken")),
            "Expected chain broken error for the subsequent event"
        );
    }
}
