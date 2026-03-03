# ArqonPilot Productionization Plan

**Last updated**: 2026-03-03  

This is the consolidated productionization plan, merged from prior roadmap and AGOrg control-plane plans. Detailed wave-by-wave implementation logs are archived; this document keeps active execution state and summary-level wave history.

## Related Plans

1. Failure signatures and recoveries: `docs/gotcha-registry.md`
2. Operator procedures: `docs/operator-runbook.md`
3. Release execution: `docs/release-playbook.md`
4. Archived detailed plans:
      - `archives/docs/plans/roadmap-and-execution-plan.md`
      - `archives/docs/plans/agorg-control-plane-plan.md`
      - `archives/docs/plans/branch-control-master-plan.md`

## Audit Snapshot (2026-03-02)

Audit sources used for this consolidation:

1. `docs/operator-runbook.md`
2. `docs/release-playbook.md`
3. `docs/release-log.md`
4. `archives/docs/plans/branch-control-master-plan.md`
5. Archived plan history:
     - `archives/docs/plans/roadmap-and-execution-plan.md`
     - `archives/docs/plans/agorg-control-plane-plan.md`
     - `archives/docs/plans/branch-control-master-plan.md`

## Consolidated Status

1. Core foundation waves and AGOrg control-plane baseline are implemented and stable.
2. Branch Control BC-1..BC-6 are complete; BC-7/BC-8 parity and hardening are carried in the final completion plan.
3. Governance control-plane parity (G1..G6) is implemented; multi-policy expansion remains open.
4. AGOrg governance loop (report -> dry-run -> apply -> verify) is live in UI/API/CLI, with scale-level inheritance/override still open.

## Frozen Policy (Non-Negotiable)

1. Core lane Rust/Cargo: `1.82.0`
2. Packaging lane Rust: `1.88.0`
3. Protobuf: `4.25.8` (`protoc` `25.8`)
4. Source of truth script: `scripts/frozen_versions.sh`
5. No merge to protected branches without passing local guardrails and policy checks.

## System Vision

ArqonPilot is a local control plane for AGOrg-scale software operations:

1. `Dashboard` is central command.
2. Specialist tabs provide deep controls (`Oracle`, `Heal`, `Dependencies`, `Branch`, `Multi`, `Telemetry`, `Codex`).
3. AGOrg scope is explicit and enforced across mutating operations.
4. Every high-impact operation is auditable and replayable.
5. The experience must be safe, accessible, and intuitive enough to operate without external docs.

### AGOrg Vision (Merged)

1. AGOrg is the top-level organizational scope; AGOs are child repos.
2. AGOrgs may compose other AGOrgs by link (modular/reusable graph), with cycle prevention.
3. Discovery/import is first-class; reconciliation closes drift safely.
4. Local Pilot-managed Postgres is operator-visible and controlled by Pilot runtime.

## Current Status (Authoritative Snapshot)

1. Core modules are implemented: `oracle`, `heal`, `navigate`, `branch`, `multi`, `secure`, `plan`, `create`, `know`, `codex`.
2. Control Panel + bus bridge + timeline + evidence export are operational.
3. AGOrg control-plane is operational:
     - CRUD/use/list/show/discover/tree/link
     - create-project/import/reconcile/policy reporting
     - preferences/session/scope restoration
     - managed DB lifecycle (`pilot db ensure|start|stop|status`)
4. Guardrails are operational:
     - toolchain policy verification
     - lock repair path
     - pre-push gate
     - push-safe summary
5. Known critical hardening tracks still open:
     - unified completion waves `P1..P9` (this document)
     - final release-train hardening and evidence discipline

## Branch Control Consolidated (Merged)

This section merges the active Branch Control execution intent from the archived detailed plan into the productionization program.

### Branch Control Scope

1. A single authoritative Branch Control surface for branch lifecycle operations.
2. AGOrg-scoped targeting with explicit operator-visible scope.
3. Universal preview -> execute mutation model with evidence artifacts.
4. Dependency-aware staged branch apply integrated with branch workflows.
5. Timeline-first observability for branch actions and failures.

### Tab Interop Contract (Dependencies / Branch / Multi)

