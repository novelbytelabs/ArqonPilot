use assert_cmd::Command;

#[test]
#[allow(deprecated)]
fn test_branch_help_surface() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("pilot")?;
    cmd.arg("branch")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("create"))
        .stdout(predicates::str::contains("sync"))
        .stdout(predicates::str::contains("status"))
        .stdout(predicates::str::contains("prune"));

    Ok(())
}
