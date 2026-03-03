use anyhow::{Context, Result};
use chrono::Utc;
use pilot_multi::RepoEntry;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchOutcome {
    pub repo: String,
    pub path: String,
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoBranchStatus {
    pub repo: String,
    pub path: String,
    pub current_branch: String,
    pub clean: bool,
}

pub fn create_branch(
    repos: &[RepoEntry],
    branch: &str,
    base_branch: &str,
    dry_run: bool,
) -> Vec<BranchOutcome> {
    repos
        .iter()
        .map(|repo| {
            if dry_run {
                return BranchOutcome {
                    repo: repo.name.clone(),
                    path: repo.path.display().to_string(),
                    success: true,
                    message: format!(
                        "[DRY RUN] Would create/reset branch '{}' from '{}'",
                        branch, base_branch
                    ),
                };
            }

            let fetch = Command::new("git")
                .args(["fetch", "origin"])
                .current_dir(&repo.path)
                .output();
            if fetch.as_ref().map(|o| !o.status.success()).unwrap_or(true) {
                return BranchOutcome {
                    repo: repo.name.clone(),
                    path: repo.path.display().to_string(),
                    success: false,
                    message: "git fetch origin failed".to_string(),
                };
            }

            let checkout = Command::new("git")
                .args(["checkout", "-B", branch, &format!("origin/{}", base_branch)])
                .current_dir(&repo.path)
                .output();

            match checkout {
                Ok(out) if out.status.success() => BranchOutcome {
                    repo: repo.name.clone(),
                    path: repo.path.display().to_string(),
                    success: true,
                    message: format!("Created branch '{}'", branch),
                },
                Ok(out) => BranchOutcome {
                    repo: repo.name.clone(),
                    path: repo.path.display().to_string(),
                    success: false,
                    message: String::from_utf8_lossy(&out.stderr).to_string(),
                },
                Err(e) => BranchOutcome {
                    repo: repo.name.clone(),
                    path: repo.path.display().to_string(),
                    success: false,
                    message: e.to_string(),
                },
            }
        })
        .collect()
}

pub fn sync_branch(
    repos: &[RepoEntry],
    branch: &str,
    base_branch: &str,
    dry_run: bool,
) -> Vec<BranchOutcome> {
    repos
        .iter()
        .map(|repo| {
            if dry_run {
                return BranchOutcome {
                    repo: repo.name.clone(),
                    path: repo.path.display().to_string(),
                    success: true,
                    message: format!(
                        "[DRY RUN] Would sync '{}' with origin/{}",
                        branch, base_branch
                    ),
                };
            }

            let merge_target = format!("origin/{}", base_branch);
            let commands = vec![
                vec!["fetch".to_string(), "origin".to_string()],
                vec!["checkout".to_string(), branch.to_string()],
                vec!["merge".to_string(), "--ff-only".to_string(), merge_target],
            ];

            for cmd in commands {
                let out = Command::new("git")
                    .args(&cmd)
                    .current_dir(&repo.path)
                    .output();
                match out {
                    Ok(o) if o.status.success() => {}
                    Ok(o) => {
                        return BranchOutcome {
                            repo: repo.name.clone(),
                            path: repo.path.display().to_string(),
                            success: false,
                            message: String::from_utf8_lossy(&o.stderr).to_string(),
                        }
                    }
                    Err(e) => {
                        return BranchOutcome {
                            repo: repo.name.clone(),
                            path: repo.path.display().to_string(),
                            success: false,
                            message: e.to_string(),
                        }
                    }
                }
            }

            BranchOutcome {
                repo: repo.name.clone(),
                path: repo.path.display().to_string(),
                success: true,
                message: format!("Synced '{}'", branch),
            }
        })
        .collect()
}

pub fn branch_status(repos: &[RepoEntry]) -> Vec<RepoBranchStatus> {
    repos
        .iter()
        .map(|repo| {
            let current_branch = Command::new("git")
                .args(["rev-parse", "--abbrev-ref", "HEAD"])
                .current_dir(&repo.path)
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .unwrap_or_else(|| "unknown".to_string());

            let clean = Command::new("git")
                .args(["status", "--porcelain"])
                .current_dir(&repo.path)
                .output()
                .ok()
                .map(|o| o.status.success() && o.stdout.is_empty())
                .unwrap_or(false);

            RepoBranchStatus {
                repo: repo.name.clone(),
                path: repo.path.display().to_string(),
                current_branch,
                clean,
            }
        })
        .collect()
}

pub fn prune_branches(
    repos: &[RepoEntry],
    base_branch: &str,
    dry_run: bool,
) -> Result<Vec<BranchOutcome>> {
    let mut outcomes = Vec::new();

    for repo in repos {
        let merged = Command::new("git")
            .args(["branch", "--merged", base_branch])
            .current_dir(&repo.path)
            .output()
            .with_context(|| {
                format!("Failed to list merged branches in {}", repo.path.display())
            })?;

        if !merged.status.success() {
            outcomes.push(BranchOutcome {
                repo: repo.name.clone(),
                path: repo.path.display().to_string(),
                success: false,
                message: String::from_utf8_lossy(&merged.stderr).to_string(),
            });
            continue;
        }

        let branches: Vec<String> = String::from_utf8_lossy(&merged.stdout)
            .lines()
            .map(|l| l.trim().trim_start_matches('*').trim().to_string())
            .filter(|b| !b.is_empty() && b != base_branch && b != "main" && b != "master")
            .collect();

        if branches.is_empty() {
            outcomes.push(BranchOutcome {
                repo: repo.name.clone(),
                path: repo.path.display().to_string(),
                success: true,
                message: "No merged branches to prune".to_string(),
            });
            continue;
        }

        if dry_run {
            outcomes.push(BranchOutcome {
                repo: repo.name.clone(),
                path: repo.path.display().to_string(),
                success: true,
                message: format!("[DRY RUN] Would prune: {}", branches.join(", ")),
            });
            continue;
        }

        let mut failed = None;
        for b in &branches {
            let out = Command::new("git")
                .args(["branch", "-d", b])
                .current_dir(&repo.path)
                .output();
            match out {
                Ok(o) if o.status.success() => {}
                Ok(o) => {
                    failed = Some(String::from_utf8_lossy(&o.stderr).to_string());
                    break;
                }
                Err(e) => {
                    failed = Some(e.to_string());
                    break;
                }
            }
        }

        outcomes.push(BranchOutcome {
            repo: repo.name.clone(),
            path: repo.path.display().to_string(),
            success: failed.is_none(),
            message: failed.unwrap_or_else(|| format!("Pruned {} branches", branches.len())),
        });
    }

    Ok(outcomes)
}

// ─────────────────────────────────────────────────────────────────────────────
// P4 Deliverable 1: Conflict Radar
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictRadarResult {
    pub repo: String,
    pub path: String,
    pub has_conflicts: bool,
    pub conflicting_files: Vec<String>,
    pub merge_base: String,
    pub ahead: usize,
    pub behind: usize,
    pub error: Option<String>,
}

/// Pre-flight conflict detection for sync/merge operations.
/// Uses `git merge-tree` (git 2.38+) to detect file-level conflicts without
/// modifying the working tree. Falls back to a simple ahead/behind check
/// if merge-tree is unavailable.
pub fn conflict_radar(
    repos: &[RepoEntry],
    branch: &str,
    base_branch: &str,
) -> Vec<ConflictRadarResult> {
    repos
        .iter()
        .map(|repo| {
            let repo_path = &repo.path;

            // Get merge base
            let merge_base_out = Command::new("git")
                .args(["merge-base", &format!("origin/{}", base_branch), branch])
                .current_dir(repo_path)
                .output();

            let merge_base = match merge_base_out {
                Ok(o) if o.status.success() => {
                    String::from_utf8_lossy(&o.stdout).trim().to_string()
                }
                _ => {
                    return ConflictRadarResult {
                        repo: repo.name.clone(),
                        path: repo_path.display().to_string(),
                        has_conflicts: false,
                        conflicting_files: vec![],
                        merge_base: String::new(),
                        ahead: 0,
                        behind: 0,
                        error: Some(
                            "Failed to compute merge-base; branch may not exist on remote"
                                .to_string(),
                        ),
                    };
                }
            };

            // Ahead/behind counts
            let (ahead, behind) = ahead_behind_count(repo_path, branch, base_branch);

            // Conflict detection via git merge-tree
            let merge_tree_out = Command::new("git")
                .args([
                    "merge-tree",
                    "--write-tree",
                    "--no-messages",
                    &format!("origin/{}", base_branch),
                    branch,
                ])
                .current_dir(repo_path)
                .output();

            match merge_tree_out {
                Ok(o) => {
                    let has_conflicts = !o.status.success();
                    let conflicting_files = if has_conflicts {
                        // Parse conflicting file paths from merge-tree output
                        parse_merge_tree_conflicts(&String::from_utf8_lossy(&o.stdout))
                    } else {
                        vec![]
                    };

                    ConflictRadarResult {
                        repo: repo.name.clone(),
                        path: repo_path.display().to_string(),
                        has_conflicts,
                        conflicting_files,
                        merge_base: merge_base.clone(),
                        ahead,
                        behind,
                        error: None,
                    }
                }
                Err(_) => {
                    // Fallback: git merge-tree not available (git < 2.38)
                    ConflictRadarResult {
                        repo: repo.name.clone(),
                        path: repo_path.display().to_string(),
                        has_conflicts: false,
                        conflicting_files: vec![],
                        merge_base,
                        ahead,
                        behind,
                        error: Some(
                            "git merge-tree --write-tree not available; upgrade to git 2.38+"
                                .to_string(),
                        ),
                    }
                }
            }
        })
        .collect()
}

fn ahead_behind_count(repo_path: &Path, branch: &str, base_branch: &str) -> (usize, usize) {
    let out = Command::new("git")
        .args([
            "rev-list",
            "--left-right",
            "--count",
            &format!("{}...origin/{}", branch, base_branch),
        ])
        .current_dir(repo_path)
        .output();

    match out {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout);
            let parts: Vec<&str> = text.trim().split('\t').collect();
            if parts.len() == 2 {
                let ahead = parts[0].parse().unwrap_or(0);
                let behind = parts[1].parse().unwrap_or(0);
                (ahead, behind)
            } else {
                (0, 0)
            }
        }
        _ => (0, 0),
    }
}

