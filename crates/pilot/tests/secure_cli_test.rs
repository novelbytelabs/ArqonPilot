use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

#[test]
#[allow(deprecated)]
fn test_secure_help_surface() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("pilot")?;
    cmd.arg("secure")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("scan"))
        .stdout(predicates::str::contains("fix"));
    Ok(())
}

#[test]
#[allow(deprecated)]
fn test_secure_scan_and_fix_dry_run_single_repo() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"x\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )?;
    fs::write(
        temp.path().join("lib.rs"),
        "const AWS: &str = \"AKIAABCDEFGHIJKLMNOP\";\n",
    )?;

    let mut scan = Command::cargo_bin("pilot")?;
    scan.current_dir(temp.path())
        .arg("secure")
        .arg("scan")
        .assert()
        .success()
        .stdout(predicates::str::contains("Repo:"))
        .stdout(predicates::str::contains("secret.aws_access_key"));

    let mut fix = Command::cargo_bin("pilot")?;
    fix.current_dir(temp.path())
        .arg("secure")
        .arg("fix")
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "[DRY RUN] Would run: cargo update",
        ));

    Ok(())
}
