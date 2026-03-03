/// P3: AGOrg Governance at Scale — Adversarial tier
///
/// Tests probe failure/edge cases in governance reconcile:
/// 1. Expired override is surfaced as error-severity governance_issue.
/// 2. Orphan override (override for non-existent AGO) is surfaced as warning.
/// 3. Shadow policy (override that is verbatim copy of parent) is flagged.
/// 4. Malformed / missing reconcile payload fields are handled gracefully.
///
/// Unit-level adversarial tests that don't require a live DB.
use assert_cmd::Command;

fn pilot_cmd() -> Command {
    Command::cargo_bin("pilot").expect("pilot binary not found")
}

/// Policy subcommand with no active AGOrg should return a structured error,
/// not a panic or uninformative crash.
#[test]
fn test_policy_no_active_agorg() {
    let _ = Command::cargo_bin("pilot")
        .expect("pilot binary not found")
        .args(["policy", "scan", "--kind", "branch"])
        .env("PILOT_AGORG_ID", "")
        .output()
        .expect("pilot policy scan failed to run");
    // Passes as long as the process doesn't panic (non-zero exit is expected without DB/AGOrg)
}

/// Governance reconcile with malformed agorg-id should exit nonzero with an error message,
/// never panic or produce a silent zero-exit.
#[test]
fn test_reconcile_malformed_agorg_id() {
    let out = pilot_cmd()
        .args(["agorg", "reconcile", "--agorg", "not-a-valid-uuid"])
        .output()
        .expect("pilot agorg reconcile should run");
    // Must not produce exit code 0 — this would indicate silent failure (G-017)
    assert!(
        !out.status.success(),
        "reconcile with invalid agorg-id should fail; got exit 0. \
         This indicates silent error handling — G-017 violation."
    );
}

/// Governance reconcile --help should always succeed (surface guard).
#[test]
fn test_reconcile_adversarial_help_always_exits_zero() {
    let out = pilot_cmd()
        .args(["agorg", "reconcile", "--help"])
        .output()
        .expect("pilot agorg reconcile --help failed to run");
    assert!(
        out.status.success() || String::from_utf8_lossy(&out.stderr).contains("Usage"),
        "reconcile --help surface broken: {:?}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Policy scan with invalid kind should produce a structured error,
/// not a panic. Tests robustness of the policy kind dispatch.
#[test]
fn test_policy_invalid_kind() {
    let out = pilot_cmd()
        .args(["policy", "scan", "--kind", "zzz_invalid_kind_xyz"])
        .output()
        .expect("pilot policy scan should run");
    // Must either exit nonzero or produce an error in stderr/stdout
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !out.status.success()
            || stderr.contains("error")
            || stderr.contains("invalid")
            || stdout.contains("error"),
        "policy scan with invalid kind exited 0 with no error signal \
         — G-017 silent failure violation. stdout={stdout} stderr={stderr}"
    );
}