fn parse_merge_tree_conflicts(output: &str) -> Vec<String> {
    // git merge-tree --write-tree outputs conflicting file paths after the tree hash.
    // Lines starting with a mode entry indicate conflicts.
    let mut files = Vec::new();
    for line in output.lines().skip(1) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Conflict entries look like: "100644 <hash> 1\t<path>" (stage 1/2/3)
        if let Some(tab_pos) = trimmed.find('\t') {
            let path = &trimmed[tab_pos + 1..];
            if !files.contains(&path.to_string()) {
                files.push(path.to_string());
            }
        }
    }
    files
}

// ─────────────────────────────────────────────────────────────────────────────
// P4 Deliverable 2: Undoable Operation Journal
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchUndoEntry {
    pub id: String,
    pub timestamp: String,
    pub repo: String,
    pub path: String,
    pub action: String,
    pub branch_name: String,
    pub prior_ref: String,
    pub new_ref: String,
    pub scope_id: Option<String>,
    pub undone: bool,
}

fn undo_journal_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".pilot").join("branch_undo.jsonl")
}

/// Capture the current ref position for a branch before mutation.
/// Returns the entry to be persisted after the mutation succeeds.
pub fn record_undo_entry(
    repo_path: &Path,
    repo_name: &str,
    branch: &str,
    action: &str,
    scope_id: Option<String>,
) -> Result<BranchUndoEntry> {
    let prior_ref = Command::new("git")
        .args(["rev-parse", branch])
        .current_dir(repo_path)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "0000000000000000000000000000000000000000".to_string());

    Ok(BranchUndoEntry {
        id: Uuid::new_v4().to_string(),
        timestamp: Utc::now().to_rfc3339(),
        repo: repo_name.to_string(),
        path: repo_path.display().to_string(),
        action: action.to_string(),
        branch_name: branch.to_string(),
        prior_ref,
        new_ref: String::new(), // Filled after mutation
        scope_id,
        undone: false,
    })
}

