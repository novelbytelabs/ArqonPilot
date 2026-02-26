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

## Test

```bash
./scripts/test_matrix.sh all
```

## Release Readiness

```bash
./scripts/release_readiness_check.sh
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
