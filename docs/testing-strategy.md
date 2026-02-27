# Testing Strategy

Testing in ArqonPilot is designed to answer one simple question: "Can I trust this change?"

If you are new to the project, you do not need to understand every module before running tests. You can think of the test system as a safety ladder. The lower rungs check small pieces of behavior. The upper rungs check realistic workflows across multiple repos. Running tests from bottom to top gives you confidence that a local refactor did not silently break production behavior.

ArqonPilot groups tests into five layers. Each layer has a different purpose, and each catches a different type of failure.

## Quick Start for Beginners

If you only want the shortest safe path, run this first:

```bash
./scripts/test_matrix.sh all
```

Before every commit/push, run the mandatory pre-push gate:

```bash
./scripts/prepush_gate.sh
```

This runs all five layers in order. If this command passes, your change has passed the full local gate.

If you are preparing a release, run the release gate right after:

```bash
./scripts/release_readiness_check.sh
```

## Guardrail Script Map

Use these scripts as the standard pre-check stack:

1. `./scripts/install_git_hooks.sh`
- One-time per clone.
- Installs `.githooks/pre-push` so push automatically runs the gate.

2. `./scripts/prepush_gate.sh`
- Mandatory before commit/push (or automatically on push via hook).
- Runs policy checks, locked compile, targeted locked CLI tests, and help-surface smoke.
- Writes a timestamped log to `~/.pilot/reports/` (fallback: `/tmp/pilot-reports/`).

3. `./scripts/verify_toolchain_policy.sh`
- Verifies dual-lane policy wiring and lockfile compatibility for Rust `1.82.0` core lane.
- Fails early with explicit incompatible crate/version entries.

4. `./scripts/verify_git_hook_policy.sh`
- Ensures hook wiring and pre-push gate contract do not drift.

5. `./scripts/repair_lock_182.sh`
- Recovery path when lockfiles drift to `edition2024` dependencies.
- Attempts compatible lock restore from history; falls back to exact-version transitions.

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
# optional Control Panel + API smoke (deterministic lane)
./scripts/ui_smoke_check.sh
# optional full command-lane checks when bus compatibility is confirmed
PILOT_UI_SMOKE_INCLUDE_COMMANDS=1 ./scripts/ui_smoke_check.sh
```

Use this when:
- You changed branch, navigate, or multi-repo orchestration behavior.
- You need confidence that complete workflows still execute correctly.
- You changed Control Panel action flow/state chips/sequence guidance and want a quick endpoint-level sanity check.

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
4. Run `prepush_gate.sh` before every commit/push.

For release prep:

1. Run `all`.
2. Run `release_readiness_check.sh`.
3. Confirm packaging workflow and install smoke tests.

## CI Policy and Toolchain Notes

Core CI runs with Rust `1.82.0` for deterministic project validation. Packaging has a scoped exception in the PyPI workflow when needed for ecosystem compatibility. This separation is intentional and documented so you can keep core engineering policy stable while still shipping installable artifacts.

Frozen versions are enforced by guardrail scripts:
- core Rust `1.82.0`
- packaging Rust `1.88.0`
- protobuf `4.25.8` (`protoc` `25.8`)

CI parity is enforced by:
- `./scripts/packaging_lane_check.sh` (packaging lane local/CI check)
- `./scripts/ci_parity_check.sh` (combined core + packaging lane validation)

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
5. `edition2024` parser failures in Rust `1.82.0` usually mean lockfile drift, not source-code breakage.
6. `cargo update -p <name>` can be ambiguous when multiple versions exist; pin with `name@from_version`.
7. A passing local `cargo check` with a newer toolchain does not guarantee core-lane `1.82.0` compatibility.

## One-Line Reference

If you forget everything else, run:

```bash
./scripts/test_matrix.sh all
```

Then run:

```bash
./scripts/release_readiness_check.sh
```
