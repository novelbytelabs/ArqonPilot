use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn setup_agorg(org_name: &str, root: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("pilot")?;
    cmd.arg("agorg")
        .arg("create")
        .arg("--name")
        .arg(org_name)
        .arg("--root")
        .arg(root.to_string_lossy().to_string())
        .assert()
        .success();

    let mut use_cmd = Command::cargo_bin("pilot")?;
    use_cmd.arg("agorg")
        .arg("use")
        .arg(org_name)
        .assert()
        .success();

    Ok(())
}

#[test]
#[allow(deprecated)]
fn test_policy_parity_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let repo_root = temp.path().join("mock-repo");
    fs::create_dir_all(&repo_root)?;
    
    setup_agorg("ParityRustTest", &repo_root)?;

    let kinds = vec!["branch", "dependency", "release", "security", "quality", "runtime"];

    for kind in kinds {
        let policy_file = temp.path().join(format!("policy_{}.json", kind));
        let payload = match kind {
            "branch" => r#"{
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
            }"#,
            "dependency" => r#"{
                "kind": "dependency",
                "version": 1,
                "allowed_registries": { "level": "off", "items": [] },
                "banned_packages": { "level": "block", "items": ["left-pad"] },
                "allowed_licenses": { "level": "warn", "items": ["MIT"] },
                "require_lockfile": { "level": "block", "enabled": true }
            }"#,
            "release" => r#"{
                "kind": "release",
                "version": 1,
                "require_changelog": { "level": "block", "enabled": true },
                "require_semver": { "level": "block", "enabled": true },
                "version_strategy": "semver",
                "allowed_channels": { "level": "block", "items": ["stable"] },
                "forbidden_days": { "level": "warn", "items": ["Friday"] }
            }"#,
            "security" => r#"{
                "kind": "security",
                "version": 1,
                "max_cve_severity": "critical",
                "block_naked_secrets": { "level": "block", "enabled": true }
            }"#,
            "quality" => r#"{
                "kind": "quality",
                "version": 1,
                "require_lint_pass": { "level": "warn", "enabled": true },
                "require_format_pass": { "level": "warn", "enabled": true },
                "require_coverage": { "level": "off", "enabled": false },
                "min_test_coverage": 0.0
            }"#,
            "runtime" => r#"{
                "kind": "runtime",
                "version": 1,
                "require_dockerfile": { "level": "off", "enabled": false },
                "require_healthcheck": { "level": "off", "enabled": false },
                "allowed_base_images": { "level": "block", "items": ["alpine"] }
            }"#,
            _ => unreachable!(),
        };

        fs::write(&policy_file, payload)?;

        // Set draft
        let mut set_cmd = Command::cargo_bin("pilot")?;
        set_cmd.arg("policy")
            .arg("set-draft")
            .arg("--kind")
            .arg(kind)
            .arg("--file")
            .arg(&policy_file)
            .assert()
            .success();

        // Get and compare
        let mut get_cmd = Command::cargo_bin("pilot")?;
        let output = get_cmd.arg("policy")
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
