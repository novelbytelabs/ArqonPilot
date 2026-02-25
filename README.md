# ArqonPilot

ArqonPilot is a standalone CLI for single-repo and cross-repo engineering operations: oracle indexing/query, healing, release navigation, branch orchestration, security scans, planning, scaffolding, and knowledge capture.

## Core Modules

- `oracle`
- `heal`
- `navigate`
- `branch`
- `multi`
- `secure`
- `plan`
- `create`
- `know`

## Quickstart

```bash
cargo run -p pilot -- --help
cargo run -p pilot -- init
cargo run -p pilot -- multi register --path /path/to/repo --group core --tag apply-pilot
cargo run -p pilot -- multi status --group core
cargo run -p pilot -- branch create feat/pilot-wave --group core --dry-run
```

## Testing

Run the full matrix:

```bash
./scripts/test_matrix.sh all
```

Run by category:

```bash
./scripts/test_matrix.sh unit
./scripts/test_matrix.sh integration
./scripts/test_matrix.sh e2e
./scripts/test_matrix.sh regression
./scripts/test_matrix.sh adversarial
```

Release gate:

```bash
./scripts/release_readiness_check.sh
```

## Packaging

PyPI packaging uses `maturin`.

```bash
python3 -m pip install maturin
maturin build --release --locked --out dist
./scripts/pypi_smoke_check.sh
```

CI workflows:

- `.github/workflows/ci.yml`
- `.github/workflows/pypi.yml`

## Documentation

Primary docs are in `docs/` and published with MkDocs.

- `docs/developer-guide.md`
- `docs/testing-strategy.md`
- `docs/operator-runbook.md`
- `docs/branch-management-guide.md`
- `docs/pilot-deep-dive-plan.md`
