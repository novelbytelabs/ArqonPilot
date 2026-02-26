# Developer Guide

## Prerequisites

- Rust toolchain pinned by `rust-toolchain.toml`
- `git`
- Python 3.10+ for packaging smoke checks

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

For a local operator panel (Branch, Multi, Telemetry), run:

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

## Release Readiness

```bash
./scripts/release_readiness_check.sh
```

## Toolchain Drift Prevention

Arqon Pilot enforces a dual-lane policy:
- core dev/test lane: Rust `1.82.0` + `Cargo.lock`
- packaging lane: Rust `1.88.0` + `Cargo.lock.packaging`

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
