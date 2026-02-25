use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

#[test]
#[allow(deprecated)]
fn test_navigate_help_surface() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("pilot")?;
    cmd.arg("navigate")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("--dry-run"))
        .stdout(predicates::str::contains("--skip-checks"))
        .stdout(predicates::str::contains("--multi"));

    Ok(())
}

#[test]
#[allow(deprecated)]
fn test_navigate_multi_dry_run() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let home = temp.path().join("home");
    let repo = temp.path().join("repo-a");
    fs::create_dir_all(&home)?;
    fs::create_dir_all(&repo)?;

    let mut register = Command::cargo_bin("pilot")?;
    register
        .env("HOME", &home)
        .arg("multi")
        .arg("register")
        .arg("--path")
        .arg(repo.to_string_lossy().to_string())
        .arg("--group")
        .arg("core")
        .assert()
        .success();

    let mut navigate_multi = Command::cargo_bin("pilot")?;
    navigate_multi
        .env("HOME", &home)
        .arg("navigate")
        .arg("--multi")
        .arg("--dry-run")
        .arg("--group")
        .arg("core")
        .assert()
        .success()
        .stdout(predicates::str::contains("Coordinated release order"));

    Ok(())
}
