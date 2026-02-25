use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

#[test]
#[allow(deprecated)]
fn test_multi_help_surface() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("pilot")?;
    cmd.arg("multi")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("register"))
        .stdout(predicates::str::contains("list"))
        .stdout(predicates::str::contains("status"))
        .stdout(predicates::str::contains("query"))
        .stdout(predicates::str::contains("deps"))
        .stdout(predicates::str::contains("order"))
        .stdout(predicates::str::contains("prs"));

    Ok(())
}

#[test]
#[allow(deprecated)]
fn test_multi_dependencies_order_and_prs_plan() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let home = temp.path().join("home");
    let repo_a = temp.path().join("repo-a");
    let repo_b = temp.path().join("repo-b");

    fs::create_dir_all(&home)?;
    fs::create_dir_all(&repo_a)?;
    fs::create_dir_all(&repo_b)?;

    let mut register_a = Command::cargo_bin("pilot")?;
    register_a
        .env("HOME", &home)
        .arg("multi")
        .arg("register")
        .arg("--path")
        .arg(repo_a.to_string_lossy().to_string())
        .arg("--group")
        .arg("core")
        .assert()
        .success();

    let mut register_b = Command::cargo_bin("pilot")?;
    register_b
        .env("HOME", &home)
        .arg("multi")
        .arg("register")
        .arg("--path")
        .arg(repo_b.to_string_lossy().to_string())
        .arg("--group")
        .arg("core")
        .assert()
        .success();

    let mut deps_set = Command::cargo_bin("pilot")?;
    deps_set
        .env("HOME", &home)
        .arg("multi")
        .arg("deps")
        .arg("set")
        .arg("--repo")
        .arg("repo-b")
        .arg("--depends-on")
        .arg("repo-a")
        .assert()
        .success()
        .stdout(predicates::str::contains("Updated dependencies"));

    let mut order = Command::cargo_bin("pilot")?;
    order
        .env("HOME", &home)
        .arg("multi")
        .arg("order")
        .arg("--group")
        .arg("core")
        .assert()
        .success()
        .stdout(predicates::str::contains("1. repo-a"))
        .stdout(predicates::str::contains("2. repo-b"));

    let plan_path = temp.path().join("linked_prs.json");
    let mut prs = Command::cargo_bin("pilot")?;
    prs.env("HOME", &home)
        .arg("multi")
        .arg("prs")
        .arg("create")
        .arg("--group")
        .arg("core")
        .arg("--output")
        .arg(plan_path.to_string_lossy().to_string())
        .assert()
        .success()
        .stdout(predicates::str::contains("Linked PR manifest:"));

    assert!(plan_path.exists());
    Ok(())
}

#[test]
#[allow(deprecated)]
fn test_multi_register_and_list() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let home = temp.path().join("home");
    let repo = temp.path().join("repo-a");

    fs::create_dir_all(&home)?;
    fs::create_dir_all(&repo)?;

    // Register repository
    let mut register = Command::cargo_bin("pilot")?;
    register
        .env("HOME", &home)
        .arg("multi")
        .arg("register")
        .arg("--path")
        .arg(repo.to_string_lossy().to_string())
        .arg("--group")
        .arg("core")
        .arg("--tag")
        .arg("rust")
        .assert()
        .success()
        .stdout(predicates::str::contains("Registered:"));

    // List by group filter
    let mut list = Command::cargo_bin("pilot")?;
    list.env("HOME", &home)
        .arg("multi")
        .arg("list")
        .arg("--group")
        .arg("core")
        .assert()
        .success()
        .stdout(predicates::str::contains("repo-a"))
        .stdout(predicates::str::contains("group=Some(\"core\")"));

    Ok(())
}
