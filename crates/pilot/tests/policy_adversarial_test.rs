use assert_cmd::Command;
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
    {
        eprintln!("Skipping test: managed Postgres socket/tcp bind denied by runtime environment.");
        return Ok(true);
    }
    Ok(false)
}

fn is_runtime_db_denied(stderr: &str) -> bool {
    stderr.contains("Operation not permitted")
        || (stderr.contains("Permission denied") && stderr.contains("shared memory"))
        || stderr.contains("could not open shared memory segment")
        || stderr.contains("could not bind Unix address")
        || stderr.contains("could not create any Unix-domain sockets")
}

#[test]
#[allow(deprecated)]
fn test_policy_invalid_kind() -> Result<(), Box<dyn std::error::Error>> {
    let temp_root = std::path::Path::new(".pilot_test_tmp");
    fs::create_dir_all(temp_root)?;
    let temp = tempfile::Builder::new()
        .prefix("policy_adv_kind_")
        .tempdir_in(temp_root)?;
    let suffix = temp
        .path()
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("x");
    let org_name = format!("AdversarialOrg-{}", suffix);
    let home = temp.path().join("home");
    let pilot_home = std::path::PathBuf::from(format!("/tmp/pilotdb_adv_kind_{}", suffix));
    fs::create_dir_all(&home)?;
    if skip_if_db_env_denied(&home, &pilot_home, "9343")? {
        return Ok(());
    }

    // Setup AGOrg
    let create_out = Command::cargo_bin("pilot")?
        .env("HOME", &home)
        .env("PILOT_HOME", &pilot_home)
        .env("PILOT_DB_PORT", "9343")
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
        if is_runtime_db_denied(&stderr) {
            eprintln!("Skipping test: managed Postgres denied by runtime environment.");
            return Ok(());
        }
        let full = format!(
            "agorg create failed unexpectedly.\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&create_out.stdout),
            stderr
        );
        return Err(full.into());
    }

    Command::cargo_bin("pilot")?
        .env("HOME", &home)
        .env("PILOT_HOME", &pilot_home)
        .env("PILOT_DB_PORT", "9343")
        .env("PILOT_DB_MODE", "unix_socket")
        .arg("agorg")
        .arg("use")
        .arg(&org_name)
        .assert()
        .success();

    // 1. Invalid kind for get
    Command::cargo_bin("pilot")?
        .env("HOME", &home)
        .env("PILOT_HOME", &pilot_home)
        .env("PILOT_DB_PORT", "9343")
        .env("PILOT_DB_MODE", "unix_socket")
        .arg("policy")
        .arg("get")
        .arg("--kind")
        .arg("nonexistent")
        .assert()
        .success() // CLI returns Ok(CommandReport) even if none found
        .stdout(predicates::str::contains("No nonexistent policy found"));

    // 2. Invalid kind for preview
    Command::cargo_bin("pilot")?
        .env("HOME", &home)
        .env("PILOT_HOME", &pilot_home)
        .env("PILOT_DB_PORT", "9343")
        .env("PILOT_DB_MODE", "unix_socket")
        .arg("policy")
        .arg("preview")
        .arg("--kind")
        .arg("imaginary")
        .arg("--version")
        .arg("1")
        .assert()
        .failure()
        .stderr(predicates::str::contains("Preview currently supports"));

    Ok(())
}

#[test]
#[allow(deprecated)]
fn test_policy_malformed_json() -> Result<(), Box<dyn std::error::Error>> {
    let temp_root = std::path::Path::new(".pilot_test_tmp");
    fs::create_dir_all(temp_root)?;
    let temp = tempfile::Builder::new()
        .prefix("policy_adv_json_")
        .tempdir_in(temp_root)?;
    let suffix = temp
        .path()
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("x");
    let org_name = format!("MalformedOrg-{}", suffix);
    let home = temp.path().join("home");
    let pilot_home = std::path::PathBuf::from(format!("/tmp/pilotdb_adv_json_{}", suffix));
    fs::create_dir_all(&home)?;
    if skip_if_db_env_denied(&home, &pilot_home, "9344")? {
        return Ok(());
    }

    // Setup AGOrg
    let create_out = Command::cargo_bin("pilot")?
        .env("HOME", &home)
        .env("PILOT_HOME", &pilot_home)
        .env("PILOT_DB_PORT", "9344")
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
        if is_runtime_db_denied(&stderr) {
            eprintln!("Skipping test: managed Postgres denied by runtime environment.");
            return Ok(());
        }
        let full = format!(
            "agorg create failed unexpectedly.\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&create_out.stdout),
            stderr
        );
        return Err(full.into());
    }

    Command::cargo_bin("pilot")?
        .env("HOME", &home)
        .env("PILOT_HOME", &pilot_home)
        .env("PILOT_DB_PORT", "9344")
        .env("PILOT_DB_MODE", "unix_socket")
        .arg("agorg")
        .arg("use")
        .arg(&org_name)
        .assert()
        .success();

    let broken_file = temp.path().join("broken.json");
    fs::write(
        &broken_file,
        "{ \"kind\": \"security\", \"version\": \"oops\" }",
    )?; // version should be i32

    Command::cargo_bin("pilot")?
        .env("HOME", &home)
        .env("PILOT_HOME", &pilot_home)
        .env("PILOT_DB_PORT", "9344")
        .env("PILOT_DB_MODE", "unix_socket")
        .arg("policy")
        .arg("set-draft")
        .arg("--kind")
        .arg("security")
        .arg("--file")
        .arg(&broken_file)
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "Invalid security policy payload schema",
        ));

    Ok(())
}

#[test]
#[allow(deprecated)]
fn test_policy_no_active_agorg() -> Result<(), Box<dyn std::error::Error>> {
    let temp_root = std::path::Path::new(".pilot_test_tmp");
    fs::create_dir_all(temp_root)?;
    let temp = tempfile::Builder::new()
        .prefix("policy_adv_scope_")
        .tempdir_in(temp_root)?;
    let suffix = temp
        .path()
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("x");
    let home = temp.path().join("home");
    let pilot_home = std::path::PathBuf::from(format!("/tmp/pilotdb_adv_scope_{}", suffix));
    fs::create_dir_all(&home)?;
    if skip_if_db_env_denied(&home, &pilot_home, "9345")? {
        return Ok(());
    }

    // Try to get policy without active AGOrg in a fresh HOME
    let out = Command::cargo_bin("pilot")?
        .env("HOME", &home)
        .env("PILOT_HOME", &pilot_home)
        .env("PILOT_DB_PORT", "9345")
        .env("PILOT_DB_MODE", "unix_socket")
        .arg("policy")
        .arg("get")
        .arg("--kind")
        .arg("branch")
        .output()?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        if is_runtime_db_denied(&stderr) {
            eprintln!("Skipping test: managed Postgres denied by runtime environment.");
            return Ok(());
        }
        assert!(
            stderr.contains("No active AGOrg"),
            "Unexpected stderr for no-active-AGOrg test:\n{}",
            stderr
        );
        return Ok(());
    }
    return Err("policy get unexpectedly succeeded without active AGOrg".into());
}