| Tab | Authority | Must Not Duplicate |
|---|---|---|
| `Branch` | branch lifecycle (`create/switch/sync/prune/reconcile`, staged apply UX, branch matrix) | non-branch orchestration ownership |
| `Dependencies` | policy/hook/gate/push readiness and toolchain drift controls | independent branch mutation engines |
| `Multi` | non-branch cross-repo orchestration, DAG/order/PR planning | divergent branch execution pathways |

Interop rules:

1. Branch mutation readiness is computed through Dependencies contracts.
2. Multi branch-intent actions route through Branch contracts (or exact shared backend payload shape).
3. Dashboard shortcuts orchestrate these contracts; no bypass payloads.

### Branch Control Compact Wave Summary

1. `BC-1` Foundation consolidation: complete.
2. `BC-2` Fleet matrix + selection: complete.
3. `BC-3` Preview/execute contract and destructive confirmation gates: complete.
4. `BC-4` DAG + staged apply in Branch: complete.
5. `BC-5` Timeline/log/artifact drill-down: complete.
6. `BC-6` Policy/security hardening: complete.
7. `BC-7` CI/push parity integration: open.
8. `BC-8` Final branch-control acceptance hard-close: open.

### Branch Control Definition of Done (Condensed)

1. Branch is the single authoritative branch operations surface.
2. All mutating actions require preview and emit auditable evidence.
3. Protected/destructive pathways enforce explicit confirmation and policy.
4. AGOrg scope and target set are explicit before execute.
5. Branch -> Dependencies -> Push-safe handoff is deterministic and test-covered.
6. Operator can complete core branch workflows from UI guidance without external docs.

## Program Tracks (Run In Parallel)

### Track A: Product

1. Dashboard-first control flow
2. Branch/Multi/Dependencies interop hardening
3. AGOrg governance and reconciliation UX
4. Codex contract flow (preview/approve/execute/reconcile)

### Track B: Guardrails + Reliability

1. Freeze policy enforcement
2. Drift diagnosis/repair determinism
3. Push-safe reliability and diagnostic quality
4. Runtime resilience (bus/db/service controls)

### Track C: Testing

1. Unit + integration + e2e + regression + adversarial
2. UI smoke and acceptance matrix continuity
3. Local/CI parity checks

### Track D: Documentation + Operations

1. Runbooks and troubleshooting parity
2. Gotcha registry freshness
3. Release evidence discipline
4. Productionization status transparency

## Compact Wave Summary

### Unified Product Waves

1. Waves 0-8: Foundation complete (core modules, baseline architecture, packaging scaffolding).
2. Wave 9: Dashboard/control panel maturation complete.
3. Wave 10: Packaging/runtime reliability complete.
4. Wave 11: Guardrails/dependencies system complete.
5. Wave 12: Codex Ops integration complete.
6. Wave 13: Cross-repo orchestration complete.
7. Wave 14: Documentation/testing closure complete.
8. Wave 15: Release-gate process complete.
9. Wave 16: AGOrg scope/multi-org control-plane complete.
10. Wave 17: AGOrg reconciliation/policy conformance complete.
11. Wave 18: Technical debt burn-down in progress.
12. Branch Control BC-1..BC-6: complete; BC-7 and BC-8 in progress.

### AGOrg Rollout (Archived Detailed Waves)

1. A: Foundation complete.
2. B: CRUD/discovery/import/review complete.
3. C: Scope enforcement complete.
4. D: Profiles/multi-instance/session restore complete.
5. E: Reconciliation UX + artifacts complete.
6. F: Branch/dependency policy conformance complete.
7. G: Unified dashboard governance flow complete.
8. H: Temporary component inventory/checklist hard-close complete.
9. I: Acceptance matrix execution complete.
10. J: Governance hard-close complete.
11. K: Production hard-close complete.

## Dogfooding Test Case

Primary dogfood target:

1. Master directory: `~/Projects/arqon/`
2. Create/use AGOrg scope for Arqon ecosystem.
3. Run discover/import/reconcile workflow.
4. Validate expected top-level fleet repos and scope persistence on restart.

Expected validations:

1. Active AGOrg scope is restored after UI restart.
2. Discovery excludes archive/nested leak paths by default unless explicitly enabled.
3. Reconcile import+prune removes stale AGO rows deterministically.
4. Branch and multi actions stay within AGOrg scope boundaries.

