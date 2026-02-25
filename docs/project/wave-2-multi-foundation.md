# Wave 2 Multi-Repo Foundation

Date: 2026-02-25

## Objective

Deliver multi-repo registry and scoped cross-repo operations as the Wave 2 foundation.

## Completed

1. New module crate: `pilot-multi`
- Introduced `crates/pilot-multi` with a SQLite-backed registry.
- Added workspace membership and CLI dependency wiring.

2. Registry schema and operations
- Registry DB path: `~/.pilot/workspace.db`
- Tables:
  - `repos` (name/path/group)
  - `repo_tags` (many-to-one tags)
- Implemented:
  - register repo (upsert by canonical path)
  - list repos
  - status fanout over selected repos

3. Scoped selection model
- Added `RepoFilter` with:
  - `group`
  - repeated `tags`
- Filter semantics:
  - group exact match
  - all provided tags must be present on repo

4. CLI surface integrated
- Added `pilot multi` command group with:
  - `pilot multi register --path ... [--name ...] [--group ...] [--tag ...]`
  - `pilot multi list [--group ...] [--tag ...]`
  - `pilot multi status [--group ...] [--tag ...]`
  - `pilot multi query --query "..." [--group ...] [--tag ...] [--per-repo-limit N]`

5. Cross-repo Oracle query fanout
- Implemented async fanout query over selected repos.
- Per-repo behavior:
  - if `.pilot/graph.db` or `.pilot/vectors.lance` missing, returns per-repo error
  - otherwise runs query via `pilot-oracle::QueryEngine`

6. Reporting integration
- `multi.*` commands now emit standardized `CommandReport` summaries.
- `--report-json` continues to work for multi commands.

7. Tests
- Added CLI integration tests:
  - `tests/multi_cli_test.rs`
- Added unit coverage in `pilot-multi` for register/list/filter behavior.

## Validation

Passed in this environment:
- `cargo fmt --all -- --check`
- `./scripts/wave0_safety_check.sh`

Blocked by environment networking:
- `cargo check -p pilot --locked`
- `cargo test -p pilot --locked`

Reason: DNS/network to `index.crates.io` is unavailable in this environment.

## Notes

Wave 2 focused on multi-repo registry and fanout foundations only. Advanced cross-repo automation (branch orchestration, dependency-order merge planning, linked PRs) is part of subsequent phases.
