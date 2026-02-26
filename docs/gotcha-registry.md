# ArqonPilot Gotcha Registry

This is the canonical list of recurring failures, their signature, and exact recovery flow.
Keep this file current whenever a new failure class appears.

## G-001: Rust 1.82 drift to `edition2024` crates

- Signature:
  - `feature edition2024 is required`
  - `rustc 1.82.0 is not supported ...`
- Typical crates:
  - `time 0.3.47+`
  - `comfy-table 7.2.2+`
  - `wit-bindgen 0.51.0+`
  - `globset 0.4.18+`
  - `constant_time_eq 0.4.x`
- Recovery:
  1. `./scripts/repair_lock_182.sh --no-gate`
  2. `./scripts/prepush_gate.sh`
  3. `./scripts/push_main.sh`

## G-002: ICU 2.1.x drift in core lockfile

- Signature:
  - `icu_collections@2.1.1 requires rustc 1.83`
  - same for `icu_locale_core`, `icu_normalizer`, `icu_properties`, `icu_provider`
- Cause:
  - `Cargo.lock` drifted to ICU `2.1.x`, but core lane is frozen at Rust `1.82.0`.
- Recovery:
  1. `./scripts/repair_lock_182.sh --no-gate`
  2. `./scripts/prepush_gate.sh`
  3. `./scripts/push_main.sh`

## G-003: DNS/index failures during cargo operations

- Signature:
  - `Could not resolve host: index.crates.io`
  - `failed to download from https://index.crates.io/...`
- Recovery:
  1. Verify DNS:
     - `getent hosts index.crates.io`
     - `getent hosts static.crates.io`
  2. Re-run:
     - `./scripts/prepush_gate.sh`
- Notes:
  - Gate has retry logic and emits DNS diagnostics.
  - If DNS is down, repair scripts that need `cargo update` cannot complete.

## G-004: Generic VS Code push failure

- Signature:
  - `error: failed to push some refs ...` with no useful detail.
- Recovery:
  1. Use wrapper:
     - `./scripts/push_main.sh`
  2. Read final summary block:
     - `result`, `prepush_gate_rc`, `git_push_rc`, `likely_cause`, `full_log`.

## Frozen Policy (Do Not Change)

- Core Rust lane: `1.82.0`
- Packaging Rust lane: `1.88.0`
- Protobuf: `4.25.8` / `protoc 25.8`
- Source of truth: `scripts/frozen_versions.sh`