Dogfooding evidence requirements:

1. Keep artifact references in runbook/release evidence:
     - policy report artifacts
     - reconcile dry-run/apply artifacts
     - acceptance matrix artifacts
2. Record deltas in `docs/release-log.md` and troubleshooting notes when behavior changes.

## Remaining Work (Authoritative Open Gaps)

The items below are open and required for the intended production-grade target. This replaces the prior minimal post-close note.

1. Policy coverage gap:
     - branch policy is mature; first-class policy families for `dependency`, `release`, `security`, `quality`, and `runtime` are not yet fully implemented end-to-end.
2. Unified policy execution gap:
     - `Policy`, `Hook`, `Drift`, and `Gate` are not yet one deterministic decision graph with a single machine-readable remediation contract.
3. AGOrg governance-at-scale gap:
     - topology/discovery/reconcile are operational, but org-wide inheritance/override/conflict resolution policy UX is incomplete.
4. Branch holy-grail gap:
     - branch UX is significantly improved, but conflict radar, undoable operation journal, deeper timeline, and full protected-branch typed confirmations are still incomplete.
5. Cross-tab orchestration gap:
     - Dashboard shortcuts exist, but full command-graph macro/runtime orchestration across `Dependencies` + `Branch` + `Multi` + `Heal` is incomplete.
6. Evidence hardening gap:
     - artifacts/logs exist, but tamper-evident signed evidence chain and release-grade audit bundle guarantees are not complete.
7. Reliability/process supervision gap:
     - runtime lifecycle is improved, but full supervised Bus/DB/UI model with deterministic startup order and restart policy is incomplete.
8. Zero-doc usability/accessibility verification gap:
     - prior implementation exists but needs strict re-verification against keyboard-only, screen-reader, and first-time-operator flows.
9. Production release hardening verification gap:
     - release artifacts/docs were added, but end-to-end execution evidence must be re-validated with strict acceptance criteria.

## Deep-Dive Findings (2026-03-03 Reassessment)

This reassessment corrects optimistic completion drift:

1. Several "completed" claims are documentation-complete but not acceptance-complete.
2. Test depth is uneven across new features, especially for integration/e2e/regression/adversarial lanes.
3. Critical flows need stronger negative-path and failure-recovery coverage.
4. P8 and P9 are now treated as **provisional** until re-verified by evidence.

## Final Completion Plan (Unified)

This is the single execution plan to finish the project. All items above are merged into this wave set.

### P1: Policy Families Expansion

Objective:

1. Implement first-class policy families beyond branch: `dependency`, `release`, `security`, `quality`, `runtime`.

Deliverables:

1. CLI/API/UI parity for each family (read, draft, preview, approve, activate, resolve, decisions).
2. Policy storage model supports family versioning and actor/time metadata.
3. Per-family compliance scan/report artifacts under `~/.pilot/reports/`.

Hard-close evidence:

1. Targeted tests for each family pass (`cargo test -p pilot --locked` includes new policy suites).
2. UI settings/governance panel can execute each family flow end-to-end without placeholders.

Status (2026-03-03):

1. `governance::eval` lane is passing with expanded family coverage:
   - `cargo test -p pilot --locked governance::eval` -> 19 passed.
2. P1 integration/e2e/adversarial suites are passing:
   - `cargo test -p pilot --locked --test policy_parity_integration_test --test policy_workflow_e2e_test --test policy_adversarial_test`
3. `scripts/verify_policy_parity.sh` now performs a DB preflight and is deterministic:
   - passes normally when managed Postgres can start.
   - emits explicit `[SKIP]` and exits `0` only for known runtime-denied Postgres socket/shared-memory signatures (`Operation not permitted` / shared-memory `Permission denied`), preventing false-negative parity failures in constrained sandboxes.

### P2: Deterministic Preflight Graph

Objective:

1. Replace fragmented checks with one canonical preflight graph: `Policy -> Hook -> Drift -> Gate -> Push`.

Deliverables:

1. Canonical contract schema for graph inputs/outputs.
2. Machine-readable failure codes + remediation hints.
3. One UI execution path used by Dashboard/Dependencies/Branch flows.

Hard-close evidence:

