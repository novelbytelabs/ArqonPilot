# P9 Hard-Close Checklist

Purpose:
- Provide a strict, evidence-first checklist to hard-close `P9: Release Train Hardening`.
- Eliminate tribal steps and prevent "done by claim" outcomes.

Scope:
- Repo: `/home/irbsurfer/Projects/arqon/ArqonPilot`
- Wave: `P9`
- Authoritative context: `docs/PRODUCTIONIZE.md`

Frozen policy (must not change):
- Core Rust lane: `1.82.0`
- Packaging Rust lane: `1.88.0`
- Protobuf: `4.25.8`

## 1. Preconditions

Required before any edits:
1. Confirm repo root:
   - `git rev-parse --show-toplevel`
2. Confirm no path drift to sibling repos (`Arqon`, `ArqonBus`, etc.) for this wave.
3. Re-read:
   - `docs/PRODUCTIONIZE.md`
   - `docs/release-playbook.md`
   - `docs/release-log.md`
   - `docs/operator-runbook.md`
   - `docs/gotcha-registry.md`
4. Confirm active branch/worktree state:
   - `git status -sb`

Stop gate:
- If any P9 work is being attempted outside `ArqonPilot`, stop and report.

## 2. Deliverables Checklist

All items required for hard-close:
1. Channel policy + gating criteria are explicit and executable (`alpha`, `beta`, `stable`).
2. Migration and rollback playbooks are deterministic and operator-usable.
3. Compatibility matrix reflects real frozen/tooling constraints and supported environments.
4. Incident/SLO linkage is documented for release decisions.
5. Release procedure is runnable end-to-end with artifact evidence.

## 3. Verification Matrix (Mandatory)

Run and capture results:
1. `cargo check -p pilot --locked`
2. `./scripts/ui_smoke_check.sh`
3. `./scripts/prepush_gate.sh`
4. Release dry-run procedure from `docs/release-playbook.md`
5. Bundle/evidence verification commands from the release flow

Required evidence:
- Command outputs (pass/fail)
- Artifact paths under `~/.pilot/reports/`
- Any generated release docs/log entries updated in-tree

## 4. Evidence Requirements

For hard-close, all must exist:
1. Dry-run release evidence bundle path(s)
2. One alpha release execution record by documented steps
3. Rollback drill evidence
4. Compatibility matrix smoke evidence
5. `docs/release-log.md` updated with timestamps, outcome, and artifact references

Evidence format (recommended block per run):
- `timestamp_utc`
- `command`
- `result`
- `artifact_path`
- `notes/remediation`

## 5. Guardrails and Gotchas

Critical gotchas to enforce:
1. `G-017`: No completion claims without observable evidence.
2. `G-015`: JS syntax regressions can pass Rust checks; run `node -c` when UI changes.
3. `G-010`: Stale binaries can mask true behavior; verify command pathing/version source.
4. `G-043`: Evidence writes must surface failures and not silently pass.

Operational rules:
1. No placeholders/stubs/fake success paths.
2. No mutation of frozen policy/toolchain constraints.
3. No hidden/manual side-channel steps; all release actions must be documented and reproducible.

## 6. Hard-Close Definition of Done

`P9` is hard-closed only when:
1. All verification matrix commands pass.
2. All release-train deliverables are implemented and documented.
3. Evidence artifacts exist and are referenced from `docs/release-log.md`.
4. `docs/PRODUCTIONIZE.md` P9 section is updated from provisional -> hard-closed with observed evidence.
5. No open stop-gate exceptions remain.

## 7. Required Final Packet

The implementing AI/operator must return:
1. Changed files list
2. Exact commands run
3. Pass/fail outcomes
4. Artifact paths
5. Residual risks (if any)
6. Final status:
   - `P9 HARD-CLOSED`
   - or `P9 NOT HARD-CLOSED` with explicit blockers
