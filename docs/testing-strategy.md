# Testing Strategy

Arqon Pilot uses five test layers to keep behavior stable as capabilities expand.

## Test Layers

1. Unit tests
- Scope: crate-local logic in `src` modules.
- Command: `./scripts/test_matrix.sh unit`

2. Integration tests
- Scope: CLI command groups and module boundaries.
- Command: `./scripts/test_matrix.sh integration`

3. End-to-end tests
- Scope: realistic multi-step flows across registered repos.
- Command: `./scripts/test_matrix.sh e2e`

4. Regression tests
- Scope: previously broken behavior that must never return.
- Command: `./scripts/test_matrix.sh regression`

5. Adversarial tests
- Scope: malformed input, cycle/failure isolation, and hostile edge cases.
- Command: `./scripts/test_matrix.sh adversarial`

## Full Local Validation

```bash
./scripts/test_matrix.sh all
./scripts/release_readiness_check.sh
```

## CI Policy

- CI runs on Rust `1.82.0` for core checks.
- PyPI packaging is a scoped exception with newer Rust in `pypi.yml`.
- `cargo --locked` is required in release and packaging flows.

## Gotchas

1. If `Cargo.lock` is missing, builds may resolve incompatible crate versions.
2. Adversarial failures should emit machine-readable `--report-json` output.
3. E2E tests require local `git` available for temporary repos.
4. Packaging can fail even when core CI passes; always run the TestPyPI lane.
