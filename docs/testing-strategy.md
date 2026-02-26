# Testing Strategy

Testing in ArqonPilot is designed to answer one simple question: "Can I trust this change?"

If you are new to the project, you do not need to understand every module before running tests. You can think of the test system as a safety ladder. The lower rungs check small pieces of behavior. The upper rungs check realistic workflows across multiple repos. Running tests from bottom to top gives you confidence that a local refactor did not silently break production behavior.

ArqonPilot groups tests into five layers. Each layer has a different purpose, and each catches a different type of failure.

## Quick Start for Beginners

If you only want the shortest safe path, run this first:

```bash
./scripts/test_matrix.sh all
```

This runs all five layers in order. If this command passes, your change has passed the full local gate.

If you are preparing a release, run the release gate right after:

```bash
./scripts/release_readiness_check.sh
```

## Why There Are Multiple Test Layers

A single huge test suite is hard to debug. When one command fails, you still do not know whether the bug is in a small function, a CLI surface, or a cross-repo workflow.

ArqonPilot splits tests by purpose so that failure location is clearer:
- Unit tests fail when local logic is wrong.
- Integration tests fail when commands or module boundaries are wrong.
- End-to-end tests fail when realistic workflows fail.
- Regression tests fail when old bugs come back.
- Adversarial tests fail when edge cases or hostile inputs are unsafe.

## Test Layers (Plain English)

## 1. Unit tests

Unit tests check small pieces of logic in isolation. They are fast and are usually the best first signal while you are coding.

Run:

```bash
./scripts/test_matrix.sh unit
```

Use this when:
- You changed internal Rust code in one crate.
- You want fast feedback before running heavier suites.

## 2. Integration tests

Integration tests check command surfaces and module boundaries. They verify that the CLI wiring and cross-module contracts still work.

Run:

```bash
./scripts/test_matrix.sh integration
```

Use this when:
- You changed CLI args/flags/subcommands.
- You changed behavior that spans more than one crate.

## 3. End-to-end (E2E) tests

E2E tests simulate realistic usage flows, including multi-step operations across temporary repos. These tests answer: "Can an operator actually complete the workflow?"

Run:

```bash
./scripts/test_matrix.sh e2e
```

Use this when:
- You changed branch, navigate, or multi-repo orchestration behavior.
- You need confidence that complete workflows still execute correctly.

## 4. Regression tests

Regression tests protect against bugs that were already fixed once. If these tests fail, an old problem likely returned.

Run:

```bash
./scripts/test_matrix.sh regression
```

Use this when:
- You touched areas with prior incidents.
- You are validating a release candidate.

## 5. Adversarial tests

Adversarial tests feed bad or risky inputs into the system on purpose. These tests verify safe failure behavior, isolation, and clear reporting.

Run:

```bash
./scripts/test_matrix.sh adversarial
```

Use this when:
- You changed validation, parsing, or dependency-order logic.
- You care about failure mode quality, not only success paths.

## Recommended Local Workflow

For everyday development:

1. Run `unit` while coding.
2. Run `integration` before opening a PR.
3. Run `all` before merge.

For release prep:

1. Run `all`.
2. Run `release_readiness_check.sh`.
3. Confirm packaging workflow and install smoke tests.

## CI Policy and Toolchain Notes

Core CI runs with Rust `1.82.0` for deterministic project validation. Packaging has a scoped exception in the PyPI workflow when needed for ecosystem compatibility. This separation is intentional and documented so you can keep core engineering policy stable while still shipping installable artifacts.

Release and packaging paths use locked dependency resolution. In practice, this means `Cargo.lock` must be present and up to date. If `Cargo.lock` drifts, CI failures are expected and should be fixed before release.

## How to Read Failures

When a test suite fails, do not rerun everything immediately. Start by understanding failure class:

- Unit failure: inspect the specific function/module logic first.
- Integration failure: inspect CLI parsing, command routing, and crate interfaces.
- E2E failure: inspect workflow assumptions (repo state, branch preconditions, tool availability).
- Regression failure: compare with the previous fix and re-check that behavior contract.
- Adversarial failure: inspect error handling and ensure failure is safe and explicit.

## Common Gotchas

1. Missing or stale `Cargo.lock` can break deterministic builds.
2. Some tests expect `git` to be available locally.
3. Packaging can fail even if core tests pass, so packaging validation is still required.
4. Adversarial failures should produce clear `--report-json` output for machine-readable triage.

## One-Line Reference

If you forget everything else, run:

```bash
./scripts/test_matrix.sh all
```

Then run:

```bash
./scripts/release_readiness_check.sh
```
