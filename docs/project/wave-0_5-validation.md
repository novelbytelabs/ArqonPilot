# Wave 0.5 Validation and Dependency Strategy

Date: 2026-02-25

## Objective

Harden extraction stability after Wave 0 with a focus on non-breaking behavior and reproducible builds.

## What was added

1. Locked CI installs/checks
- CI now runs `cargo check -p pilot --locked` and `cargo test -p pilot --locked`.

2. Toolchain pin
- Added `rust-toolchain.toml` pinned to Rust `1.82.0` with `rustfmt` and `clippy`.

3. Safety invariant script
- Added `scripts/wave0_safety_check.sh`.
- Verifies hard-cut invariants:
  - no `.arqon` usage
  - no `arqon` binary name
  - no `ship` command wiring
  - no ArqonHPO-specific `crates/core/Cargo.toml` fallback
  - presence of `pilot`, `oracle`, and `navigate` command surfaces

4. Online verification runner
- Added `scripts/verify_online.sh`.
- Runs invariant checks, format check, locked compile/tests, and CLI help smoke check.

## Validation results in current environment

1. Passed
- `cargo fmt --all -- --check`
- `scripts/wave0_safety_check.sh`

2. Blocked by environment networking
- `cargo check -p pilot --locked`
- `cargo test -p pilot --locked`

Reason: this environment cannot resolve `index.crates.io` (DNS/network unavailable), so dependency resolution cannot complete online.

## Recommendation

When network is available, run:

```bash
./scripts/verify_online.sh
```

If lockfile drift appears, refresh in a controlled commit:

```bash
cargo update
cargo check -p pilot
cargo test -p pilot
```

Then commit `Cargo.lock` update with no other behavioral changes.
