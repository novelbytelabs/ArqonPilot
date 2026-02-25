# Wave 8 Pilot v1 Release Readiness

Date: 2026-02-25

## Objective

Finalize operational readiness and release-candidate gating for `pilot-v1-rc1`.

## Delivered in Wave 8

1. Operator workflow runbook
- Added:
  - `docs/operator-runbook.md`
- Covers:
  - daily dry-run operations
  - controlled apply workflow
  - rollback and incident handling

2. Release-candidate acceptance checklist
- Added scripted checklist:
  - `scripts/release_readiness_check.sh`
- Includes:
  - locked compile
  - targeted locked CLI/E2E tests
  - command-surface checks

3. CI/readiness alignment
- Updated CI to run release-readiness verification script.
- Rust remains pinned to `1.82.0`.

4. Packaging decision lock for Wave 10
- PyPI strategy fixed to:
  - `maturin`/`pyo3` based package flow.

## RC Gate Checklist (`pilot-v1-rc1`)

- [ ] `cargo check -p pilot --locked` passes
- [ ] targeted locked test matrix passes
- [ ] runbook reviewed against actual operations
- [ ] latest acceptance artifacts (`audit.jsonl` and reports) available
- [ ] Wave 6, 6.5, and 7 completion reports linked in release notes

## Status

Wave 8 readiness artifacts are prepared; RC tag cut is pending final run + manual tag/push from operator environment.
