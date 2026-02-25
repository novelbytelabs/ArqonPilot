# Wave 6 + Wave 6.5 Completion Report

Date: 2026-02-25

## Objective

Complete cross-repo acceptance hardening and self-hosting validation without invasive changes to active repos.

## Delivered

1. Unified mutation audit logging
- Added shared audit/event utilities in `pilot-core`.
- Mutating command paths now append JSONL audit records at:
  - `~/.pilot/audit.jsonl`

2. Partial-failure and per-repo artifacts
- Added timestamped per-command JSON outcome artifacts at:
  - `~/.pilot/reports/<command>_<timestamp>.json`
- Instrumented mutating flows:
  - `branch create/sync/prune`
  - `multi register`, `multi deps set`, `multi prs create`
  - `secure fix`
  - `create feature/tests`
  - `know record`
  - `plan issues/score/roadmap`
  - `navigate` and `navigate --multi`
  - `init`

3. Deterministic dependency ordering fix
- Fixed `multi order` tie-breaking to use stable repo-name ordering when multiple roots exist.

4. Wave 6 E2E orchestration test
- Added test:
  - `tests/e2e_wave6_dryrun_test.rs`
- Covers:
  - registry setup
  - dependency ordering
  - branch dry-run orchestration
  - multi navigate dry-run
  - secure fix dry-run
  - artifact/audit file existence checks

5. Wave 6.5 dogfooding execution
- Ran full dry-run lifecycle against ArqonPilot as a registered repo in isolated `HOME`.
- Confirmed command surfaces and artifact generation for:
  - `multi status/order`
  - `branch create/sync/prune --dry-run`
  - `navigate --multi --dry-run`
  - `secure scan`, `secure fix` (dry-run default)
  - `plan issues/score/roadmap`
  - `create feature/tests --dry-run`
  - `know record/query`

6. Controlled apply subset (safe isolation)
- Executed apply-mode subset in an isolated local clone of ArqonPilot:
  - `branch create feat/dogfood-apply`
  - `create feature dogfood-apply`
- This satisfied Wave 6.5 apply validation while avoiding mutation of the active working repo.

## Validation

Passed:
- `cargo check -p pilot --locked`
- `cargo test -p pilot --locked --test e2e_wave6_dryrun_test --test branch_cli_test --test multi_cli_test --test navigate_cli_test --test secure_cli_test --test plan_cli_test --test create_cli_test --test know_cli_test --test heal_cli_test --test oracle_cli_test --test report_cli_test`

## Gotchas Observed

- `secure scan` can detect secret-like strings in docs/tests and produce expected false positives; triage policy is required.
- Deterministic order required explicit tie-breaking logic in dependency traversal.
- Apply-mode dogfooding is safest in an isolated clone when the active repo has in-flight changes.

## Exit Criteria Assessment

- Acceptance protocol tooling and checks implemented: yes.
- Audit + per-repo artifacts for mutating paths: yes.
- Dry-run lifecycle deterministic and repeatable: yes.
- Self-hosting on ArqonPilot command flow: yes.
