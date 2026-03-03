use crate::preflight::model::*;
use miette::Result;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn run_preflight_graph(
    repo_path: &Path,
    steps: Vec<PreflightStepType>,
    branch: Option<&str>,
    remote: Option<&str>,
) -> Result<PreflightReport> {
    let mut report = PreflightReport::new();

    for step in steps {
        if !report.is_pass() {
            // Fail fast
            report.add(
                step.clone(),
                PreflightResult {
                    status: PreflightStatus::Skip,
                    failure_code: None,
                    hint: None,
                    messages: vec!["Skipped due to previous failure".to_string()],
                },
            );
            continue;
        }

        let result = match step {
            PreflightStepType::Policy => check_toolchain_policy(repo_path).await,
            PreflightStepType::Hook => check_git_hooks(repo_path).await,
            PreflightStepType::Drift => check_drift(repo_path).await,
            PreflightStepType::Gate => check_gate(repo_path).await,
            PreflightStepType::Push => {
                let b = branch.unwrap_or("main");
                let r = remote.unwrap_or("origin");
                execute_push(repo_path, b, r).await
            }
        };

        match result {
            Ok(res) => report.add(step, res),
            Err(e) => report.add(
                step,
                PreflightResult {
                    status: PreflightStatus::Fail,
                    failure_code: Some("ERR_EXECUTION_FAULT".to_string()),
                    hint: Some("Check pilot daemon logs or ensure script exists".to_string()),
                    messages: vec![e.to_string()],
                },
            ),
        }
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let now_secs = now.as_secs();
    let now_nanos = now.subsec_nanos();
    let root = std::env::var("HOME")
        .map(|h| PathBuf::from(h).join(".pilot").join("reports"))
        .unwrap_or_else(|_| PathBuf::from("/tmp/pilot-reports"));
    let _ = std::fs::create_dir_all(&root);
    // Use both seconds and nanoseconds to avoid timestamp collision within same second
    let report_name = format!("preflight_{}_{}.json", now_secs, now_nanos);
    let report_path = root.join(&report_name);

    report.evidence_path = Some(report_path.display().to_string());
    if let Ok(json) = serde_json::to_string_pretty(&report) {
        if let Err(e) = std::fs::write(&report_path, &json) {
            eprintln!(
                "Warning: Failed to write preflight evidence to {}: {}",
                report_path.display(), e
            );
        }
    }

    Ok(report)
}

fn command_from_script(repo_path: &Path, script_path: &str) -> Command {
    // Determine the root of the arqon pilot project to locate scripts accurately
    // The scripts are normally executed relative to the ArqonPilot directory
    let mut cmd = Command::new("bash");
    // Pilot runs from ArqonPilot root usually, so `./scripts/...` might be valid if run there,
    // but the target repo for the push/drift is `repo_path`.
    // Actually, `verify_toolchain_policy.sh` and others are global repository scripts residing in ArqonPilot/scripts.
    // We should compute the absolute path to to the script.
    // For now, assume the pilot binary is run from ArqonPilot root or `scripts/` is in CWD.
    let script = PathBuf::from(script_path);
    let abs_script = std::fs::canonicalize(&script).unwrap_or(script);
    cmd.arg(abs_script).current_dir(repo_path);
    cmd
}

async fn run_script_captured(
    repo_path: &Path,
    script_path: &str,
    args: &[&str],
    err_code: &str,
    hint: &str,
) -> Result<PreflightResult> {
    let mut cmd = command_from_script(repo_path, script_path);
    if !args.is_empty() {
        cmd.args(args);
    }

    let output = cmd.output().await.map_err(|e| miette::miette!(e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    let mut messages = Vec::new();
    if !stdout.is_empty() {
        messages.push(stdout);
    }
    if !stderr.is_empty() {
        // Many scripts emit info to stderr, distinguishing ok/fail purely by exit code
        messages.push(stderr);
    }

    if output.status.success() {
        Ok(PreflightResult {
            status: PreflightStatus::Pass,
            failure_code: None,
            hint: None,
            messages,
        })
    } else {
        Ok(PreflightResult {
            status: PreflightStatus::Fail,
            failure_code: Some(err_code.to_string()),
            hint: Some(hint.to_string()),
            messages,
        })
    }
}

pub async fn check_toolchain_policy(repo_path: &Path) -> Result<PreflightResult> {
    // Note: the script logic relies on ArqonPilot structure, but when running it handles `--json` optionally.
    run_script_captured(
        repo_path,
        "./scripts/verify_toolchain_policy.sh",
        &[],
        "ERR_TOOLCHAIN_DRIFT",
        "Run `./scripts/repair_lock_182.sh --no-gate` to correct toolchain drift",
    )
    .await
}

pub async fn check_git_hooks(repo_path: &Path) -> Result<PreflightResult> {
    run_script_captured(
        repo_path,
        "./scripts/verify_git_hook_policy.sh",
        &[],
        "ERR_HOOK_MISSING",
        "Run `make initialize` to restore required Git hooks",
    )
    .await
}

pub async fn check_drift(repo_path: &Path) -> Result<PreflightResult> {
    run_script_captured(
        repo_path,
        "./scripts/drift_report.sh",
        &[],
        "ERR_DRIFT_DETECTED",
        "Synchronize configuration profiles or accept the drift changes",
    )
    .await
}

pub async fn check_gate(repo_path: &Path) -> Result<PreflightResult> {
    run_script_captured(
        repo_path,
        "./scripts/prepush_gate.sh",
        &[],
        "ERR_PREPUSH_GATE",
        "Repository fails local pre-push tests (lint/fmt/clippy). Fix warnings.",
    )
    .await
}

pub async fn execute_push(repo_path: &Path, branch: &str, remote: &str) -> Result<PreflightResult> {
    run_script_captured(
        repo_path,
        "./scripts/push_main.sh",
        &[branch, remote],
        "ERR_PUSH_REJECTED",
        "The upstream remote rejected the push. Check for branch conflicts.",
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_preflight_graph_fail_fast_on_policy() {
        // If we inject a fake step that immediately fails, the subsequent should be Skipped.
        let mut report = PreflightReport::new();
        report.add(
            PreflightStepType::Policy,
            PreflightResult {
                status: PreflightStatus::Fail,
                failure_code: Some("BAD".into()),
                hint: None,
                messages: vec![],
            },
        );

        // Next step added simulates the engine parsing the graph skipping
        if !report.is_pass() {
            report.add(
                PreflightStepType::Hook,
                PreflightResult {
                    status: PreflightStatus::Skip,
                    failure_code: None,
                    hint: None,
                    messages: vec!["Skipped due to previous failure".to_string()],
                },
            );
        }

        // Check manual continuation constraint.
        assert!(!report.is_pass());
        assert_eq!(report.steps.len(), 2);
        assert_eq!(report.steps[1].result.status, PreflightStatus::Skip);
    }

    #[tokio::test]
    async fn test_preflight_graph_pass() {
        // Evaluate the fundamental orchestrator loop with an empty request
        let path = PathBuf::from(".");
        let report = run_preflight_graph(&path, vec![], None, None)
            .await
            .unwrap();
        assert!(report.is_pass());
        assert_eq!(report.steps.len(), 0);
    }
}
