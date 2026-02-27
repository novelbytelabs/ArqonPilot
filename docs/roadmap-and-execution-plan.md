# ArqonPilot Unified Master Plan

This is the canonical execution plan for ArqonPilot. It merges product delivery, guardrails, testing, and documentation into one system plan so work does not fragment across sessions.

## Frozen Policy (Non-Negotiable)

1. Core lane Rust/Cargo: `1.82.0`
2. Packaging lane Rust: `1.88.0`
3. Protobuf: `4.25.8` (`protoc` `25.8`)
4. Source of truth: `scripts/frozen_versions.sh`

## System Vision

ArqonPilot becomes a centralized control system:

1. `Dashboard` is central command and catches everything.
2. Specialist tabs (`Oracle`, `Heal`, `Dependencies`, `Branch`, `Multi`, `Telemetry`, `Know`) provide deep controls.
3. Codex is integrated as an auditable operator with `preview -> execute -> verify -> record` flow.
4. All operations are policy-aware, dependency-aware, and telemetry-backed.

## Current State (Authoritative Snapshot)

1. Core modules exist: `oracle`, `heal`, `navigate`, `branch`, `multi`, `secure`, `plan`, `create`, `know`.
2. Bus bridge and Control Panel exist and are operational.
3. Guardrail scripts exist:
     - `scripts/prepush_gate.sh`
     - `scripts/verify_toolchain_policy.sh`
     - `scripts/verify_git_hook_policy.sh`
     - `scripts/repair_lock_182.sh`
     - `scripts/push_main.sh`
4. Gotcha tracking exists:
     - `docs/gotcha-registry.md`
5. Active risk: lockfile drift families (now including ICU `2.1.x`) and transient DNS failures.

## Program Tracks (Run In Parallel)

### Track A: Product (Control System)
1. Dashboard-first workflows.
2. Deep specialist tabs.
3. Codex action planner/executor.
4. Cross-repo dependency/branch orchestration.

### Track B: Guardrails and Drift Immunity
1. Frozen policy enforcement.
2. Deterministic pre-push gates.
3. One-shot lock drift recovery path.
4. Explicit push and CI diagnostics.

### Track C: Testing (All Levels)
1. Unit tests
2. Integration tests
3. End-to-end tests
4. Regression suite
5. Adversarial/chaos tests
6. CI parity tests (local vs CI lane comparison)

### Track D: Documentation (Comprehensive)
1. Main README
2. Operator runbook
3. Developer guide
4. Testing strategy
5. Troubleshooting
6. Gotcha registry
7. API/event schema docs
8. Incident response and recovery runbooks

## Wave Plan (Unified)

## Wave 0 to Wave 8 (Completed Baseline)
Completed foundational extraction, modularization, multi-repo core, branch/navigate, secure/heal, plan/create/know, rollout, and release readiness.

## Wave 9 (Control Panel Maturation) - Completed
Deliverables:

1. Dashboard as default central command.
2. Tab parity for high-frequency operations.
3. Unified operation timeline with status chips.

Exit criteria:

1. All key actions invokable from Dashboard.
2. Live operation statuses are visible without opening logs.

Completion notes:

1. Dashboard now acts as central command for Oracle/Heal/Dependencies/Branch/Multi operations.
2. Dependencies actions are executable from Dashboard, including safe push via `push_main.sh`.
3. Timeline, operation detail, status chips, and telemetry stream are centralized in Dashboard.

## Wave 10 (Packaging and Runtime Reliability) - Completed

Deliverables:

1. Deterministic packaging lane (`1.88.0`, `Cargo.lock.packaging`).
2. Conda/Linux runtime guidance and fixes.
3. Stable PyPI release process.

Exit criteria:

1. Packaging workflow is reproducible.
2. Install + `pilot --help` smoke documented and repeatable.

Completion notes:

1. Added packaging-lane check script (`scripts/packaging_lane_check.sh`) using Rust `1.88.0` + `Cargo.lock.packaging`.
2. Added CI parity script (`scripts/ci_parity_check.sh`) for deterministic local validation of both lanes.
3. Added CI `packaging-parity` job in `.github/workflows/ci.yml` to catch packaging drift before publish.
4. Strengthened policy checks so CI must retain both lane pins and packaging check step.

## Wave 11 (Guardrails and Dependencies System) - Completed
Deliverables:

1. Dependencies/Guardrails tab in UI.
2. Actions:
     - policy check
     - drift report
     - lock repair
     - pre-push gate
     - push-safe with summary
3. Lock drift map includes known families:
     - `time/comfy-table/wit-bindgen`
     - `blake3/constant_time_eq`
     - `globset`
     - `icu_* 2.1.x`

Exit criteria:

1. Local-pass/CI-fail has one-click diagnosis path.
2. Push failure always prints root cause + next action.

Progress update:

1. Added UI dependency action `drift` backed by `scripts/drift_report.sh` with JSON output support.
2. Added System Status bus controls: `Start Bus`, `Stop Bus`, `Bus Status`.
3. Added persisted bus health status in Dashboard (`localStorage`) for resume visibility.
4. Upgraded `scripts/push_main.sh` to classify push failures and print cause-specific remediation.
5. Added dashboard bus lifecycle controls (`Start Bus`, `Stop Bus`, `Bus Status`) and shim integration.
6. Added deterministic drift diagnostics via `scripts/drift_report.sh` and UI integration.
7. Live Event Stream moved to bottom of Dashboard to stay visible as a persistent monitoring surface.
8. Added `Export Evidence` action and `/api/evidence/export` endpoint to generate evidence bundles in `~/.pilot/reports`.
9. Hardened push summary to separate expected GitHub auth challenges (`auth_challenge_events`) from real errors.

Wave 11 closure notes:

1. Added automated post-publish visibility checks in `.github/workflows/pypi.yml`:
     - `Verify release visibility (TestPyPI)`
     - `Verify release visibility (PyPI)`
     using `scripts/verify_pypi_release.sh`.
2. Added explicit drift-family checks in CI for both lanes in `.github/workflows/ci.yml`:
     - `Drift Family Scan (Core Lock)` in core + packaging-parity jobs
     - `Drift Family Scan (Packaging Lock Report)` JSON report in packaging-parity.

## Wave 12 (Codex Ops Integration) - Completed
Deliverables:

1. Codex action contract:
     - intent
     - dry-run plan
     - execution steps
     - expected effect
     - rollback strategy
2. Dashboard Codex controls:
     - preview
     - approve
     - execute
     - reconcile
3. Audit + telemetry for every Codex action.

Exit criteria:

1. No opaque AI actions; every action is auditable.
2. Operator can replay and resume failed operations.

Progress update:

1. Added `Codex` tab in Control Panel with `Preview`, `Approve`, `Execute`, and `Reconcile` controls.
2. Added `/api/codex/action` contract endpoint with:
     - required `intent`
     - `pilot.*` command enforcement
     - normalized payload preview
     - read-only mutation protection
     - stateful contract lifecycle (`previewed -> approved -> executed|failed -> reconciled`)
     - telemetry events for preview/approved/start/complete/fail/reconciled.
3. Added durable Codex contract persistence:
     - contracts append to `~/.pilot/reports/codex_contracts.jsonl`
     - startup reload restores contract state for session resume.
4. Added Codex contract query APIs:
     - `GET /api/codex/contracts` (filterable by status)
     - `GET /api/codex/contract?contract_id=...`
5. Added UI replay/resume controls:
     - refresh contract list
     - load prior contract into form
     - retry failed contract (`approve -> execute`) from the panel.

## Wave 13 (Cross-Repo Orchestration) - Completed

Deliverables:

1. Cross-repo dependency DAG.
2. Dependency-aware branch/PR sequencing.
3. Cohort apply with staged execution.

Exit criteria:

1. Multi-repo feature flow works end-to-end from Dashboard.
2. Merge/release ordering is validated against dependency graph.

Completion summary:

1. Added dependency DAG export/reporting:
     - `pilot multi dag [--group ... --tag ...] [--dry-run] [--output ...]`
     - emits repos, edges, and stage plan.
2. Added staged cohort apply orchestration:
     - `pilot multi apply --branch <feat/x> [--base-branch dev] [--stage-size N] [--apply]`
     - executes dependency-aware stages with batch control and failure policy.
3. Added bus contracts for orchestration commands:
     - `pilot.multi.dag`
     - `pilot.multi.apply`
4. Added Multi tab controls for DAG and staged apply (dry-run + execute).
5. Added explicit action lifecycle UX for high-impact operations:
     - Multi: DAG + Staged Apply chips (`idle/running/success/failed`) with button lock/unlock.
     - Oracle tab: Scan/Query chips and running-state controls.
     - Heal tab: Plan/Run chips and running-state controls.
     - Dashboard quick ops: Oracle/Heal chips and running-state controls.