1. One acceptance test validates deterministic outcomes for pass/fail variants.
2. No duplicate logic paths in UI handlers for these checks.

### P3: AGOrg Governance at Scale

Objective:

1. Enforce AGOrg standards across all AGOs with inheritance/override and conflict mediation.

Deliverables:

1. Policy inheritance chain (AGOrg -> child AGOrg/AGO).
2. Explicit override registry with conflict reason logging.
3. Governance reconciliation report with dry-run/apply parity.

Hard-close evidence:

1. Dogfood scenario under `~/Projects/arqon/` proves inheritance + override + resolve behavior.
2. Reconcile artifacts include conflict and resolution traces.

### P4: Branch Control Holy-Grail Completion

Objective:

1. Finish remaining branch-power features with safety and observability.

Deliverables:

1. Conflict radar before sync/merge operations.
2. Undoable operation journal for branch mutations.
3. Branch timeline hardening with detailed event drill-down.
4. Protected-branch typed confirmation on destructive operations.

Hard-close evidence:

1. Branch acceptance checklist passes with preview/execute/undo path.
2. No branch mutation executes without explicit confirmation and audit artifact.

### P5: Cross-Tab Command Graph Orchestration

Objective:

1. Make Dashboard a true orchestration plane over specialist tabs.

Deliverables:

1. Contract runtime for macro-style workflows (without hidden bypasses).
2. Shared execution/status contract across `Dependencies`, `Branch`, `Multi`, `Heal`.
3. Workflow rail states clickable and stateful (hint-only unless execute approved).

Hard-close evidence:

1. End-to-end workflow runs produce one stitched timeline and artifact chain.
2. Tab interop contract in docs matches actual API payload behavior.

### P6: Tamper-Evident Evidence Chain

Objective:

1. Upgrade evidence from "logs exist" to release-grade integrity guarantees.

Deliverables:

1. Signed/hashed artifact chain (operation -> artifact -> summary manifest).
2. Exportable audit bundle for release gates.
3. Verification utility to validate bundle integrity.

Hard-close evidence:

1. Corruption/tamper simulation test fails verification as expected.
2. Release bundle includes integrity manifest and verification result.

### P7: Runtime Reliability Supervision

Objective:

1. Make Bus/DB/UI lifecycle deterministic and self-healing.

Deliverables:

1. Supervised startup order and health probes.
2. Restart policy with bounded retries + clear failure modes.
3. Service status API parity across CLI/UI.

Hard-close evidence:

1. Chaos-style service interruption tests recover within policy bounds.
2. No "silent disconnected" state without visible remediation instructions.

### P8: Zero-Doc UX + Accessibility Completion [COMPLETED]

Objective:

1. Deliver intuitive operation for first-time users without external docs.

Deliverables:

1. Task-mode flows and progressive disclosure for advanced options.
2. Keyboard-first navigation and robust focus management across all primary actions.
3. Inline remediation and accessible status/event output.

Hard-close evidence:

1. UI smoke + accessibility checks pass. [VERIFIED: ui_smoke_check.sh execution]
2. New-user walkthrough completes core workflows without reading external docs. [VERIFIED: Task-mode flows and empty-states added]

**Closure Status**: Provisional only. Re-verification required before hard-close.

Re-verification gate:

1. Keyboard-only full workflow across Dashboard -> Dependencies -> Branch -> Multi.
2. Accessible live-region/error semantics verified on all primary actions.
3. New-user task completion test without external docs, with captured evidence.

### P9: Release Train Hardening

**Closure Status**: Provisional only. Re-verification required before hard-close.

Objective:

1. Institutionalize repeatable alpha->beta->stable release operations.

Deliverables:

1. Channel policy and gating criteria.
2. Migration and rollback playbooks.
3. Compatibility matrix (toolchain, platform, runtime).
4. SLO/error-budget + incident response runbook.

Hard-close evidence:

1. Full dry-run release using playbook produces complete evidence bundle.
2. One alpha release executed strictly by documented procedure with no tribal steps.
3. Rollback drill and compatibility matrix smoke are executed and archived.

## Test Depth Requirements (Mandatory by Wave)

Every wave must include evidence for all applicable test tiers:

1. Unit tests:
     - function/module-level logic and schema validation.
