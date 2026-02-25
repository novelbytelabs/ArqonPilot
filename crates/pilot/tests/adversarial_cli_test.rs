use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

#[test]
#[allow(deprecated)]
fn test_adversarial_dependency_cycle_fails_cleanly() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let home = temp.path().join("home");
    let repo_a = temp.path().join("repo-a");
    let repo_b = temp.path().join("repo-b");

    fs::create_dir_all(&home)?;
    fs::create_dir_all(&repo_a)?;
    fs::create_dir_all(&repo_b)?;

    for repo in [&repo_a, &repo_b] {
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
    }

    let mut dep_a = Command::cargo_bin("pilot")?;
    dep_a
        .env("HOME", &home)
        .arg("multi")
        .arg("deps")
        .arg("set")
        .arg("--repo")
        .arg("repo-a")
        .arg("--depends-on")
        .arg("repo-b")
        .assert()
        .success();

    let mut dep_b = Command::cargo_bin("pilot")?;
    dep_b
        .env("HOME", &home)
        .arg("multi")
        .arg("deps")
        .arg("set")
        .arg("--repo")
        .arg("repo-b")
        .arg("--depends-on")
        .arg("repo-a")
        .assert()
        .success();

    let mut order = Command::cargo_bin("pilot")?;
    order
        .env("HOME", &home)
        .arg("--report-json")
        .arg("multi")
        .arg("order")
        .arg("--group")
        .arg("core")
        .assert()
        .failure()
        .stdout(predicates::str::contains("\"command\":\"multi.order\""))
        .stdout(predicates::str::contains("\"success\":false"))
        .stderr(predicates::str::contains("Dependency graph has a cycle"));

    Ok(())
}
