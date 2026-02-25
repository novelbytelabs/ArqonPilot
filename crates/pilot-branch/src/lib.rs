use anyhow::{Context, Result};
use pilot_multi::RepoEntry;
use serde::{Deserialize, Serialize};
use std::process::Command;

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
