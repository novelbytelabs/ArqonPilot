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

## G-005: Local pass but CI fail (lane mismatch)

- Signature:
  - local `cargo check -p pilot --locked` passes
  - CI fails in core or packaging job with dependency/toolchain mismatch
- Cause:
  - lane drift (core `1.82.0` vs packaging `1.88.0`)
  - lockfile mismatch (`Cargo.lock` vs `Cargo.lock.packaging`)
  - CI workflow changes not validated locally
- Prevention:
  1. `./scripts/ci_parity_check.sh`
  2. `./scripts/push_main.sh`
- Recovery:
  1. `./scripts/verify_toolchain_policy.sh`
  2. `./scripts/repair_lock_182.sh --no-gate`
  3. `./scripts/packaging_lane_check.sh`
  4. `./scripts/prepush_gate.sh`

## G-006: Packaging lane toolchain missing locally

- Signature:
  - `toolchain '1.88.0-x86_64-unknown-linux-gnu' is not installed`
  - failure from `./scripts/packaging_lane_check.sh` or `./scripts/ci_parity_check.sh`
- Recovery:
  1. `rustup toolchain install 1.88.0-x86_64-unknown-linux-gnu`
  2. `./scripts/packaging_lane_check.sh`
  3. `./scripts/ci_parity_check.sh`

## G-007: ArqonBus shim drops after short uptime

- Signature:
  - Control Panel `ArqonBus` chip flips `CONNECTED -> DISCONNECTED`
  - telemetry stream shows:
    - `IO error: Connection refused (os error 111)`
- Cause:
  - launching long-lived shim via `conda run` process tree may terminate unexpectedly.
- Recovery:
  1. Use shim manager:
     - `PILOT_REPORT_DIR=/tmp/pilot-reports ./scripts/arqonbus_shim.sh start`
     - `PILOT_REPORT_DIR=/tmp/pilot-reports ./scripts/arqonbus_shim.sh status`
  2. In Dashboard:
     - click `Bus Status` then `Start Bus`
  3. Verify listener:
     - `ss -ltnp | rg ':9100'`

## Frozen Policy (Do Not Change)

- Core Rust lane: `1.82.0`
- Packaging Rust lane: `1.88.0`
- Protobuf: `4.25.8` / `protoc 25.8`
- Source of truth: `scripts/frozen_versions.sh`
