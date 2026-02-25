use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

#[test]
#[allow(deprecated)]
fn test_ship_e2e_flow() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Setup temp git repo
    let temp = TempDir::new()?;
    let root = temp.path();

    // Initialize git repo
    std::process::Command::new("git")
        .arg("init")
        .current_dir(root)
        .output()?;

    // Config git user for commits
    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(root)
        .output()?;
    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(root)
        .output()?;

    // Create Cargo.toml
    let cargo_toml = r#"
[workspace.package]
version = "0.1.0"
edition = "2021"
"#;
    fs::write(root.join("Cargo.toml"), cargo_toml)?;

    // Initial commit
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(root)
        .output()?;
    std::process::Command::new("git")
        .args(["commit", "-m", "chore: initial commit"])
        .current_dir(root)
        .output()?;

    // 2. Make a fix commit
    fs::write(root.join("fix.txt"), "bugfix")?;
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(root)
        .output()?;
    std::process::Command::new("git")
        .args(["commit", "-m", "fix: critical bug"])
        .current_dir(root)
        .output()?;

    // 3. Run ship binary
    // Note: We use assert_cmd to run the binary built by cargo
    let mut cmd = Command::cargo_bin("pilot")?;
    cmd.current_dir(root)
        .arg("navigate")
        .arg("--skip-checks")
        .arg("--dry-run") // Use dry-run first to check output
        .assert()
        .success()
        .stdout(predicates::str::contains("Next version: v0.1.1"));

    // Now run for real (but without GitHub API usage, so we need to mock or just check local effects)
    // The current main.rs attempts to call GitHub API.
    // For this E2E, we might fail at the GitHub step if we don't have tokens or mocking.
    // However, the *file writing* happens before GitHub.
    // Let's assert that the file modification happens even if the process fails later at the network step?
    // Or simpler: The `ship` command in main.rs:178 writes to Cargo.toml.
    // The GitHub part is later.

    // We can't easily mock the GitHubClient in a binary E2E without dependency injection or env vars.
    // But `ship --dry-run` is safe.
    // To test the WRITE, we need to bypass GitHub.
    // Let's rely on Unit Tests for the write logic (already done) and use E2E mainly for `dry-run` output verification
    // OR we can modify `ship` to have a `--no-push` or `--local-only` flag in the future.
    // For now, let's stick to verifying `dry-run` output which confirms the calculation logic + git state reading works E2E.

    Ok(())
}

#[test]
#[allow(deprecated)]
fn test_ship_fails_on_dirty_state() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let root = temp.path();

    // 1. Init repo
    std::process::Command::new("git")
        .arg("init")
        .current_dir(root)
        .output()?;
    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(root)
        .output()?;
    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(root)
        .output()?;

    // 2. Commit a file
    fs::write(
        root.join("Cargo.toml"),
        r#"[package]
name = "test"
version = "0.1.0"
"#,
    )?;
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(root)
        .output()?;
    std::process::Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(root)
        .output()?;

    // 3. Make it dirty
    fs::write(root.join("dirty.txt"), "changed")?;

    // 4. Run ship, expect failure
    let mut cmd = Command::cargo_bin("pilot")?;
    cmd.current_dir(root)
        .arg("navigate")
        .arg("--dry-run")
        .assert()
        .failure()
        .stdout(predicates::str::contains("Constitution checks failed"));

    Ok(())
}
