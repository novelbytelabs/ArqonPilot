# Federated Gate Contract Schema

**Version**: 1.0
**Generated**: 2026-03-03T14:31:00Z
**Status**: CANONICAL

## Overview

This document defines the canonical contract schema for federated CI gates in the Arqon Pilot system. The schema specifies inputs, outputs, failure codes, remediation hints, and evidence pointers for all gate operations.

## Gate Types

| Gate Type | Description | Script |
|-----------|-------------|--------|
| toolchain_policy | Verify frozen toolchain versions | `verify_toolchain_policy.sh` |
| prepush_gate | Pre-push validation | `prepush_gate.sh` |
| push_safe | Safe push with diagnostics | `push_main.sh` |
| lock_repair | Lockfile repair for Rust 1.82 | `repair_lock_182.sh` |
| ci_parity | Local/CI parity check | `ci_parity_check.sh` |
| packaging_lane | Packaging toolchain check | `packaging_lane_check.sh` |

## 1. Toolchain Policy Gate

### Inputs

```json
{
  "core_rust_version": "1.82.0",
  "packaging_rust_version": "1.88.0",
  "protoc_version": "4.25.8",
  "lockfiles": ["Cargo.lock", "Cargo.lock.packaging"],
  "ci_workflows": [".github/workflows/ci.yml", ".github/workflows/pypi.yml"]
}
```

### Outputs

```json
{
  "ok": true,
  "failed_checks": [],
  "timestamp": "2026-03-03T14:29:00Z"
}
```

### Failure Codes

| Code | Check | Description |
|------|-------|-------------|
| E001 | rust_toolchain_pin | rust-toolchain.toml not pinned to 1.82.0 |
| E002 | core_toolchain_installed | Core Rust 1.82.0 not installed |
| E003 | ci_lane_pin | CI workflow not pinned to correct toolchain |
| E004 | ci_protoc_pin | CI workflow missing protoc 4.25.8 |
| E005 | packaging_lane_pin | PyPI workflow not pinned to 1.88.0 |
| E006 | packaging_lock_exists | Cargo.lock.packaging missing |
| E007 | core_lock_compatibility | Lockfile contains 1.82-incompatible deps |
| E008 | version_alignment | pyproject.toml vs Cargo.toml version mismatch |

### Remediation

| Code | Remediation Steps |
|------|------------------|
| E001 | Add `channel = "1.82.0"` to rust-toolchain.toml |
| E002 | Run `rustup toolchain install 1.82.0` |
| E003 | Update .github/workflows/ci.yml toolchain to "1.82.0" |
| E004 | Add protoc 4.25.8 download step to CI |
| E005 | Update .github/workflows/pypi.yml toolchain to "1.88.0" |
| E006 | Run `cp Cargo.lock Cargo.lock.packaging` |
| E007 | Run `./scripts/repair_lock_182.sh --no-gate` |
| E008 | Align versions in pyproject.toml and Cargo.toml |

### Evidence Pointers

- Log file: `~/.pilot/reports/toolchain_policy_<stamp>.log`
- JSON output: `~/.pilot/reports/toolchain_policy_<stamp>.json`

---

## 2. Pre-Push Gate

### Inputs

```json
{
  "target_branch": "main",
  "target_remote": "origin",
  "core_rust": "1.82.0",
  "tests": ["branch_cli_test", "multi_cli_test", "navigate_cli_test", "secure_cli_test", "plan_cli_test", "create_cli_test", "know_cli_test", "heal_cli_test", "oracle_cli_test", "report_cli_test"]
}
```

### Outputs

```json
{
  "status": "PASS|FAIL",
  "steps": {
    "toolchain_policy": "PASS|FAIL",
    "locked_compile": "PASS|FAIL",
    "targeted_tests": "PASS|FAIL",
    "help_surface": "PASS|FAIL"
  },
  "log_file": "~/.pilot/reports/prepush_gate_<stamp>.log",
  "timestamp": "2026-03-03T14:30:00Z"
}
```

### Failure Codes

| Code | Step | Description |
|------|------|-------------|
| P001 | toolchain_policy | Toolchain policy verification failed |
| P002 | locked_compile | `cargo check -p pilot --locked` failed |
| P003 | targeted_tests | One or more CLI tests failed |
| P004 | help_surface | Help surface smoke check failed |
| P005 | transient_network | Network/DNS transient failure during gate |

### Remediation

| Code | Remediation Steps |
|------|------------------|
| P001 | Run `./scripts/verify_toolchain_policy.sh` and fix failures |
| P002 | Run `./scripts/repair_lock_182.sh --no-gate` then re-run gate |
| P003 | Run `cargo test -p pilot --locked <test_name>` to diagnose |
| P004 | Run `cargo run -q -p pilot -- --help` and verify all commands listed |
| P005 | Re-run `./scripts/prepush_gate.sh` (gate has retry logic) |

### Evidence Pointers

- Log file: `~/.pilot/reports/prepush_gate_<stamp>.log`
- Pre-push status: `[pre-push] status: PASS|FAIL`

---

## 3. Push Safe Wrapper

### Inputs