/// Persist the undo entry to the journal file after capturing the new ref.
pub fn persist_undo_entry(entry: &mut BranchUndoEntry) -> Result<()> {
    // Capture new ref
    let new_ref = Command::new("git")
        .args(["rev-parse", &entry.branch_name])
        .current_dir(Path::new(&entry.path))
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    entry.new_ref = new_ref;

    let path = undo_journal_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let line = serde_json::to_string(&entry)
        .map_err(|e| anyhow::anyhow!("Failed to serialize undo entry: {}", e))?;
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    writeln!(file, "{}", line)?;
    Ok(())
}

/// Execute an undo: restore the branch ref to its prior position.
pub fn execute_undo(entry: &BranchUndoEntry, dry_run: bool) -> BranchOutcome {
    if entry.prior_ref.starts_with("0000000") {
        return BranchOutcome {
            repo: entry.repo.clone(),
            path: entry.path.clone(),
            success: false,
            message: "Cannot undo: branch did not exist before this operation".to_string(),
        };
    }

    if dry_run {
        return BranchOutcome {
            repo: entry.repo.clone(),
            path: entry.path.clone(),
            success: true,
            message: format!(
                "[DRY RUN] Would restore '{}' from {} to {}",
                entry.branch_name,
                &entry.new_ref[..8.min(entry.new_ref.len())],
                &entry.prior_ref[..8.min(entry.prior_ref.len())]
            ),
        };
    }

    let out = Command::new("git")
        .args([
            "update-ref",
            &format!("refs/heads/{}", entry.branch_name),
            &entry.prior_ref,
        ])
        .current_dir(Path::new(&entry.path))
        .output();

    match out {
        Ok(o) if o.status.success() => BranchOutcome {
            repo: entry.repo.clone(),
            path: entry.path.clone(),
            success: true,
            message: format!(
                "Restored '{}' to {}",
                entry.branch_name,
                &entry.prior_ref[..8.min(entry.prior_ref.len())]
            ),
        },
        Ok(o) => BranchOutcome {
            repo: entry.repo.clone(),
            path: entry.path.clone(),
            success: false,
            message: String::from_utf8_lossy(&o.stderr).to_string(),
        },
        Err(e) => BranchOutcome {
            repo: entry.repo.clone(),
            path: entry.path.clone(),
            success: false,
            message: e.to_string(),
        },
    }
}

