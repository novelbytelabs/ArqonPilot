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

fn setup_agorg(
    org_name: &str,
    root: &std::path::Path,
    home: &std::path::Path,
    pilot_home: &std::path::Path,
) -> Result<bool, Box<dyn std::error::Error>> {
    let out = Command::cargo_bin("pilot")?
        .env("HOME", home)
        .env("PILOT_HOME", pilot_home)
        .env("PILOT_DB_PORT", "9341")
        .env("PILOT_DB_MODE", "unix_socket")
        .arg("agorg")
        .arg("create")
        .arg("--name")
        .arg(org_name)
        .arg("--root")
        .arg(root.to_string_lossy().to_string())
        .output()?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        if (stderr.contains("Permission denied") && stderr.contains("shared memory"))
            || stderr.contains("could not open shared memory segment")
            || stderr.contains("Operation not permitted")
        {
            eprintln!(
                "Skipping test: managed Postgres shared-memory denied by runtime environment."
            );
            return Ok(true);
        }
        let full = format!(
            "agorg create failed unexpectedly.\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&out.stdout),
            stderr
        );
        return Err(full.into());
    }

    Command::cargo_bin("pilot")?
        .env("HOME", home)
        .env("PILOT_HOME", pilot_home)
        .env("PILOT_DB_PORT", "9341")
        .env("PILOT_DB_MODE", "unix_socket")
        .arg("agorg")
        .arg("use")
        .arg(org_name)
        .assert()
        .success();

    Ok(false)
}

#[test]
#[allow(deprecated)]
fn test_policy_parity_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let temp_root = std::path::Path::new(".pilot_test_tmp");
    fs::create_dir_all(temp_root)?;
    let temp = tempfile::Builder::new()
        .prefix("policy_parity_")
        .tempdir_in(temp_root)?;
    let suffix = temp
        .path()
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("x");
    let home = temp.path().join("home");
    let pilot_home = std::path::PathBuf::from(format!("/tmp/pilotdb_parity_{}", suffix));
    let repo_root = temp.path().join("mock-repo");
    fs::create_dir_all(&home)?;
    fs::create_dir_all(&repo_root)?;
    if skip_if_db_env_denied(&home, &pilot_home, "9341")? {
        return Ok(());
    }

    let org_name = format!("ParityRustTest-{}", suffix);
    if setup_agorg(&org_name, &repo_root, &home, &pilot_home)? {
        return Ok(());
    }

    let kinds = vec![
        "branch",
        "dependency",
        "release",
        "security",
        "quality",
        "runtime",
    ];

    for kind in kinds {
        let policy_file = temp.path().join(format!("policy_{}.json", kind));
        let payload = match kind {
            "branch" => {
                r#"{
                "kind": "branch",
                "version": 1,
                "naming": {
                    "level": "block",
                    "required_prefix": ["feat", "fix"],
                    "separator": "/",
                    "body_format": "kebab-case",
                    "max_length": 80
                },
                "protected_branches": {
                    "level": "block",
                    "patterns": ["main"],
                    "confirmation_type": "typed_phrase",
                    "confirmation_phrase": "CONFIRM"
                },
                "lifecycle": {
                    "auto_prune_merged": { "level": "off", "enabled": false },
                    "prune_requires_confirmation": true,
                    "confirmation_phrase": "PRUNE",
                    "prune_confirmation_type": "typed_phrase",
                    "max_stale_days": { "level": "off", "days": 30 }
                },
                "sync": {
                    "strategy": "ff-only",
                    "auto_fetch_before_sync": true
                },
                "create": {
                    "require_preview": true,
                    "base_branch_default": "main"
                }
            }"#
            }
            "dependency" => {
                r#"{
                "kind": "dependency",
                "version": 1,
                "allowed_registries": { "level": "off", "items": [] },
                "banned_packages": { "level": "block", "items": ["left-pad"] },
                "allowed_licenses": { "level": "warn", "items": ["MIT"] },
                "require_lockfile": { "level": "block", "enabled": true }
            }"#
            }
            "release" => {
                r#"{
                "kind": "release",
                "version": 1,
                "require_changelog": { "level": "block", "enabled": true },
                "require_semver": { "level": "block", "enabled": true },
                "version_strategy": "semver",
                "allowed_channels": { "level": "block", "items": ["stable"] },
                "forbidden_days": { "level": "warn", "items": ["Friday"] }
            }"#
            }
            "security" => {
                r#"{
                "kind": "security",
                "version": 1,
                "max_cve_severity": "critical",
                "block_naked_secrets": { "level": "block", "enabled": true }
            }"#
            }
            "quality" => {
                r#"{
                "kind": "quality",
                "version": 1,
                "require_lint_pass": { "level": "warn", "enabled": true },
                "require_format_pass": { "level": "warn", "enabled": true },
                "require_coverage": { "level": "off", "enabled": false },
                "min_test_coverage": 0.0
            }"#
            }
            "runtime" => {
                r#"{
                "kind": "runtime",
                "version": 1,
                "require_dockerfile": { "level": "off", "enabled": false },
                "require_healthcheck": { "level": "off", "enabled": false },
                "allowed_base_images": { "level": "block", "items": ["alpine"] }
            }"#
            }
            _ => unreachable!(),
        };

        fs::write(&policy_file, payload)?;

        // Set draft
        let mut set_cmd = Command::cargo_bin("pilot")?;
        set_cmd
            .env("HOME", &home)
            .env("PILOT_HOME", &pilot_home)
            .env("PILOT_DB_PORT", "9341")
            .env("PILOT_DB_MODE", "unix_socket")
            .arg("policy")
            .arg("set-draft")
            .arg("--kind")
            .arg(kind)
            .arg("--file")
            .arg(&policy_file)
            .assert()
            .success();

        // Get and compare
        let mut get_cmd = Command::cargo_bin("pilot")?;
        let output = get_cmd
            .env("HOME", &home)
            .env("PILOT_HOME", &pilot_home)
            .env("PILOT_DB_PORT", "9341")
            .env("PILOT_DB_MODE", "unix_socket")
            .arg("policy")
            .arg("get")
            .arg("--kind")
            .arg(kind)
            .output()?;

        let actual: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        let expected: serde_json::Value = serde_json::from_str(payload)?;

        assert_eq!(actual, expected, "Policy mismatch for kind: {}", kind);
    }

    Ok(())
}
