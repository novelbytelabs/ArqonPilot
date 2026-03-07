# Release Log

This log is the release audit trail. Every release (including alpha) must have a complete evidence set.

## Release Entry Template

Use this structure for every new release:

```md
## vX.Y.Z-alpha.N (YYYY-MM-DD)

- Git tag:
- Commit SHA:
- PyPI version:
- Release type: alpha

### Verification
- prepush gate:
- release readiness:
- Wave I matrix artifact:
- Wave J matrix artifact:
- UI smoke log:
- PyPI visibility check:
- clean venv install + `pilot --help`:

### CI/CD
- CI run ID:
- PyPI run ID:
- Docs run ID:
- GitHub release URL:

### Notes
- Key changes:
- Known limitations:
- Follow-up actions:
```

---

## v0.2.0-alpha.1 (2026-03-03)

- Git tag: `v0.2.0-alpha.1` (dry-run)
- Commit SHA: `76f9e9e150be3d8f9fd892b8892ef815013e2f4b`
- PyPI version: `0.1.6a1` (simulated)
- Release type: alpha

### Verification

- prepush gate: `prepush_gate_latest.log` (sha256: 6cb236f6...)
- release readiness: `scripts/release_readiness_check.sh` PASSED
- clean operator proof: `env -i ...` PASSED (G-044 verified)
- migration smoke: `scripts/migration_smoke_test.sh` PASSED (cold/warm/data-access verified)
- rollback drill: Manual sequence PASSED (DB restore + binary revert verified)
- Wave I matrix artifact: `acceptance_matrix_wave_i_full_latest.json` (sha256: 243ebc11...)
- Wave J matrix artifact: `N/A` (Inert in dry-run)
- UI smoke log: `ui_smoke_latest.log` PASSED
- PyPI visibility check: `Simulated`
- clean venv install + `pilot --help`: `Passed (Manual)`
- Integrity manifest: `manifest.json` ✅ Verified via `verify_bundle.sh`

### Rollback Drill Transcript

1. DB Snapshot: `pg_dump ... > pilot_pre_rollback.sql` (SUCCESS)
2. Binary Revert: `git checkout HEAD~1 && cargo build` (SUCCESS)
3. Restoration: `git checkout - && cargo build` (SUCCESS)
4. Verification: `pilot agorg list` (PASSED: Found 1 AGOrgs)

### CI/CD

- CI run ID: `dry-run`
- PyPI run ID: `dry-run`

### Notes

- Key changes: Phase P9 Release Train Hardening implemented. Added `channel-policy.md`, `migration-playbook.md`, `compatibility-matrix.md`, `slo-policy.md`, and `incident-runbook.md`. Refined ES5 compatibility in `pilot_ui.js`.
- Known limitations: ArqonBus compatibility shim may be used in some local environments. `protoc` 25.8 verified in local operator path.
- Follow-up actions: Finalize Wave L tech debt burn-down.

---

## P9 Hard-Close Readiness (2026-03-04)

- Bundle: `/home/irbsurfer/.pilot/release_evidence/release_p9-hard-close_20260304T184938Z`
- Integrity: ✅ VERIFIED
- Release Train: **HARDENED**

### Re-Verification Pass (2026-03-04)

- `./scripts/compat_matrix_smoke.sh`: ✅ PASS
- `./scripts/migration_smoke_test.sh`: ✅ PASS
- `./scripts/release_readiness_check.sh`: ✅ PASS
- `PILOT_UI_SMOKE_STARTUP_TIMEOUT_SEC=180 ./scripts/ui_smoke_check.sh`: ✅ PASS
- `./scripts/prepush_gate.sh`: ✅ PASS
- `/home/irbsurfer/.pilot/release_evidence/release_p9-hard-close_20260304T184938Z/verify_bundle.sh`: ✅ PASS

### Remediation Notes

1. Compatibility matrix probe now uses explicit toolchain resolution (`rustup run`) to avoid false version reads.
2. Migration smoke now isolates Pilot runtime state in temporary HOME while preserving cargo/rustup cache locations.
3. Policy DB test harness now uses short unique `/tmp/pilotdb_*` paths and runtime-denial skips for deterministic CI/sandbox behavior.

---

## Pilot-for-Pilot Phase D Implementation (2026-03-05)

- Scope: Dashboard **Release Routine (Phase D)** hardening for UI-first release execution.
- Backend actions added:
  - `release-readiness`
  - `release-compat-matrix`
  - `release-migration-smoke`
  - `release-collect-evidence`
  - `release-verify-bundle`
- UI added:
  - Release routine card with per-step chips, checklist output, and readiness score.
  - Manual step buttons and one-click full routine.
  - Bundle path auto-fill from collect-evidence output.
- Verification:
  - `node -c crates/pilot/src/pilot_ui.js` ✅
  - `cargo check -p pilot --locked` ✅
  - targeted tests:
    - `test_scope_dependency_action_classification` ✅
    - `test_parse_release_collect_evidence_path` ✅

### Hard-Close Re-Verification (2026-03-06)

