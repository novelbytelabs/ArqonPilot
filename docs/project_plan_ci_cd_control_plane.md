# Project Plan (Legacy Subplan): Federated CI/CD Control Plane

> Merged into master program: `project_plan_pilot_federated_ci_program.md`

## Objective
Eliminate avoidable GitHub Actions failures across Arqon repos by enforcing one deterministic, replayable, policy-bound CI/CD routine.

## Scope
- `ArqonPilot`
- `ArqonBus`
- `ArqonLattice`
- `ArqonStudio`
- `ArqonHPO`

## Success Criteria
- Every protected branch uses required checks derived from canonical repo gate scripts.
- Local pre-push gate and CI pipeline execute the same command family.
- Failure classes observed in recent runs (lock drift, brittle smoke checks, path/auth mismatch, formatting drift) are blocked pre-merge.
- Each failure produces a minimal deterministic replay command.

## Workstreams

### WS1: Canonical Gate Standardization
- Define required gate interface per repo: `policy`, `fmt`, `lint`, `check`, `test`, `packaging`, `smoke`, `release-readiness`.
- Ensure scripts are versioned and executable in local + CI contexts.
- Remove duplicated workflow-only logic where possible; delegate to scripts.

### WS2: Policy and Parity Enforcement
- Enforce lockfile parity rules and compatibility checks.
- Require branch protection checks tied to canonical workflows only.
- Add drift reports for lock/toolchain/config mismatches.

### WS3: Resilience Hardening
- Replace brittle UI/text assertions with stable contract checks.
- Add preflight path checks for Docker contexts and working directories.
- Resolve cross-repo clone reliability with authenticated and pinned fetch strategy.

### WS4: Observability and Replay
- Standardize artifact/log naming for all critical lanes.
- Emit reproducible command manifests per failed job.
- Add one-click or one-command replay guidance in failure output.

## Milestones

### M1: Pilot Reference (Week 1)
- Canonical gate fully wired into pre-push + CI + release readiness.
- Packaging and UI smoke lanes deterministic and policy-bound.

### M2: Bus and Studio Stabilization (Week 2)
- Bus: format/test/docker preflight parity complete.
- Studio: node/cache path checks stabilized.

### M3: Lattice and HPO Hardening (Week 3)
- Lattice: cross-repo dependency auth + pin strategy complete.
- HPO: dependabot lanes constrained by pre-merge gate and compatibility policy.

### M4: Federation Guardrail Lock (Week 4)
- Required checks + merge queue + no direct push on protected branches.
- Nightly parity sweep across repositories.

## Risks and Controls
- External outages (GitHub/registry): mitigate with retries/caching; accept residual risk.
- Over-gating latency: use tiered suites (`fast gate` vs `full gate`) with same policy core.
- Drift reintroduction: add periodic parity audits and gate contract tests.

## Deliverables
- Canonical gate scripts and workflow bindings in each scoped repo.
- Branch protection policy specification.
- Replay and evidence specification for failures.
- Rollout report with pre/post failure-rate comparison.
