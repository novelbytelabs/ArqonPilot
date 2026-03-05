use assert_cmd::Command;
use predicates::str::contains;
use std::fs;

fn skip_if_db_env_denied(
    home: &std::path::Path,
    pilot_home: &std::path::Path,
    port: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let out = Command::cargo_bin("pilot")?
        .env("HOME", home)
        .env("PILOT_HOME", pilot_home)
        .env("PILOT_DB_PORT", port)
        .env("PILOT_DB_MODE", "unix_socket")
        .arg("db")
        .arg("start")
        .output()?;
    if out.status.success() {
        let _ = Command::cargo_bin("pilot")?
            .env("HOME", home)
            .env("PILOT_HOME", pilot_home)
            .env("PILOT_DB_PORT", port)
            .env("PILOT_DB_MODE", "unix_socket")
            .arg("db")
            .arg("stop")
            .output();
        return Ok(false);
    }
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if stderr.contains("Operation not permitted")
        || (stderr.contains("Permission denied") && stderr.contains("shared memory"))
        || stderr.contains("could not open shared memory segment")
        || stderr.contains("could not create any Unix-domain sockets")
        || stderr.contains("could not bind Unix address")
    {
        eprintln!("Skipping test: managed Postgres denied by runtime environment.");
        return Ok(true);
    }
    Ok(false)
}

fn git_init(path: &std::path::Path) {
    std::process::Command::new("git")
        .arg("init")
        .current_dir(path)
        .output()
        .expect("Failed to git init");

    std::process::Command::new("git")
        .arg("config")
        .arg("user.email")
        .arg("test@example.com")
        .current_dir(path)
        .output()
        .expect("Failed to set git email");
    std::process::Command::new("git")
        .arg("config")
        .arg("user.name")
        .arg("test")
        .current_dir(path)
        .output()
        .expect("Failed to set git name");

    fs::write(path.join("README.md"), "test").expect("write readme");
    std::process::Command::new("git")
        .arg("add")
        .arg(".")
        .current_dir(path)
        .output()
        .expect("git add");
    std::process::Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg("initial")
        .current_dir(path)
        .output()
        .expect("git commit");
}

#[test]
#[allow(deprecated)]
fn test_operator_routine_policy_set_preview_scan() -> Result<(), Box<dyn std::error::Error>> {
    let temp_root = std::path::Path::new(".pilot_test_tmp");
    fs::create_dir_all(temp_root)?;
    let temp = tempfile::Builder::new()
        .prefix("policy_opr_")
        .tempdir_in(temp_root)?;
    let suffix = temp
        .path()
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("x");
    let org_name = format!("OperatorRoutineOrg-{}", suffix);
    let home = temp.path().join("home");
    let pilot_home = std::path::PathBuf::from(format!("/tmp/pilotdb_opr_{}", suffix));
    let repo_path = temp.path().join("routine-repo");
    fs::create_dir_all(&home)?;
    fs::create_dir_all(&repo_path)?;
    if skip_if_db_env_denied(&home, &pilot_home, "9346")? {
        return Ok(());
    }

    git_init(&repo_path);

    let create_out = Command::cargo_bin("pilot")?
        .env("HOME", &home)
        .env("PILOT_HOME", &pilot_home)
        .env("PILOT_DB_PORT", "9346")
        .env("PILOT_DB_MODE", "unix_socket")
        .arg("agorg")
        .arg("create")
        .arg("--name")
        .arg(&org_name)
        .arg("--root")
        .arg(temp.path().to_string_lossy().to_string())
        .output()?;
    if !create_out.status.success() {
        let stderr = String::from_utf8_lossy(&create_out.stderr);
        if stderr.contains("Operation not permitted")
            || stderr.contains("Permission denied")
            || stderr.contains("shared memory")
            || stderr.contains("could not create any Unix-domain sockets")
            || stderr.contains("could not bind Unix address")
        {
            eprintln!("Skipping test: managed Postgres denied by runtime environment.");
            return Ok(());
        }
        return Err(format!(
            "agorg create failed unexpectedly.\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&create_out.stdout),
            stderr
        )
        .into());
    }

    Command::cargo_bin("pilot")?
        .env("HOME", &home)
        .env("PILOT_HOME", &pilot_home)
        .env("PILOT_DB_PORT", "9346")
        .env("PILOT_DB_MODE", "unix_socket")
        .arg("agorg")
        .arg("use")
        .arg(&org_name)
        .assert()
        .success();

    Command::cargo_bin("pilot")?
        .env("HOME", &home)
        .env("PILOT_HOME", &pilot_home)
        .env("PILOT_DB_PORT", "9346")
        .env("PILOT_DB_MODE", "unix_socket")
        .arg("multi")
        .arg("register")
        .arg("--path")
        .arg(repo_path.to_string_lossy().to_string())
        .arg("--name")
        .arg("routine-repo")
        .assert()
        .success();

    let policy_file = temp.path().join("operator_routine_policy.json");
    fs::write(
        &policy_file,
        r#"{
        "kind": "operator_routine",
        "version": 1,
        "require_active_scope": { "level": "block", "enabled": true },
        "require_registered_repo": { "level": "block", "enabled": true },
        "require_clean_worktree_for_push": { "level": "warn", "enabled": true },
        "allowed_push_branches": { "level": "warn", "items": ["master"] },
        "required_prepush_steps": { "level": "warn", "items": ["gate"] }
    }"#,
    )?;

    Command::cargo_bin("pilot")?
        .env("HOME", &home)
        .env("PILOT_HOME", &pilot_home)
        .env("PILOT_DB_PORT", "9346")
        .env("PILOT_DB_MODE", "unix_socket")
        .arg("policy")
        .arg("set-draft")
        .arg("--kind")
        .arg("operator_routine")
        .arg("--file")
        .arg(&policy_file)
        .assert()
        .success()
        .stdout(contains("Saved draft policy operator_routine version 1"));

    Command::cargo_bin("pilot")?
        .env("HOME", &home)
        .env("PILOT_HOME", &pilot_home)
        .env("PILOT_DB_PORT", "9346")
        .env("PILOT_DB_MODE", "unix_socket")
        .arg("policy")
        .arg("preview")
        .arg("--kind")
        .arg("operator_routine")
        .arg("--version")
        .arg("1")
        .assert()
        .success()
        .stdout(contains("\"kind\": \"operator_routine\""))
        .stdout(contains("Artifact: "));

    Command::cargo_bin("pilot")?
        .env("HOME", &home)
        .env("PILOT_HOME", &pilot_home)
        .env("PILOT_DB_PORT", "9346")
        .env("PILOT_DB_MODE", "unix_socket")
        .arg("policy")
        .arg("scan")
        .arg("--kind")
        .arg("operator_routine")
        .assert()
        .success()
        .stdout(contains("\"kind\": \"operator_routine\""));

    Ok(())
}