```json
{
  "branch": "main",
  "remote": "origin",
  "push_retries": 6,
  "dns_preflight_attempts": 12
}
```

### Outputs

```json
{
  "result": "SUCCESS|FAILED",
  "branch": "main",
  "remote": "origin",
  "duration_sec": 45,
  "prepush_gate_rc": 0,
  "git_push_rc": 0,
  "warnings_in_log": 2,
  "errors_in_log": 0,
  "auth_challenge_events": 1,
  "divergence_after_push": "behind=0 ahead=0",
  "likely_cause": "none",
  "full_log": "~/.pilot/reports/push_main_<stamp>.log"
}
```

### Failure Codes

| Code | Category | Description |
|------|----------|-------------|
| S001 | prepush_gate_failed | Pre-push gate returned non-zero |
| S002 | auth_or_token | Authentication/token failure |
| S003 | non_fast_forward_or_remote_ahead | Non-fast-forward or remote ahead |
| S004 | dns_or_name_resolution | DNS resolution failure |
| S005 | network_transport_instability | Network transport issues |
| S006 | branch_protection_or_server_hook | Branch protection blocked push |
| S007 | git_push_failed_uncategorized | Uncategorized git push failure |

### Remediation

| Code | Remediation Steps |
|------|------------------|
| S001 | Run repair_lock_182.sh, then prepush_gate.sh, then retry |
| S002 | Run `gh auth status` and `gh auth login` |
| S003 | Run `git fetch`, `git pull --rebase`, then retry |
| S004 | Run `getent hosts github.com`, flush DNS cache, retry |
| S005 | Wait for network stability, retry |
| S006 | Push to dev/feature branch and open PR instead |
| S007 | Inspect full log for root cause |

### Evidence Pointers

- Full log: `~/.pilot/reports/push_main_<stamp>.log`
- Summary block at end of log with all metrics

---

## 4. Lock Repair

### Inputs

```json
{
  "core_rust_version": "1.82.0",
  "lockfile": "Cargo.lock",
  "known_drift_sources": ["comfy-table", "time", "wit-bindgen", "constant_time_eq", "globset", "icu_*"]
}
```

### Outputs

```json
{
  "status": "REPAIRED|NO_DRIFT|FAILED",
  "packages_pinned": ["comfy-table@7.1.0", "time@0.3.36"],
  "log_file": "~/.pilot/reports/repair_lock_<stamp>.log"
}
```

### Failure Codes

| Code | Description |
|------|-------------|
| R001 | Failed to parse Cargo.lock |
| R002 | Could not find compatible version for drift source |
| R003 | Cargo update failed after pinning |
| R004 | Network failure during repair |

### Remediation

| Code | Remediation Steps |
|------|------------------|
| R001 | Check Cargo.lock syntax, restore from git if corrupted |
| R002 | Manually specify compatible version in [patch.crates-io] |
| R003 | Check for conflicting dependencies, try `--no-gate` flag |
| R004 | Ensure network connectivity to crates.io |

---

## 5. CI Parity Check

### Inputs

```json
{
  "core_rust": "1.82.0",
  "packaging_rust": "1.88.0",
  "local_lanes": ["core", "packaging"],
  "ci_lanes": ["core", "packaging", "ui-smoke"]
}
```

### Outputs

```json
{
  "ok": true,
  "parity_status": {
    "core": "MATCH|MISMATCH",
    "packaging": "MATCH|MISMATCH"
  },
  "log_file": "~/.pilot/reports/ci_parity_<stamp>.log"
}
```

### Failure Codes

| Code | Description |
|------|-------------|
| C001 | Local core lane differs from CI core lane |
| C002 | Local packaging lane differs from CI packaging lane |
| C003 | Missing required toolchain locally |

### Remediation

| Code | Remediation Steps |
|------|------------------|
| C001 | Run `cargo update`, verify lock matches CI |
| C002 | Sync packaging lock: `cp Cargo.lock Cargo.lock.packaging` |
| C003 | Install missing toolchain via rustup |

---

## Canonical Failure Envelope

All gate failures follow this envelope:

```json
{
  "gate": "toolchain_policy|prepush_gate|push_safe|lock_repair|ci_parity",
  "status": "PASS|FAIL",
  "error_code": "E001|P001|S001|R001|C001",
  "error_message": "Human-readable error description",
  "remediation": ["step 1", "step 2", "step 3"],
  "evidence": {
    "log_file": "/path/to/log",
    "artifact_path": "/path/to/artifact"
  },
  "timestamp": "ISO8601 timestamp",
  "retryable": true|false
}
```

---

## Integration Points

| System | Integration |
|--------|-------------|
| CLI | All gates accessible via `./scripts/*.sh` |
| UI | Dashboard runs gates via API endpoints |
| CI | CI jobs delegate to canonical scripts |
| Governance | Policy gates integrated with governance store |

---

## Summary

This canonical contract schema provides:
- **5 gate types** with defined inputs/outputs
- **20+ failure codes** covering all known failure modes
- **Remediation paths** for each failure code
- **Evidence pointers** for audit and debugging
- **Canonical failure envelope** for consistent error handling

**Federated Gate Contract Schema**: ✅ CANONICAL
