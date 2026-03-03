# Arqon Pilot Federated CI/CD Program (Ultimate System Edition)

**Status**: Active program plan  
**Last updated**: 2026-03-03  
**Primary target**: Arqon Pilot as the deterministic control plane for multi-repo engineering and governance.

## 1) Alignment Statement

This program must align with the Arqon Pilot product direction:

1. Safe-by-default operation with explicit preview before mutation.
2. Deterministic execution and replayability across local + CI + release lanes.
3. AGOrg-scoped governance across multiple repositories, not one hardcoded workspace.
4. Evidence-backed hard-close criteria; no "route exists == done" shortcuts.
5. "Holy grail" quality bar: operator-grade reliability, accessibility, and auditability.

If this plan conflicts with Arqon Pilot source-of-truth docs, Arqon Pilot docs win.

## 2) Mandatory Context Packet (Read First)

Before coding, the implementing AI must read these in order:

1. `docs/PRODUCTIONIZE.md`
2. `docs/gotcha-registry.md`
3. `docs/operator-runbook.md`
4. `docs/troubleshooting.md`
5. `docs/settings-tab-and-governance-plan.md`
6. `docs/release-playbook.md`
7. `docs/release-log.md`

Then inspect key implementation surfaces:

1. `crates/pilot/src/main.rs`
2. `crates/pilot/src/serve_ui.rs`
3. `crates/pilot/src/pilot_ui.js`
4. `crates/pilot/src/governance/store.rs`
5. `scripts/prepush_gate.sh`
6. `scripts/push_main.sh`
7. `scripts/verify_toolchain_policy.sh`

## 2.1) Context Rebuild Protocol (Required For New AI Sessions)

Before writing any code, the implementing AI must do this in order:

1. Confirm repository root:
   - `pwd`
   - `git rev-parse --show-toplevel`
2. Re-read all items in Section 2.
3. Build a short "Context Rebuild Summary" that includes:
   - current active wave
   - frozen version constraints
   - open gotchas most likely to impact the wave
4. Produce a "No-Drift Plan" with:
   - exact files to edit
   - exact tests to run
   - expected artifact outputs
5. Do not execute mutations until steps 1-4 are completed and recorded.

Execution rule:
1. Evidence-first, claim-last. No wave may be marked complete without command outputs, test outcomes, and artifact paths.

## 3) Frozen Runtime Policy (Non-Negotiable)

1. Core lane Rust/Cargo: `1.82.0`
2. Packaging lane Rust: `1.88.0`
3. Protobuf/protoc: `4.25.8`
4. Mutating controls must remain preview-first.
5. Any freeze change requires explicit policy update and script update in the same PR.

## 4) Scope

Primary federation scope:

1. `ArqonPilot`
2. `ArqonBus`
3. `ArqonLattice`
4. `ArqonStudio`
5. `ArqonHPO`

Program must support arbitrary customer AGOrgs, not Arqon-only assumptions.

## 5) Why This Program Exists

1. Prevent local-pass/CI-fail loops through strict parity contracts.
2. Move from fragmented scripts to one deterministic preflight model.
3. Make Pilot orchestrate multi-repo governance safely and observably.
4. Provide signed evidence for release and incident-grade auditability.

## 6) Critical Gotchas to Actively Guard Against

Pulled from Arqon Pilot registry; these are mandatory at runtime:

1. `G-001`, `G-002`: lock drift to edition2024/ICU incompatible with Rust 1.82.
2. `G-003`, `G-013`: DNS/index instability requires retry-aware execution.
3. `G-005`, `G-006`: lane parity failures (core vs packaging).
4. `G-007`: ArqonBus lifecycle flaps and false disconnected states.
5. `G-010`: stale installed `pilot` binary shadowing repo binary.
6. `G-014`: missing `protoc` in CI/UI smoke lane.
7. `G-015`: JS syntax regressions not caught by Rust compiler.
8. `G-017`: "complete" claims with static/stub behavior.

Program rule:
1. Every encountered failure signature must map to a known gotcha or create a new gotcha entry in the same iteration.

## 7) Program Tracks (Coordinated)

### Track A: Canonical Gate Physics

1. Define canonical gate contracts for policy/hook/drift/gate/check/test/packaging/smoke/release-readiness.
2. Ensure local scripts and CI jobs call the same contract path.
3. Enforce replayable output format and remediation guidance.

### Track B: Pilot Orchestrator Surface

1. Expose contracts through typed `pilot.ci.*` and governance workflows.
2. Preview -> execute discipline for every mutating action.
3. Provide one evidence/timeline stream across tabs.

### Track C: Federation Governance

1. AGOrg-aware execution targeting with inheritance/override semantics.
2. Conflict mediation and policy exception workflows with audit trails.
3. Protected branch and release safety enforcement.

### Track D: Reliability + Auditability

1. Supervised lifecycle for Bus/DB/UI.
2. Signed/tamper-evident evidence bundles for release.
3. Incident-ready diagnostics and replay entry points.

## 7.1) Current Wave Status and Active Target

Program wave status (authoritative):

1. FC-1: HARD-CLOSED
2. FC-2: HARD-CLOSED
3. FC-3: HARD-CLOSED
4. FC-4: HARD-CLOSED
5. FC-5: HARD-CLOSED
6. FC-6: HARD-CLOSED
7. FC-7: HARD-CLOSED
8. FC-8: ACTIVE TARGET
9. FC-9: PENDING

