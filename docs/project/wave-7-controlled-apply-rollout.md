# Wave 7 Controlled Apply Rollout

Date: 2026-02-25

## Objective

Validate safe apply-mode behavior on a low-risk cohort with rollback proof and failure isolation.

## Pilot Cohort

- `ArqonContinuum`
- `ArqonCortex`

## Executed Steps

1. Cohort registration and preflight
- Registered both repos with `apply-pilot` tags.
- Verified clean state and branch status.

2. Branch apply rollout
- Created `feat/pilot-wave7` from `dev` in both repos.
- Confirmed both repos on new branch and clean.

3. Secure fix dry-run
- `ArqonCortex`: dry-run produced actionable steps (`cargo update`, `cargo check`).
- `ArqonContinuum`: no supported dependency manifest for secure-fix apply path (expected no-op).

4. Secure fix apply
- Applied to `ArqonContinuum` safely (no-op, success).
- Applied to `ArqonCortex`:
  - `cargo update` succeeded.
  - `cargo check` succeeded after guardrail fix.

Execution note:
- For this run, Pilot state home was isolated to `/tmp/pilot_wave7_home` due sandbox write restrictions on `/home/irbsurfer/.pilot` in this execution environment.

5. Rollback validation
- `ArqonContinuum`: commit + `git revert` rollback drill completed cleanly.
- `ArqonCortex`: post-apply lockfile mutation (`Cargo.lock`) restored via `git restore`, worktree returned clean.

## Hardening Improvement Included

- Fixed secure-fix pipeline to allow verification steps after mutating update step:
  - `cargo check` no longer blocked by same-run post-update dirtiness.

## Evidence Artifacts

- Audit log:
  - `/tmp/pilot_wave7_home/.pilot/audit.jsonl`
- Per-command outcomes:
  - `/tmp/pilot_wave7_home/.pilot/reports/*.json`

## Exit Criteria Assessment

- Apply-mode pilot on low-risk cohort: passed.
- Failures isolated and actionable: passed.
- Rollback path documented and tested: passed.

## Outcome

Wave 7 objectives met for controlled pilot rollout. Project can proceed to Wave 8 release readiness tasks.
