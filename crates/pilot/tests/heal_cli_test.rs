use assert_cmd::Command;

#[test]
#[allow(deprecated)]
fn test_heal_help_surface() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("pilot")?;
    cmd.arg("heal")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("--log-file"))
        .stdout(predicates::str::contains("--max-attempts"));

    Ok(())
}