/// Read the undo journal, optionally filtered by scope.
pub fn list_undo_journal(scope_id: Option<&str>, limit: usize) -> Vec<BranchUndoEntry> {
    let path = undo_journal_path();
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let mut entries: Vec<BranchUndoEntry> = content
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .filter(|e: &BranchUndoEntry| {
            scope_id.map_or(true, |sid| e.scope_id.as_deref() == Some(sid))
        })
        .collect();

    entries.reverse(); // Most recent first
    entries.truncate(limit);
    entries
}

/// Mark an undo entry as executed in the journal.
pub fn mark_undone(entry_id: &str) -> Result<()> {
    let path = undo_journal_path();
    let content = std::fs::read_to_string(&path).with_context(|| "Failed to read undo journal")?;

    let updated: Vec<String> = content
        .lines()
        .map(|line| {
            if let Ok(mut e) = serde_json::from_str::<BranchUndoEntry>(line) {
                if e.id == entry_id {
                    e.undone = true;
                    return serde_json::to_string(&e).unwrap_or_else(|_| line.to_string());
                }
            }
            line.to_string()
        })
        .collect();

    std::fs::write(&path, updated.join("\n") + "\n")?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// ─────────────────────────────────────────────────────────────────────────────
// P4 Deliverable 4: Confirmation Types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ConfirmationType {
    /// No extra confirmation required
    None,
    /// Simple checkbox / "Execute" button
    Standard,
    /// Must type an exact phrase to confirm
    TypedPhrase,
    /// Typed phrase + countdown timer before enable
    DoubleConfirm,
}

