# Wave 3 Branch + Navigate Orchestration

Date: 2026-02-25

## Objective

Deliver dependency-aware cross-repo branch and release planning operations.

## Completed

1. New module crate: `pilot-branch`
- Introduced `crates/pilot-branch` with branch lifecycle operations:
  - `create_branch`
  - `sync_branch`
  - `branch_status`
  - `prune_branches`
- Added workspace and CLI dependency wiring.

2. Multi-repo dependency graph model
- Extended `pilot-multi` registry schema with `repo_deps`.
- Added:
  - `set_dependencies(repo, depends_on...)`
  - `dependency_order(filter)` with topological ordering.
- Added cycle/unresolved dependency failure guard.

3. Linked PR manifest planning
- Added linked PR plan types and manifest generator in `pilot-multi`.
- New command:
  - `pilot multi prs create [--group ...] [--tag ...] [--head-branch ...] [--base-branch ...] [--output ...] [--dry-run]`

4. CLI surface expanded
- Added `pilot branch` command group:
  - `pilot branch create <branch> [--base-branch ...] [--group ...] [--tag ...] [--dry-run]`
  - `pilot branch sync [--branch ...] [--base-branch ...] [--group ...] [--tag ...] [--dry-run]`
  - `pilot branch status [--group ...] [--tag ...]`
  - `pilot branch prune [--base-branch ...] [--group ...] [--tag ...] [--dry-run]`
- Added `pilot multi deps set` and `pilot multi order`.
- Extended `pilot navigate` with multi mode:
  - `pilot navigate --multi [--group ...] [--tag ...] [--plan-output ...] [--dry-run]`

5. Safety and determinism
- Mutating Wave 3 commands include `--dry-run` mode:
  - `branch create`, `branch sync`, `branch prune`
  - `multi deps set`
  - `multi prs create` (dry-run planning without writing manifest)
- Dependency order is deterministic via topological sort over selected repos.
- Per-repo outcomes are emitted for branch operations for failure isolation.

6. Tests
- Added:
  - `tests/branch_cli_test.rs`
- Extended:
  - `tests/multi_cli_test.rs` for deps/order/prs flow
  - `tests/navigate_cli_test.rs` for `--multi` help and dry-run behavior
- Added unit coverage in `pilot-multi` for dependency ordering.

## Gotchas Addressed

- Registry dependency edges require unique repo names; `deps set` resolves by name.
- Cyclic dependency edges return an explicit error from `multi order` / planning flows.
- Branch create/sync uses dependency order to reduce downstream break risk.
- Prune logic excludes protected defaults (`main`, `master`, selected base branch).

## Exit Criteria Mapping

- Branch control delivered: `branch create/sync/status/prune` implemented.
- Dependency-order graph delivered and enforced in orchestration commands.
- Linked PR preparation command delivered via manifest generation.
- Navigate now supports multi-repo coordination planning mode.
