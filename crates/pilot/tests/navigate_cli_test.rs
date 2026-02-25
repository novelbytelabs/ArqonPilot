use assert_cmd::Command;

#[test]
#[allow(deprecated)]
fn test_navigate_help_surface() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("pilot")?;
    cmd.arg("navigate")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("--dry-run"))
        .stdout(predicates::str::contains("--skip-checks"));

    Ok(())
}