Immediate directive:

1. Start FC-8 now.
2. Do not reopen FC-7 unless a regression is discovered with reproducible evidence.
3. If FC-7 regression is detected, log it in gotchas and attach a corrective artifact before resuming FC-8.

## 8) Execution Waves (Federated CI Program)

### FC-1: Context and Contract Baseline

Deliver:
1. Context packet verification checklist committed into working notes/artifacts.
2. Canonical contract schema for gate actions and outcomes.
3. Cross-repo inventory of current gate entry points.

Hard-close evidence:
1. Contract doc + schema checked in.
2. Inventory artifact with repo-by-repo mapping.

### FC-2: Local/CI/Release Parity Lock

Deliver:
1. Script parity matrix for each scoped repo.
2. CI jobs delegate to canonical scripts where possible.
3. Drift detection report for mismatched commands.

Hard-close evidence:
1. Parity check artifacts for all scoped repos.
2. At least one intentional mismatch test detected and reported.

### FC-3: Failure-Class Hardening

Deliver:
1. Harden known failure classes from gotchas (DNS, lock drift, protoc, stale binary, JS parse).
2. Add deterministic retries and explicit diagnostics for transient failures.
3. Add proactive preflight checks for common missing dependencies.

Hard-close evidence:
1. Simulated failure runs with expected remediation output.
2. Updated gotcha references in runbook.

### FC-4: Unified Preflight Decision Graph

Deliver:
1. One deterministic graph: `Policy -> Hook -> Drift -> Gate -> Push`.
2. Single machine-readable output model (status, failure code, remediation, evidence pointer).
3. Dashboard and tab actions consume same graph output.

Hard-close evidence:
1. Graph acceptance tests pass for pass/fail branches.
2. No duplicate decision logic paths for preflight actions.

### FC-5: Pilot CI Contract Layer

Deliver:
1. Typed commands: run/replay/repair/readiness/report with schema validation.
2. Scope and policy checks before dispatch.
3. Contract previews always available before execute.

Hard-close evidence:
1. CLI/API/UI parity tests pass for contract commands.
2. Evidence includes resolved command list and payload digest.

### FC-6: Provenance and Replay

Deliver:
1. Provenance record includes input, resolved ops, env summary, output, artifacts.
2. Replay bundle generation for failed and successful runs.
3. One-click/one-command replay entry from Pilot.

Hard-close evidence:
1. Replay reproduces a known failure deterministically.
2. Replay metadata validated in tests.

### FC-7: Federated Orchestration

Deliver:
1. Grouped execution modes (`core`, `ui`, `infra`, custom AGOrg sets).
2. Dependency-aware ordering with explicit skip/fail semantics.
3. Consolidated federation status board in Pilot.

Hard-close evidence:
1. Multi-repo run with staged order and verified outcomes.
2. Failure in one repo does not corrupt unrelated repo evidence.

### FC-8: Security + Policy Hardening

Deliver:
1. Protected-branch enforcement with typed confirmations where destructive.
2. Strict command allowlist and mutation scope controls.
3. Secrets-safe logging and policy exception controls.

Hard-close evidence:
1. Security tests for blocked disallowed commands pass.
2. No secrets leak in standard evidence logs.

### FC-9: Release Train Hard-Close

Deliver:
1. Alpha/beta/stable channel policy integrated with federated gates.
2. Migration + rollback playbooks exercised.
3. Compatibility matrix + SLO/error-budget + incident workflow active.

Hard-close evidence:
1. Full dry-run and one real alpha release complete with evidence bundle.
2. Post-release review artifact with residual risk list.

## 9) Definition of Done (Program Hard-Close)

Program is hard-closed only when all conditions hold:

1. Every FC wave has test evidence and artifact paths.
2. Local/CI/release parity is verified across all scoped repos.
3. Pilot can orchestrate federated CI workflows with preview-first safety.
4. Failure outputs are deterministic, actionable, and replayable.
5. AGOrg-scoped policy enforcement is real, not inferred.
6. Release process is repeatable with signed evidence bundles.
7. No known placeholders, stubs, or fake-success shims in production paths.

## 10) Anti-Drift Operating Rules (For Implementing AI)

1. Do not optimize for "green screenshot"; optimize for deterministic behavior.
2. Do not declare done from endpoint existence alone.
3. Do not bypass AGOrg scope checks in mutating operations.
4. Do not merge without docs + gotcha updates when behavior changes.
5. Do not duplicate logic across Dashboard and specialist tabs.
6. Always include failure-path tests, not just happy-path tests.

## 11) Next-Level Suggestions (Beyond Baseline)

1. Add policy simulation mode:
   - run "what-if" governance outcomes before applying policy changes.
2. Add evidence integrity verification command:
   - verify signed bundle chain before release promotion.
3. Add conflict radar for branch + dependency orchestration:
   - pre-detect merge and policy conflicts before execution.
4. Add guided remediation mode:
   - transform failure codes into step-by-step operator actions in UI.
5. Add federation health score:
   - weighted score for drift, reliability, and policy compliance across repos.
6. Add scheduled parity sweeps:
   - nightly federation check with trend reporting and auto-ticket generation.

## 12) Legacy References

1. `project_plan_ci_cd_control_plane.md`
2. `project_plan_pilot_ci_orchestrator.md`
