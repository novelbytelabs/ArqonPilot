# Wave 1 Modularization Progress

Date: 2026-02-25

## Objective

Split the monolithic `pilot` crate into modular crates without changing behavior.

## Completed

1. Workspace split
- Added crates:
  - `pilot-core`
  - `pilot-oracle`
  - `pilot-heal`
  - `pilot-navigate`
- Updated workspace members in root `Cargo.toml`.

2. Code migration
- Moved Oracle implementation into `pilot-oracle`.
- Moved Heal implementation into `pilot-heal`.
- Moved Navigate implementation into `pilot-navigate`.
- Added initial shared types in `pilot-core` (`RepoContext`, `CommandReport`).

3. CLI rewiring
- `crates/pilot` now acts as CLI package and depends on modular crates.
- `main.rs` imports module crates via:
  - `pilot_oracle`
  - `pilot_heal`
  - `pilot_navigate`

4. Shared execution/report abstractions
- Added `pilot-core` shared abstractions:
  - `RepoContext`
  - `CommandReport`
- Wired these into CLI execution flow in `crates/pilot/src/main.rs`.
- Added optional machine-readable reporting via `--report-json`.

5. Compatibility exports
- `pilot` library re-exports module crates as:
  - `pilot::oracle`
  - `pilot::heal`
  - `pilot::navigate`
- Existing integration tests importing `pilot::oracle::*` remain valid.

6. Command-group integration and smoke tests
- Added:
  - `tests/oracle_cli_test.rs`
  - `tests/heal_cli_test.rs`
  - `tests/navigate_cli_test.rs`
- Added:
  - `tests/report_cli_test.rs`
- These validate command surface/help contract for each module group.

## Validation

Passed in this environment:
- `cargo fmt --all -- --check`
- `./scripts/wave0_safety_check.sh`

Blocked by environment networking:
- `cargo check -p pilot --locked`
- `cargo test -p pilot --locked`

Reason: DNS/network to `index.crates.io` unavailable.

## Notes

This is a structural split only. No intended behavioral changes were introduced to Oracle/Heal/Navigate execution flows.
