# Project Plan (Legacy Subplan): Pilot as CI/CD Orchestrator Interface

> Merged into master program: `project_plan_pilot_federated_ci_program.md`

## Objective
Enable conversational CI/CD control through Pilot without sacrificing determinism, safety, or auditability.

## Design Principle
Pilot is the interface. Gate scripts are the law.

## Non-Negotiable Constraints
- No free-form execution for critical lanes.
- Pilot actions map to versioned command contracts.
- Mutations require explicit dry-run evidence before apply.
- Every action emits auditable provenance.

## Command Contract (Initial)
- `pilot.ci.gate.run`
- `pilot.ci.gate.replay`
- `pilot.ci.lock.repair`
- `pilot.ci.release.readiness`
- `pilot.ci.policy.report`

Each contract must include:
- `repo`
- `branch`
- `lane`
- `dry_run`
- `schema_version`

## Implementation Phases

### Phase 1: Contract and Backend Binding
- Add a typed command registry for CI/CD actions.
- Bind each command to deterministic scripts in target repos.
- Validate payload schema and required scopes before execution.

### Phase 2: Dry-Run and Safety Rails
- Force dry-run first for mutation-capable commands.
- Add explicit apply confirmation requirements in command layer.
- Enforce branch/risk policies before dispatch.

### Phase 3: Evidence and Replay
- Store command input, resolved command, outputs, and artifacts.
- Generate replay bundles for failed operations.
- Expose replay invocation through `pilot.ci.gate.replay`.

### Phase 4: Multi-Repo Orchestration
- Add grouped execution (`core`, `ui`, `infra`) with policy-aware ordering.
- Add cross-repo dependency checks before dispatch.
- Return consolidated federation status with per-repo lane outcomes.

## Acceptance Criteria
- A user can request a federation gate run via Pilot and receive deterministic outcomes.
- Any failed CI action can be replayed from Pilot with equivalent parameters.
- No protected mutation command executes without dry-run proof and policy pass.

## Operational Metrics
- CI failure escape rate (failures first discovered in GitHub Actions).
- Mean time to reproduce a CI failure locally.
- Mean time to resolve lock/toolchain drift incidents.
- Percentage of Pilot CI commands executed in dry-run-first mode.
