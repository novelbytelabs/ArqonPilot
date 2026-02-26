# Wave 10 Packaging Design (Locked)

Date: 2026-02-25

## Decision

PyPI packaging will use `maturin` as the primary build/publish path.

- Package name: `arqon-pilot`
- Delivery goal: `pip install arqon-pilot` provides a working `pilot` CLI.
- Approach: Rust-first core with Python package wrapper built via `maturin`.

## Why `maturin`

1. Keeps Rust implementation authoritative.
2. Produces standard Python wheels for PyPI/TestPyPI.
3. Supports repeatable CI publishing workflows.
4. Avoids fragile ad-hoc shell-wrapper-only packaging.

## Planned Artifacts

1. `pyproject.toml` for maturin build backend. ✅
2. TestPyPI/PyPI publish workflow (`.github/workflows/pypi.yml`). ✅
3. Local wheel smoke-check script (`scripts/pypi_smoke_check.sh`). ✅
4. CI install smoke stage (next step).

## Compatibility Targets

- Python: 3.10+
- Platforms: linux amd64/arm64, macOS amd64/arm64 (initial)
- Rust toolchain for build: `1.82.0`

## Implementation Status

Wave 10 scaffolding is implemented. Remaining execution tasks:

1. Run TestPyPI publication from CI (`workflow_dispatch` target `testpypi`).
2. Validate install in clean environment:
- `pip install -i https://test.pypi.org/simple arqon-pilot`
- `pilot --help`
3. Promote to PyPI publication gate on release tag after successful smoke.
