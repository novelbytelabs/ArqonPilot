# Wave 4 Secure + Heal Expansion

Date: 2026-02-25

## Objective

Deliver practical security maintenance and multi-file healing preparation.

## Completed

1. New module crate: `pilot-secure`
- Added `crates/pilot-secure` with:
  - dependency vulnerability scan adapters:
    - `cargo audit --json` (if available)
    - `pip-audit --format json` (if available)
  - secret scanning across repo files with high-signal rules
  - dependency fix workflow with strict dry-run default

2. CLI security surface
- Added `pilot secure` command group:
  - `pilot secure scan [--group ...] [--tag ...]`
  - `pilot secure fix [--group ...] [--tag ...] [--apply]`
- Default fix behavior is non-mutating preview; `--apply` is required to mutate.
- Secure operations can target filtered registered repos, or current repo if none registered and no filter requested.

3. Heal multi-file planning
- Added `pilot-heal` planning module:
  - `build_multifile_repair_plan(...)`
- Added CLI support:
  - `pilot heal --log-file ... --plan-only [--max-files N]`
- Plan includes primary file, sibling candidates, conventional test companions, and related Oracle signatures.

4. Tests
- Added `tests/secure_cli_test.rs` for secure command coverage.
- Extended `tests/heal_cli_test.rs` for `--plan-only` help surface.
- Added unit tests in:
  - `pilot-secure` (secret scanning)
  - `pilot-heal` plan module (multi-file candidate inclusion)

## Gotchas Addressed

- Fix mutation safety: `secure fix` is dry-run by default, with explicit `--apply`.
- Dirty repo protection: apply mode refuses mutation on non-clean repos.
- Tool availability variance: scan output includes explicit info findings when `cargo-audit`/`pip-audit` are not installed.
- Multi-repo fallback: filtered scans require matching registered repos; unfiltered scans fall back to current repo for single-repo usage.

## Exit Criteria Mapping

- `pilot secure scan` and `pilot secure fix --dry-run` now produce actionable output across selected repos.
- Healing workflow now supports multi-file patch planning mode for safer cross-file repair preparation.
