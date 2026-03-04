# FC-9 Post-Audit Remediation Evidence (2026-03-03)

**Status**: COMPLETE  
**Reason**: Independent audit found regressions that invalidated release-grade confidence until remediated.

## Scope of Remediation

1. P4 branch test regressions were breaking full locked test suite.
2. Release-readiness false-fail under constrained sandbox due DB shared-memory denial required host-verification evidence.
3. Reconcile artifact test (`test_persist_agorg_reconcile_writes_governance_sidecar_when_fleet_report_present`) needed deterministic writable report root.

## Code Fixes Applied

1. Removed duplicate `ConflictRadarResult` type in `crates/pilot-branch/src/lib.rs`.
2. Hardened Pilot data/report root behavior in `crates/pilot/src/serve_ui.rs`:
   - `PILOT_HOME` override support retained.
   - `reports_root()` now verifies writeability and falls back to `cwd/.pilot/reports` when needed.
3. Made reconcile-sidecar unit test hermetic by forcing temporary `PILOT_HOME` and validating write probe before assertions.
4. Added/retained targeted P4 test coverage files:
   - `crates/pilot/tests/p4_branch_holy_grail_test.rs`
   - `crates/pilot/tests/p4_branch_adversarial_test.rs`

## Verification Commands Executed

```bash
cargo test -p pilot --locked --test p4_branch_holy_grail_test --test p4_branch_adversarial_test
cargo test -p pilot --locked --bin pilot test_persist_agorg_reconcile_writes_governance_sidecar_when_fleet_report_present -- --nocapture
./scripts/ci_contract_parity_test.sh
./scripts/release_readiness_check.sh
```

## Verification Results

1. P4 targeted tests: **PASS** (`25 passed; 0 failed`).
2. Reconcile sidecar persistence test: **PASS**.
3. FC-5 parity script: **PASS** (`14 passed; 0 failed`).
4. Release readiness gate: **PASS** on host-permission run, including:
   - full locked compile
   - full locked test suite
   - command surface checks
   - JS syntax + duplicate-const checks
   - rust-toolchain pin checks

## Gotcha Mapping

- `G-041`: constrained runtimes can fail managed Postgres startup with shared-memory permission errors (`/PostgreSQL.* Permission denied`) while logic is healthy.
- `G-043`: report artifact write permissions can cause warning/failure signatures if report root is not writable.

## Hard-Close Statement

FC-9 remains HARD-CLOSED with this post-audit remediation packet attached.
All previously identified audit blockers are resolved with reproducible evidence.