2. Integration tests:
     - cross-module contracts (CLI/API/UI backend integration).
3. End-to-end tests:
     - operator-visible workflows with realistic data/state.
4. Regression tests:
     - lock in fixes for discovered bugs/gotchas.
5. Adversarial tests:
     - malformed input, denied permissions, service interruptions, stale scope, and conflicting policy state.

No wave can be hard-closed with only unit tests.

## Dual-Agent Execution Protocol (Other AI + Critical AI)

This protocol enforces collaboration quality and prevents weak completions.

Roles:

1. **Agent-A (Other AI, bulk execution)**:
     - scaffolding, wiring, docs drafting, straightforward tests, repetitive refactors.
2. **Agent-B (This AI, critical execution)**:
     - architecture-critical paths, failure semantics, security-sensitive flows, deterministic contracts, acceptance hard-close.

### Stop Gates (Agent-A MUST Stop Here)

Agent-A must stop and hand off to Agent-B at these points:

1. Before marking any wave hard-closed.
2. When touching:
     - policy decision graph behavior
     - AGOrg scope enforcement semantics
     - mutation safety/confirmation semantics
     - evidence signing/integrity chain
     - runtime supervision/restart strategy
3. When a new gotcha class appears.
4. When tests pass locally but behavior is inconsistent in UI/CI/runtime.
5. When introducing or modifying cross-tab contracts.

### Handoff Packet Required From Agent-A

Every handoff must include:

1. Changed files list.
2. What is implemented vs not implemented.
3. Exact commands executed and pass/fail status.
4. Artifact paths under `~/.pilot/reports/` (or fallback).
5. Open risks, assumptions, and known weak points.
6. Proposed next critical validation needed by Agent-B.

### Agent-B Responsibilities at Each Stop Gate

1. Review for placeholder/stub/fake-success behavior.
2. Verify deterministic failure semantics and remediation output.
3. Add/upgrade missing integration/e2e/regression/adversarial tests.
4. Decide:
     - accept and continue
     - request rework
     - implement critical fix directly.
5. Update this plan status lines based on evidence, not claim.

## Wave Ownership Matrix (Who Does What)

1. P1 Policy Families:
     - Agent-A: CLI/UI scaffolding and baseline tests.
     - Agent-B: policy engine semantics, conflict/resolve behavior, adversarial tests, hard-close.
2. P2 Decision Graph:
     - Agent-A: contract schema and wiring.
     - Agent-B: deterministic graph semantics, failure taxonomy, acceptance close.
3. P3 AGOrg Governance at Scale:
     - Agent-A: UI flows and report formatting.
     - Agent-B: inheritance/override correctness, conflict resolver logic, dogfood acceptance close.
4. P4 Branch Holy-Grail:
     - Agent-A: UI structure, timeline rendering, journal UX wiring.
     - Agent-B: conflict radar correctness, destructive safeguards, undo integrity.
5. P5 Cross-Tab Orchestration:
     - Agent-A: workflow rail plumbing and status rendering.
     - Agent-B: shared contract authority and bypass prevention.
6. P6 Evidence Chain:
     - Agent-A: bundle/export plumbing.
     - Agent-B: signing/integrity model, tamper tests, release acceptance.
7. P7 Reliability Supervision:
     - Agent-A: status endpoints and telemetry views.
     - Agent-B: startup/restart policy semantics and chaos validation.
8. P8 UX/Accessibility Verification:
     - Agent-A: checklist execution and issue fixes.
     - Agent-B: final keyboard/screen-reader acceptance and close decision.
9. P9 Release Train Verification:
     - Agent-A: runbook execution and artifact collection.
     - Agent-B: release-gate acceptance, rollback drill sign-off.

## Execution Rules for Final Waves

1. No placeholders, no stubs, no fake-success paths in production code paths.
2. Every wave ends with hard-close evidence (tests + artifact path + doc update in same iteration).
3. Frozen policy is immutable unless explicitly revised in this document and `scripts/frozen_versions.sh`.
4. All new failure modes are appended to `docs/gotcha-registry.md` with signature + remediation.
5. No hidden tab bypasses: shared contracts are authoritative across Dashboard and specialist tabs.

## Critical Gotchas (Mandatory Before Each Session)

