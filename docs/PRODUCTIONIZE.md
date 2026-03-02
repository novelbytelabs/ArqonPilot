# ArqonPilot Productionization Plan

**Last updated**: 2026-03-02

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

1. Core product waves and AGOrg rollout are complete through Wave 17.
2. Remaining execution focus is hardening/closure quality:
     - Wave 18 (technical debt burn-down)
     - Branch Control BC-7 and BC-8 hard-close.
3. Release process/tooling is complete and operational (alpha cadence execution remains ongoing).
4. AGOrg governance loop (report -> dry-run -> apply -> verify) is live in UI/API/CLI.

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
     - Wave 18 technical debt closure
     - Branch Control BC-7/BC-8 hard-close

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

## Remaining Work (No Hidden Gaps)

1. Close Wave 18 technical debt (hard-close):
     - remove remaining placeholder/test debt in production paths
     - complete shim/temporary-component burn-down where feasible; any retained shim must be documented with owner + removal criteria
     - tighten lint/deprecation debt without violating frozen policy (`1.82.0` core, `1.88.0` packaging, protobuf `4.25.8`)
     - exit gate: update `docs/tech-debt.md` with explicit closed/open items and evidence links.
2. Close Branch Control BC-7 (CI/push parity):
     - wire deterministic Branch -> Dependencies -> Push-safe readiness handoff
     - ensure local/CI parity checks are surfaced in Branch workflow status and artifacts
     - exit gate: parity flow verified in acceptance matrix evidence.
3. Close Branch Control BC-8 (final acceptance hard-close):
     - execute acceptance matrix for single-repo, multi-repo, staged apply, destructive confirmations, and failure isolation
     - verify timeline/artifact drill-down for each operation class
     - exit gate: runbook + release-log updated with final evidence bundle and no open high-severity branch-control defects.
3. Continue production release cadence:
     - execute release playbook
     - maintain complete evidence bundles for each alpha.

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
