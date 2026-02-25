use assert_cmd::Command;
use tempfile::TempDir;

#[test]
#[allow(deprecated)]
fn test_create_help_surface() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("pilot")?;
    cmd.arg("create")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("feature"))
        .stdout(predicates::str::contains("tests"));
    Ok(())
}

#[test]
#[allow(deprecated)]
fn test_create_feature_and_tests_dry_run() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;

    let mut feature = Command::cargo_bin("pilot")?;
    feature
        .arg("create")
        .arg("feature")
        .arg("billing")
        .arg("--output-dir")
        .arg(temp.path().to_string_lossy().to_string())
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "[DRY RUN] Would create scaffold file",
        ));

    let mut tests = Command::cargo_bin("pilot")?;
    tests
        .arg("create")
        .arg("tests")
        .arg("billing")
        .arg("--output-dir")
        .arg(temp.path().to_string_lossy().to_string())
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "[DRY RUN] Would create scaffold file",
        ));

    Ok(())
}