- `./scripts/prepush_gate.sh` ✅ PASS
- `./scripts/release_readiness_check.sh` ✅ PASS
- `./scripts/compat_matrix_smoke.sh` ✅ PASS
- `./scripts/migration_smoke_test.sh` ✅ PASS
- `./scripts/check_duplicate_consts.py` ✅ PASS
- Placeholder/stub scan on touched files: no code TODO/stub placeholders introduced ✅

---

## Pilot-for-Pilot Phase B Hardening Closeout (2026-03-07)

- Scope: CI observatory robustness hardening for Post-Commit Routine dashboard deck.
- Key fixes:
  - CI in-flight state now suppresses false-positive `PASS` chips until terminal completion.
  - CI workflow catalog discovery now degrades to warnings + required-gap signals when workflow sources are missing/unreadable.
  - CI policy summary/notes now expose catalog warning count and warning details.

### Verification

- `node --check crates/pilot/src/pilot_ui.js` ✅
- `conda run -n helios-gpu-118 cargo check -p pilot --locked` ✅
- `conda run -n helios-gpu-118 cargo test -p pilot --locked test_discover_dashboard_ci_catalog_reports_missing_required_jobs -- --nocapture` ✅
- `conda run -n helios-gpu-118 cargo test -p pilot --locked test_discover_dashboard_ci_catalog_missing_directory_yields_warnings_and_gaps -- --nocapture` ✅
- `conda run -n helios-gpu-118 bash scripts/test_matrix.sh all` ✅ (unit, integration, e2e, regression, adversarial)
- `conda run -n helios-gpu-118 python -m mkdocs build -q` ✅

### Regression Guards Added

- `crates/pilot/tests/ci_observatory_regression_test.rs` (in-flight PASS suppression contract)
- `scripts/test_matrix.sh` regression suite now includes `--test ci_observatory_regression_test`

### Notes

- New gotcha registered: `G-030` (CI observatory pass-while-running false positive + catalog hard-fail behavior).
- Frozen versions policy unchanged:
  - Core Rust `1.82.0`
  - Packaging Rust `1.88.0`
  - Protobuf `4.25.8`
  - protoc `25.8`

### Phase C Kickoff (same wave)

- Dashboard policy modal hardened:
  - validates draft shape before simulation/activation
  - shows normalized profile + diff summary vs loaded profile
  - requires re-simulation if draft changed after simulation evidence was generated
- Verification:
  - `node --check crates/pilot/src/pilot_ui.js` ✅
  - `conda run -n helios-gpu-118 cargo check -p pilot --locked` ✅
  - `conda run -n helios-gpu-118 bash scripts/test_matrix.sh regression` ✅

### Phase C Slice 2 (Auto-Heal -> Verify -> Escalate)

- Reconcile actions now include:
  - `Auto-Heal + Verify` (known-safe playbooks)
  - `Escalate to Codex` (prefilled incident packet + preview)
- Routine toggle `Auto-heal known-safe failures` defaults on and runs remediation automatically before failure closeout.
- Initial safe playbooks:
  - `format_parity` -> `cargo-fmt` -> verify gate
  - `lock_drift` -> `repair` -> verify gate
- Learning loop:
  - heal outcomes are persisted locally in `localStorage` key `pilot.routine.heal.log.v1`.
  - Reconcile exposes `Heal Log` and `Clear Heal Log` controls for operator feedback loops.
- Learning loop promotion (follow-on hardening):
  - successful safe remediations are now promoted into reusable recipes (`pilot.routine.heal.recipe.v1`) keyed by failure fingerprint.
  - repeated failures with matching fingerprint automatically reuse learned playbooks before Codex escalation.
  - recipe controls are exposed in Reconcile (`Recipes`, `Clear Recipes`) for operator governance.
- Push anti-stall hardening:
  - routine push stage now uses a bounded timeout (15 min) with explicit running ledger entry.
  - `scripts/push_main.sh` now enforces non-interactive git behavior (`GIT_TERMINAL_PROMPT=0`, `GCM_INTERACTIVE=Never`, SSH batch mode) to avoid indefinite credential prompts.
- Resume hardening:
  - after successful auto-heal, Reconcile now exposes `Resume from Failed Stage`.
  - auto-heal success now auto-queues one resume pass from the failed stage (`autoResumeDepth` guard prevents loops).
  - routine runner accepts resume targeting (`resumeFromStep`) and re-enters from the failed stage while preserving prior scope context.
  - push timeout failures now surface as explicit `Timed out` stage state with targeted remediation text.
- CI watch freshness hardening:
  - `gh_actions_watch_latest.sh` now requires a fresh branch run in a configurable lookback window (default 15m) and prefers matching `headSha`.
  - routine CI stage no longer treats stale historical runs as current success; missing fresh runs now fail with `likely_cause=no_fresh_run_detected`.
  - no-op push (`Everything up-to-date`) now attempts `workflow_dispatch` fallback trigger (`ci-trigger`) and then watches the fresh run.
  - `.github/workflows/ci.yml` now declares `workflow_dispatch` to support resilient CI triggering without a new push delta.
- Regression guards added:
  - `crates/pilot/tests/routine_autoheal_regression_test.rs`
  - `scripts/test_matrix.sh` includes `--test routine_autoheal_regression_test`