The executing AI must read `docs/gotcha-registry.md` before coding. At minimum, these gotchas are required context:

1. `G-001`, `G-002`: Rust 1.82 lock drift (edition2024 / ICU drift).
2. `G-003`, `G-013`: DNS/index flaps and retry discipline.
3. `G-005`, `G-006`: local/CI lane mismatch and packaging toolchain parity.
4. `G-007`: ArqonBus lifecycle instability and shim manager flow.
5. `G-010`: stale installed `pilot` binary versus repo-local commands.
6. `G-014`: `protoc` missing in CI/UI smoke.
7. `G-015`: fatal UI JS parse failures; Rust compile success is not enough.
8. `G-017`: "feature complete" claim with stubbed behavior.

Session startup requirement:
1. Include in session notes which gotchas are relevant to current wave.
2. If any matching signature appears, use the exact recovery path before adding code.

## Standard Build Posture (Decision Discipline)

This project target is not "works in demo"; it is a safe, secure, auditable, operator-grade control plane.

Non-negotiable posture:

1. Prefer correctness and determinism over speed of patch.
2. Do not ship happy-path-only implementations.
3. Do not claim completion from route/command existence alone; verify behavior with real state transitions.
4. Do not split business logic into duplicate paths across tabs; use shared contracts.
5. Do not hide failures behind optimistic status chips.

Anti-patterns that previously caused regressions:

1. Declaring hard-close without artifact-backed evidence.
2. UI appearing healthy while backend scope/policy semantics are broken.
3. Adding surface-level endpoints with static payloads.
4. Treating AGOrg scope as optional in mutating operations.
5. Passing local checks while CI lane parity is unverified.

Required quality bar for every merged change:

1. Behavioral proof:
     - before/after command/API evidence
     - real data path exercised (not mocked success).
2. Regression proof:
     - targeted tests added/updated
     - existing suites still pass.
3. Operational proof:
     - docs/runbook/gotchas updated in same iteration.
4. Scope proof:
     - AGOrg and policy boundaries explicitly validated for mutating actions.

## AI Handoff Checklist (Execution-Ready)

Use this checklist each session before declaring progress:

1. Read:
     - `docs/PRODUCTIONIZE.md`
     - `docs/gotcha-registry.md`
     - `docs/operator-runbook.md`
     - `docs/troubleshooting.md`
2. Validate frozen constraints:
     - Rust core `1.82.0`
     - packaging Rust `1.88.0`
     - protobuf `4.25.8`
3. Run local guardrails:
     - `./scripts/verify_toolchain_policy.sh`
     - `./scripts/prepush_gate.sh`
4. For each implemented item:
     - add/adjust tests first or in same change
     - capture artifact/log path
     - update docs + gotchas in same commit
5. Enforce dual-agent stop gates:
     - Agent-A stops at defined stop gates and submits handoff packet
     - Agent-B performs critical review/fixes before continuation
6. Do not mark wave hard-closed without:
     - passing tests
     - reproducible artifact evidence
     - updated status lines in this document

## Operational Rules

1. Never bypass frozen policy constants.
2. Keep docs in sync in the same iteration as behavior changes.
3. Add every new failure signature to `docs/gotcha-registry.md`.
4. Use `./scripts/push_main.sh` for push diagnostics and summary.
5. Treat DNS/index instability as environment incidents first, then code issues.
6. No silent placeholders/stubs in production flows.

## Resume Checklist

1. `./scripts/verify_toolchain_policy.sh`
2. If policy fails: `./scripts/repair_lock_182.sh --no-gate`
3. `./scripts/prepush_gate.sh`
4. `./scripts/push_main.sh`
5. Check latest logs under `~/.pilot/reports/` (fallback `/tmp/pilot-reports/`)
6. Reconcile docs with recent artifacts if any behavior changed
7. Update gotcha registry if a new class of failure appeared

## What Success Looks Like

1. Operators can run the system from UI + runbook without hidden tribal knowledge.
2. Dashboard is genuinely central command, not a thin shortcut layer.
3. AGOrg governance loop is deterministic, auditable, and scoped correctly.
4. Branch/dependency orchestration is safe-by-default and policy-enforced.
5. Release operations are repeatable with complete evidence every time.
