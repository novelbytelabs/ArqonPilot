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

## Wave 10 (Packaging and Runtime Reliability) - Current Focus
Deliverables:
1. Deterministic packaging lane (`1.88.0`, `Cargo.lock.packaging`).
2. Conda/Linux runtime guidance and fixes.
3. Stable PyPI release process.

Exit criteria:
1. Packaging workflow is reproducible.
2. Install + `pilot --help` smoke documented and repeatable.

## Wave 11 (Guardrails and Dependencies System) - In Progress
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

## Wave 12 (Codex Ops Integration)
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

## Wave 13 (Cross-Repo Orchestration)
Deliverables:
1. Cross-repo dependency DAG.
2. Dependency-aware branch/PR sequencing.
3. Cohort apply with staged execution.

Exit criteria:
1. Multi-repo feature flow works end-to-end from Dashboard.
2. Merge/release ordering is validated against dependency graph.

## Wave 14 (Documentation and Testing Closure)
Deliverables:
1. Complete documentation set (all Track D artifacts).
2. Complete test pyramid and automation (all Track C levels).
3. Regression and adversarial suites integrated in CI.

Exit criteria:
1. New operator can run system from docs only.
2. Test matrix covers unit/integration/e2e/regression/adversarial.
3. CI parity checks prevent recurrence of lane mismatch confusion.

## Wave 15 (Production Release Gate)
Deliverables:
1. Final release checklist with hard gates.
2. Versioned release evidence archive.
3. Production rollout runbook.

Exit criteria:
1. Deterministic release with reproducible artifacts.
2. Install/runbook validated in clean environment.

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