impl Default for ConfirmationType {
    fn default() -> Self {
        ConfirmationType::Standard
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_merge_tree_conflicts_empty() {
        let output = "abc123def456\n";
        let files = parse_merge_tree_conflicts(output);
        assert!(files.is_empty());
    }

    #[test]
    fn test_parse_merge_tree_conflicts_with_entries() {
        let output = "abc123def456\n100644 deadbeef 1\tsrc/main.rs\n100644 cafebabe 2\tsrc/main.rs\n100644 f00dface 3\tCargo.toml\n";
        let files = parse_merge_tree_conflicts(output);
        assert_eq!(files.len(), 2);
        assert!(files.contains(&"src/main.rs".to_string()));
        assert!(files.contains(&"Cargo.toml".to_string()));
    }

    #[test]
    fn test_undo_entry_null_ref_rejected() {
        let entry = BranchUndoEntry {
            id: "test-id".to_string(),
            timestamp: Utc::now().to_rfc3339(),
            repo: "test-repo".to_string(),
            path: "/tmp/test".to_string(),
            action: "create".to_string(),
            branch_name: "feat/test".to_string(),
            prior_ref: "0000000000000000000000000000000000000000".to_string(),
            new_ref: "abcdef1234567890".to_string(),
            scope_id: None,
            undone: false,
        };
        let outcome = execute_undo(&entry, false);
        assert!(!outcome.success);
        assert!(outcome.message.contains("Cannot undo"));
    }

    #[test]
    fn test_undo_entry_dry_run() {
        let entry = BranchUndoEntry {
            id: "test-id".to_string(),
            timestamp: Utc::now().to_rfc3339(),
            repo: "test-repo".to_string(),
            path: "/tmp/test".to_string(),
            action: "create".to_string(),
            branch_name: "feat/test".to_string(),
            prior_ref: "abcdef1234567890abcdef1234567890abcdef12".to_string(),
            new_ref: "1234567890abcdef1234567890abcdef12345678".to_string(),
            scope_id: None,
            undone: false,
        };
        let outcome = execute_undo(&entry, true);
        assert!(outcome.success);
        assert!(outcome.message.contains("DRY RUN"));
        assert!(outcome.message.contains("Would restore"));
    }

    #[test]
    fn test_confirmation_type_default() {
        assert_eq!(ConfirmationType::default(), ConfirmationType::Standard);
    }

    #[test]
    fn test_timeline_event_serialization() {
        let event = BranchTimelineEvent {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now().to_rfc3339(),
            scope_id: Some(42.to_string()),
            action: "create".to_string(),
            branch: "feat/test".to_string(),
            base_branch: "main".to_string(),
            repos: vec!["repo-a".to_string(), "repo-b".to_string()],
            dry_run: true,
            success: true,
            repo_count: 2,
            failures: 0,
            conflict_count: 0,
            undo_entry_ids: vec![],
            details: serde_json::json!({"note": "test"}),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: BranchTimelineEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.action, "create");
        assert_eq!(parsed.repos.len(), 2);
    }

    #[test]
    fn test_conflict_radar_result_serialization() {
        let result = ConflictRadarResult {
            repo: "ArqonCore".to_string(),
            path: "/home/user/ArqonCore".to_string(),
            has_conflicts: true,
            conflicting_files: vec!["src/lib.rs".to_string(), "Cargo.toml".to_string()],
            merge_base: "abc123".to_string(),
            ahead: 3,
            behind: 1,
            error: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("ArqonCore"));
        let parsed: ConflictRadarResult = serde_json::from_str(&json).unwrap();
        assert!(parsed.has_conflicts);
        assert_eq!(parsed.conflicting_files.len(), 2);
    }
}
