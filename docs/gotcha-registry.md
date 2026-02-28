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

## G-013: DNS flaps where lookup passes but git still fails

- Signature:
  - `getent hosts github.com` succeeds
  - but `git fetch` / `git push` intermittently fails with:
    - `Could not resolve host: github.com`
- Cause:
  - resolver/network flapping between checks and live HTTPS git operations.
- Recovery:
  1. Use retrying wrapper, not raw push:
     - `./scripts/push_main.sh`
  2. If it still fails, re-run wrapper after a short interval:
     - `sleep 10 && ./scripts/push_main.sh`
  3. Confirm summary block shows:
     - `result: SUCCESS`
     - `git_push_rc: 0`

## G-014: UI smoke CI fails with missing `protoc`

- Signature:
  - `failed to run custom build command for lance-encoding`
  - `Could not find protoc`
  - fails in `./scripts/ui_smoke_check.sh` during `cargo run -p pilot -- serve ...`
- Cause:
  - UI smoke workflow job started without installing pinned protoc runtime.
- Recovery:
  1. Ensure `ui-smoke` job installs pinned protoc `25.8` before smoke check.
  2. Re-run CI.

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

## G-008: Expected GitHub auth challenge misread as push failure

- Signature:
  - push log contains `HTTP/2 401` during initial handshake
  - but final push succeeds
- Cause:
  - GitHub HTTPS flow challenges first, then retries with credentials.
- Current handling:
  - `scripts/push_main.sh` reports this in `auth_challenge_events`.
  - `errors_in_log` excludes this when `git_push_rc=0`.
- Recovery:
  - none required if summary shows:
    - `result=SUCCESS`
    - `git_push_rc=0`
    - `divergence_after_push behind=0 ahead=0`

## G-009: Codex contract request rejected

- Signature:
  - `Codex` tab returns validation error.
- Cause:
  - missing `intent`, non-namespaced command, invalid JSON payload, or mutating command in read-only mode.
- Recovery:
  1. Run `Preview Contract`.
  2. Fix payload JSON and required fields.
  3. If execution requires mutation, run `pilot serve ... --ui-allow-mutations`.

## G-010: Shell is running stale installed `pilot` binary

- Signature:
  - `error: unrecognized subcommand 'db'`
  - missing newer commands in `pilot --help`
- Cause:
  - `pilot` resolves to an older package binary in environment PATH (for example conda env), not the repo build.
- Recovery:
  1. `which pilot`
  2. Run from repo wrapper:
     - `./scripts/pilot_local.sh --help`
     - `./scripts/pilot_local.sh db status`
  3. Optionally reinstall package in the environment after release.

## G-011: Managed DB startup fails with `postgres.log` permission denied

- Signature:
  - `cannot create ~/.arqon/pilot/db/postgres.log: Permission denied`
  - `pg_ctl: could not start server`
- Cause:
  - local file permission/ownership mismatch for Pilot DB log path.
- Recovery:
  1. `chmod u+rw ~/.arqon/pilot/db/postgres.log || true`
  2. `chmod u+rwx ~/.arqon/pilot/db || true`
  3. `rm -f ~/.arqon/pilot/db/postgres.log`
  4. `./scripts/pilot_local.sh db start`
  5. `./scripts/pilot_local.sh db status`

## G-012: DB running but AGOrg commands fail with socket `os error 2`

- Signature:
  - `./scripts/pilot_local.sh db status` reports `"running": true`
  - `./scripts/pilot_local.sh agorg list` fails with:
    - `No such file or directory (os error 2)`
- Cause:
  - Unix-socket DSN missing `port`; Postgres client falls back to default socket port `5432`
    while Pilot-managed DB runs on `9132`.
- Recovery:
  1. Confirm DSN includes socket host and port:
     - `./scripts/pilot_local.sh db status`
  2. Ensure DSN contains `port=9132` (or configured `PILOT_DB_PORT`).
  3. Restart managed DB:
     - `./scripts/pilot_local.sh db stop`
     - `./scripts/pilot_local.sh db start`
  4. Re-run:
     - `./scripts/pilot_local.sh agorg list`

## G-015: Entire Pilot UI Dead — Duplicate `const` Declarations Kill Script Block

- Signature:
  - **The entire Control Panel is completely unresponsive.** No tabs switch, no buttons work, no data loads. The page renders HTML/CSS but zero JavaScript executes.
  - Browser DevTools Console shows: `SyntaxError: Identifier 'agorgOut' has already been declared` (or similar for any duplicated `const`).
  - This is NOT a partial failure. If you see any single feature broken, check all features — they are likely ALL broken.
- Cause:
  - The `serve_ui.rs` file contains a single monolithic `<script>` block. All `const` variable declarations for DOM elements live in that block.
  - When editing this file, a new `const agorgOut = ...` (or any other DOM variable) was added at one location **without checking that the same `const` already existed** elsewhere in the same block.
  - JavaScript's `const` does not allow re-declaration in the same scope. A duplicate `const` causes an **immediate, fatal `SyntaxError`** that aborts parsing of the **entire** `<script>` block before any code executes.
  - This means: no tab switching, no API calls, no dropdown, no registry, no stream — the page is a static HTML shell.
