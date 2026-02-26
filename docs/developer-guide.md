# Developer Guide

## Prerequisites

- Rust toolchain pinned by `rust-toolchain.toml`
- `git`
- Python 3.10+ for packaging smoke checks
- Frozen policy file: `scripts/frozen_versions.sh`
  - core lane Rust: `1.82.0`
  - packaging lane Rust: `1.88.0`
  - protobuf: `4.25.8` (`protoc` `25.8`)

## Build

```bash
cargo check -p pilot --locked
```

## Run

```bash
cargo run -p pilot -- --help
```

## ArqonBus Bridge

Use `pilot serve` to expose Branch and Multi operations through ArqonBus command lanes:

```bash
pilot serve --ws-url ws://127.0.0.1:9100 --room pilot --channel control --telemetry-channel telemetry
```

JWT auth is optional and read from `ARQONBUS_AUTH_JWT` by default.

For a local operator panel (Oracle, Heal, Dependencies, Branch, Multi, Telemetry), run:

```bash
pilot serve --ws-url ws://127.0.0.1:9100 --room pilot --channel control --telemetry-channel telemetry --ui-port 7788
```

Safety flags:

```bash
# allow mutations from UI/API only when explicitly intended
pilot serve ... --ui-port 7788 --ui-allow-mutations

# optional allowlist for UI/API commands
pilot serve ... --ui-port 7788 --ui-allow-command pilot.branch.status --ui-allow-command pilot.multi.status
```

## Critical Linux/Conda Runtime Step

If `pilot` fails with a shared-library error like `libssl-*.so.10` not found, add the
packaged runtime library directory to `LD_LIBRARY_PATH` via conda hooks.

```bash
mkdir -p "$CONDA_PREFIX/etc/conda/activate.d" "$CONDA_PREFIX/etc/conda/deactivate.d"

cat > "$CONDA_PREFIX/etc/conda/activate.d/arqon_pilot_libs.sh" <<'EOF'
export _ARQONPILOT_OLD_LD_LIBRARY_PATH="${LD_LIBRARY_PATH-}"
export LD_LIBRARY_PATH="$CONDA_PREFIX/lib/python3.10/site-packages/arqon_pilot.libs:${LD_LIBRARY_PATH-}"
EOF

cat > "$CONDA_PREFIX/etc/conda/deactivate.d/arqon_pilot_libs.sh" <<'EOF'
export LD_LIBRARY_PATH="${_ARQONPILOT_OLD_LD_LIBRARY_PATH-}"
unset _ARQONPILOT_OLD_LD_LIBRARY_PATH
EOF
```

Reactivate env and verify:

```bash
conda deactivate
conda activate helios-gpu-118
pilot --help
```

## Test

```bash
./scripts/test_matrix.sh all
```

## Mandatory Pre-Push Gate

Run this before every commit/push:

```bash
./scripts/prepush_gate.sh
```

This gate includes `cargo check -p pilot --locked` and targeted locked CLI tests.
To enforce it automatically on every `git push`, run once per clone:

```bash
./scripts/install_git_hooks.sh
```

CI validates the hook/gate contract with:

```bash
./scripts/verify_git_hook_policy.sh
```

If `Cargo.lock` drifts and pre-push fails with `edition2024` parser errors, run:

```bash
./scripts/repair_lock_182.sh
```

This restores a compatible core lockfile (or applies exact-version fallback transitions) and can re-run the gate.

If VS Code shows only a generic push failure message, run:

```bash
./scripts/push_main.sh
```

This captures verbose git transport diagnostics and writes a timestamped push log.
By default it pushes your current checked-out branch; pass an explicit branch when needed:
`./scripts/push_main.sh main`.

## Pre-Check Scripts Reference

These scripts are the required guardrail layer before commit/push:

1. `./scripts/prepush_gate.sh`
- Runs policy checks, locked compile, targeted locked CLI tests, and help-surface smoke check.
- Includes automatic retries for transient crates.io/DNS failures on cargo steps.
- Writes a timestamped log file to `~/.pilot/reports/` (or `/tmp/pilot-reports/` fallback).

2. `./scripts/verify_toolchain_policy.sh`
- Verifies Rust lane pins (`1.82.0` core, `1.88.0` packaging), lockfile policy wiring, and lockfile compatibility checks for core lane.
- Verifies PyPI workflow protobuf/protoc freeze pin (`4.25.8` / `25.8`) and rejects unpinned `protobuf-compiler` apt usage.
- Fails fast with explicit incompatible dependencies (for example `time 0.3.47`, `wit-bindgen 0.51.0`).
- Supports machine-readable mode: `./scripts/verify_toolchain_policy.sh --json`

3. `./scripts/verify_git_hook_policy.sh`
- Ensures `.githooks/pre-push` exists and calls `./scripts/prepush_gate.sh`.
- Ensures hook installer and mandatory locked compile gate are in place.
- Supports machine-readable mode: `./scripts/verify_git_hook_policy.sh --json`

4. `./scripts/install_git_hooks.sh`
- Sets `core.hooksPath=.githooks` so pushes run the gate automatically.

5. `./scripts/repair_lock_182.sh`
- Recovery script for lockfile drift in Rust `1.82.0` lane.
- Attempts compatible lock restore from git history; falls back to exact-version pin transitions.

## Guardrail Gotchas

Canonical registry: `docs/gotcha-registry.md`

1. If `git push` fails before upload, the pre-push hook blocked it intentionally for safety.
2. `edition2024` parser errors indicate lockfile drift for Rust `1.82.0`, not necessarily source-code regressions.
3. `cargo update -p <crate>` can be ambiguous; use exact IDs like `getrandom@0.4.1`.
4. A green local build with newer toolchain does not guarantee core-lane compatibility.
5. `prepush_gate.sh` writes logs to `~/.pilot/reports/` and falls back to `/tmp/pilot-reports/` if needed.
6. Some crates must be downgraded via upstream constraints first (for example `blake3` before `constant_time_eq`).
7. If push still fails after gate passes, check for normal git remote-state errors (non-fast-forward/auth), not policy failures.

## Release Readiness

```bash
./scripts/release_readiness_check.sh
```

## Toolchain Drift Prevention

Arqon Pilot enforces a dual-lane policy:
- core dev/test lane: Rust `1.82.0` + `Cargo.lock`
- packaging lane: Rust `1.88.0` + `Cargo.lock.packaging`
- protobuf/protoc lane: `4.25.8` / `25.8` (pinned in `.github/workflows/pypi.yml`)

Validate policy locally:

```bash
./scripts/verify_toolchain_policy.sh
```

## Packaging Smoke

```bash
python3 -m pip install maturin
maturin build --release --locked --out dist
./scripts/pypi_smoke_check.sh
```

## Non-Destructive Operating Pattern

Use `--dry-run` for any mutating command first:

```bash
pilot branch create feat/x --group core --dry-run
pilot navigate --multi --group core --dry-run
pilot secure fix --group core --dry-run
```
