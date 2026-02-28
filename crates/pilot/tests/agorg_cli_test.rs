use assert_cmd::Command;

#[test]
#[allow(deprecated)]
fn test_agorg_help_surface() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("pilot")?;
    cmd.arg("agorg")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("create"))
        .stdout(predicates::str::contains("create-project"))
        .stdout(predicates::str::contains("discover"))
        .stdout(predicates::str::contains("tree"))
        .stdout(predicates::str::contains("link"));

    Ok(())
}

#[test]
#[allow(deprecated)]
fn test_agorg_discover_help_contains_prune_flag() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("pilot")?;
    cmd.arg("agorg")
        .arg("discover")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("--import-to"))
        .stdout(predicates::str::contains("--prune-missing"));

    Ok(())
}