- Why this keeps happening:
  - The `<script>` block is ~1500 lines. DOM variable declarations appear in two separate clusters (~line 3264 and ~line 3315). When adding a new variable, it is easy to insert it in the first cluster without noticing the second cluster already declares the same name.
  - `cargo check` does NOT validate JavaScript. The Rust code compiles perfectly even when the embedded JS is syntactically broken.
- Recovery:
  1. Open browser DevTools Console (F12). If you see `SyntaxError: Identifier '...' has already been declared`, that is the problem.
  2. Search the `<script>` block in `serve_ui.rs` for the duplicated identifier: `grep -n 'const <varname>' serve_ui.rs`.
  3. Delete the duplicate declaration. Keep only one.
  4. Rebuild and reload.
- Prevention:
  1. **Before adding ANY `const` declaration**, grep the entire file for that variable name first.
  2. **Before declaring done**, open the browser DevTools Console and verify zero errors on page load.
  3. `cargo check` passing does NOT mean the UI works. The JS is an opaque string to Rust.

## G-016: Discovery output is clean but AGOrg tree still contains stale AGO rows

- Signature:
  - `agorg discover --root ...` returns top-level candidates only
  - `agorg tree` still shows historical nested/archive rows (for example `archive/...`, `bindings/python`)
- Cause:
  - discovery guardrails affect new scans but do not retroactively delete already imported AGO rows.
- Recovery:
 1. Reconcile import with prune:
     - `./scripts/pilot_local.sh agorg discover --root /home/irbsurfer/Projects/arqon --depth 4 --import-to Arqon --prune-missing`

## G-017: Reconcile Apply blocked in UI with "read-only mode"

- Signature:
  - UI/API returns:
    - `reconcile apply blocked in read-only UI mode`
- Cause:
  - `pilot serve` started without mutation enable flag.
- Recovery:
  1. Restart serve with:
     - `pilot serve ... --ui-allow-mutations`
  2. Re-run:
     - `Reconcile Dry Run` first
     - then `Reconcile Apply`
  2. Verify:
     - `./scripts/pilot_local.sh agorg tree --root Arqon`
- Notes:
  - default discovery is flat-fleet (nested repos skipped, `archive/` skipped).
  - set `PILOT_AGORG_ALLOW_NESTED_REPOS=1` only when nested-repo discovery is intentionally required.

## G-017: AGOrg looks clean but policy drift still exists

- Signature:
  - Discovery/import succeeds, but downstream operations still fail scope expectations.
  - Hidden issues include missing `pyproject.toml` or legacy off-policy entries.
- Recovery:
  1. Run:
     - `./scripts/pilot_local.sh agorg reconcile --agorg Arqon`
  2. Fix reported issues (or prune/reimport for off-policy paths).
  3. Re-run reconcile until issue count is acceptable.

## G-018: Scope guard rejects command as unscoped

- Signature:
  - `No active AGOrg scope selected`
  - `Current repo path ... is outside active AGOrg scope`
  - `Scope guard: multi-repo command requires explicit selector (group or tags)`
  - Same scope failures may appear from Dashboard dependency actions (`policy`, `hook-policy`, `drift`, `gate`, `repair`, `push`).
- Cause:
  - command family now enforces AGOrg scope and selector requirements.
- Recovery:
  1. Set active AGOrg:
     - `./scripts/pilot_local.sh agorg use <id-or-name>`
  2. Ensure current repo path is under AGOrg root for repo-local commands.
3. For `pilot.multi.*`, set `group` and/or `tags` in UI/CLI payload.
4. For Dashboard dependency actions above, run Pilot from a repo path inside the active AGOrg root.

## G-019: Live Event Stream shows `agorg_scope: null`

- Signature:
  - SSE telemetry payloads include `agorg_scope: null`.
- Cause:
  - No active AGOrg is selected, or event source emitted outside scoped command flows.
- Recovery:
 1. Set active AGOrg:
     - `./scripts/pilot_local.sh agorg use <id-or-name>`
 2. Confirm:
     - `./scripts/pilot_local.sh agorg show`

## G-020: Two UI windows show different active AGOrg/tab state

- Signature:
  - Separate UI windows disagree on active scope or restored tab/context.
- Cause:
  - State is isolated per `ui_instance_id` by design (Wave D).
- Recovery:
  1. Start with explicit instance id:
     - `pilot serve --ui-port 7788 --ui-instance-id pilot-main ...`
  2. Use same id to share state across windows, different ids to isolate intentionally.

## G-021: Temporary component state is unclear during incident triage

- Signature:
  - Operator cannot quickly tell whether bus shim/editor-gap components are active.
  - Debugging starts with guesswork across multiple files or shell commands.
- Recovery:
  1. Use Dashboard:
     - `Temporary Components Inventory` -> `Refresh Inventory`
  2. Or query API directly:
     - `curl -sS http://127.0.0.1:7788/api/system/temporary_components | jq`
  3. Confirm component status and exit criteria from returned `components[]`.
- Notes:
  - Inventory includes ArqonBus shim live status and current hierarchy-editor path.
  - Use this before attempting manual workaround scripts.

## Frozen Policy (Do Not Change)

- Core Rust lane: `1.82.0`
- Packaging Rust lane: `1.88.0`
- Protobuf: `4.25.8` / `protoc 25.8`
- Source of truth: `scripts/frozen_versions.sh`