6. Added tab-level "Recommended Sequence" strips for Dashboard, Oracle, Heal, and Multi so operator flow is obvious at first glance.
7. Hardened push reliability for DNS instability:
     - `scripts/push_main.sh` now retries transient network failures for both fetch/push.
     - final summary and failure classification remain deterministic after retry exhaustion.

## Wave 14 (Documentation and Testing Closure)

Deliverables:

1. Complete documentation set (all Track D artifacts).
2. Complete test pyramid and automation (all Track C levels).
3. Regression and adversarial suites integrated in CI.

Exit criteria:

1. New operator can run system from docs only.
2. Test matrix covers unit/integration/e2e/regression/adversarial.
3. CI parity checks prevent recurrence of lane mismatch confusion.

Progress update:

1. Added `scripts/ui_smoke_check.sh` for Control Panel + API smoke validation (shim + serve + key endpoint/action checks).
2. Updated operator/testing documentation to align with tab-level `Recommended Sequence` UX and explicit status-chip workflows.
3. Added CI `ui-smoke` job in `.github/workflows/ci.yml` to run deterministic panel/API smoke checks on push/PR.

## Wave 15 (Production Release Gate)

Deliverables:

1. Final release checklist with hard gates.
2. Versioned release evidence archive.
3. Production rollout runbook.

Exit criteria:

1. Deterministic release with reproducible artifacts.
2. Install/runbook validated in clean environment.

## Wave 16 (AGOrg Scope and Multi-Organization Control Plane)

Deliverables:

1. AGOrg model and persistence (`parent` + child AGO registry).
   - AGOrg is the parent entity; AGOs are children.
   - AGOrg may include nested AGOrgs plus AGOs.
   - AGOrg identity is UUID with unique root path constraint.
2. AGOrg CRUD + discovery in UI/CLI.
   - Includes `Create AGOrg Project` wizard.
   - Includes optional autoscan hierarchy discovery on create.
   - Includes configurable discovery scan depth.
3. Active AGOrg scope selector for whole Control Panel.
4. Per-AGOrg preferences/profile state + default AGOrg on restart.
5. Scope-aware execution for Dashboard/Oracle/Heal/Dependencies/Branch/Multi.
6. Graph-link AGOrg composition model:
   - reusable AGOrg membership across multiple parent AGOrgs
   - hard cycle prevention
7. Local Postgres ownership:
   - Pilot-managed local Postgres only (this phase)
   - automatic migrations and schema lifecycle

Exit criteria:

1. Operator can load `~/Projects/arqon/Arqon` as default AGOrg and auto-resume on restart.
2. AGOrg is always visible as active scope in header and action context.
3. Mutating actions cannot run without an explicit active AGOrg scope.
4. Discovery can find AGOrg/AGO candidates from a root directory and support selective import.
5. All AGOrg operations are auditable and replayable.
6. Nested AGOrg hierarchies render correctly in AGOrg tree view.
7. Create + autoscan flow can bootstrap Arqon AGOrg in one operation.
8. AGOrg linking permits recombination without conflict while deterministically blocking circular loops.

Planning source:

1. `docs/agorg-control-plane-plan.md`

## Operational Rules

1. Never bypass frozen policy constants.
2. Every guardrail change must update docs in the same PR/commit.
3. Every new failure class must be added to `docs/gotcha-registry.md`.
4. Push via `./scripts/push_main.sh` for actionable summary output.
5. If DNS/index is down, treat as environment incident first, not code failure.

## Resume Checklist

When resuming after interruption:

1. `./scripts/verify_toolchain_policy.sh`
2. If policy fails: `./scripts/repair_lock_182.sh --no-gate`
3. `./scripts/prepush_gate.sh`
4. `./scripts/push_main.sh`
5. Check latest logs in `~/.pilot/reports/` (fallback `/tmp/pilot-reports/`)
6. Update `docs/gotcha-registry.md` if a new signature appeared.

## What Success Looks Like

1. Dashboard is the real control center.
2. Tabs are specialist tools, not separate systems.
3. Codex is integrated with auditable, deterministic operations.
4. Guardrails prevent drift before CI surprises.
5. Documentation and testing are complete enough for production use.
