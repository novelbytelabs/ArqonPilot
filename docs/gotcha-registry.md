# ArqonPilot Gotcha Registry

This is the canonical list of recurring failures, their signature, and exact recovery flow.
Keep this file current whenever a new failure class appears.

## G-040: Uuid Serialization Mismatch (Option<i64> vs String)

- Signature:
  - `Expected Option<i64> found String` or `mismatched types: expected struct String, found struct Uuid`
  - impacts API payload serialization/deserialization when extending governance models.
- Cause:
  - Some generic `Audit` event fields historically expected `i64` or implicit integers. New P4 structs (`BranchUndoEntry`, `BranchTimelineEvent`) generate UUID v4s for `scope_id`, leading to mismatch across boundaries.
- Recovery:
  1. Standardize on `Option<String>` for `scope_id` inside events and journals.
  2. For Uuid structs like `eval::Scope`, explicitly call `.id.to_string()` when packing into `Option<String>`.
  3. Ensure SQL `as_deref()` comparisons account for this string projection.

## G-041: False P1 parity failures in restricted runtimes (managed Postgres denied)

- Signature:
  - `verify_policy_parity.sh` or policy integration/e2e tests fail with:
    - `could not create any Unix-domain sockets`
    - `Unix-domain socket path ... is too long (maximum 107 bytes)`
    - `could not open shared memory segment ... Permission denied`
    - `Operation not permitted`
- Cause:
  - Runtime/sandbox denies Postgres unix-socket or shared-memory primitives; this is infra-level denial, not policy logic failure.
- Recovery:
  1. Treat as environment constraint if and only if signature matches above.
  2. Use deterministic skip path:
     - `scripts/verify_policy_parity.sh` now preflights DB start and returns `[SKIP]` with exit `0` only for these known-denied signatures.
  3. Re-run parity on normal workstation/runtime for full evidence:
     - `bash scripts/verify_policy_parity.sh`
  4. If release-readiness fails in sandbox with Postgres shared-memory denial:
     - run `./scripts/release_readiness_check.sh` on host permissions (outside sandbox) and archive the result in FC artifacts.

## G-042: `services restart` intermittently fails DB start while Bus status flaps

- Signature:
  - `pilot services restart` fails with DB startup errors, then later succeeds.
  - `pilot bus status` may report `STOPPED` even while shim is actually running.
- Cause:
  - Legacy DB log-path fallback to `/tmp/arqon_pilot_postgres.log` introduced ambiguous/stale diagnostics and non-deterministic start behavior.
  - Profile-isolated shell execution (`--noprofile --norc`) can miss `ss` in PATH (`/usr/sbin`), causing false-negative shim status checks.
- Recovery:
  1. Use latest code where:
     - managed DB log path is deterministic (`~/.arqon/pilot/run/postgres.log` by default),
     - local-script PATH includes `/usr/sbin`,
     - shim resolves `ss` via absolute fallback.
  2. Restart services:
     - `./scripts/pilot_local.sh services restart`
  3. Verify both:
     - `./scripts/pilot_local.sh db status`
     - `./scripts/pilot_local.sh bus status`

## G-043: Preflight evidence write can fail (`Permission denied`) while tests still pass

- Signature:
  - `Warning: Failed to write preflight evidence to ~/.pilot/reports/preflight_<...>.json: Permission denied (os error 13)`
  - Seen during `cargo test -p pilot --locked test_preflight_graph_pass -- --nocapture`.
- Cause:
  - Test/runtime user cannot write to `~/.pilot/reports` (ownership/permissions drift), but graph semantics still pass because file-write is surfaced as warning.
- Recovery:
  1. Repair report directory ownership/permissions:
     - `mkdir -p ~/.pilot/reports`
     - `chmod u+rwx ~/.pilot ~/.pilot/reports`
     - `chown -R "$USER":"$USER" ~/.pilot`
  2. Re-run targeted graph tests:
     - `cargo test -p pilot --locked test_preflight_graph_pass -- --nocapture`
  3. Verify fresh artifact exists:
     - `ls -lt ~/.pilot/reports/preflight_*.json | head`
- Prevention:
  - Include a writable-report-dir check in preflight hard-close packet evidence whenever artifact emission is claimed.

## G-044: Cross-repo drift (ArqonPilot work written into `Arqon/`)

- Signature:
  - ArqonPilot plan/artifact files appear under `Arqon/` paths (for example `Arqon/ArqonPilot/...` or `Arqon/docs/polity/...` unexpectedly updated during Pilot waves).
- Cause:
  - Session started in wrong repo root, or automation executed with `cwd` outside `/home/irbsurfer/Projects/arqon/ArqonPilot`.
- Recovery:
  1. Run boundary check before any edits:
     - `./scripts/repo_boundary_guard.sh`
  2. Audit misplaced files:
     - `cd /home/irbsurfer/Projects/arqon && find Arqon -type f | rg -n "ArqonPilot|PRODUCTIONIZE|federated-ci-program-plan"`
  3. Move/delete misplaced files only after operator confirmation.
- Prevention:
  - Make `./scripts/repo_boundary_guard.sh` the first command in every new AI session.

## G-045: Pre-push discipline gate blocks push after AGOrg/session drift

- Signature:
  - `[discipline] ERROR: Pilot UI API is unavailable on http://127.0.0.1:7788`
  - `[discipline] ERROR: no active AGOrg scope selected`
  - `[discipline] ERROR: current repo is not registered as an AGO under active AGOrg`
- Cause:
  - `scripts/prepush_gate.sh` now enforces `scripts/pilot_discipline_gate.sh` (step `[0/4]`) before compile/test checks.
  - Active UI scope is unset, wrong, or missing `ArqonPilot` AGO registration.
- Recovery:
  1. Start Pilot UI on expected port:
     - `cargo run -p pilot -- serve --ws-url ws://127.0.0.1:9100 --room pilot --channel control --telemetry-channel telemetry --ui-port 7788 --ui-allow-mutations`
  2. Select correct AGOrg in header chip (`AGOrg: ...`).
  3. Ensure `ArqonPilot` is registered as AGO under that AGOrg.
  4. Re-run:
     - `./scripts/prepush_gate.sh`
     - `./scripts/push_main.sh`
- Controlled bypass:
  - `PILOT_ENFORCE_AGORG_DISCIPLINE=0 ./scripts/prepush_gate.sh`

## G-046: `pilot.multi.register` appears to succeed then times out in UI

- Signature:
  - UI first shows:
    - `"status": "running", "command": "pilot.multi.register"`
  - then falls back to:
    - `"error": "Request timed out. Check ArqonBus bridge health and try again."`
- Cause:
  - A 25s frontend abort timer could fire before backend bus-recovery/fallback completed.
  - `pilot.multi.*` commands are local control-plane operations and do not require waiting on the bus bridge path.
- Recovery:
  1. Use build containing the fix:
     - `run()` timeout raised to 90s for command orchestration.
     - `pilot.multi.*` routes execute via local direct path in `run_command`.
  2. Restart Pilot UI process and hard-refresh browser.
  3. Retry `Register Repo` in Multi tab.
- Verification:
  - Result should return `ok: true` with:
    - `"execution_mode": "local_direct"`
  - If it fails, error body should include a concrete command failure, not a generic bus timeout.

## G-047: Mixed/stale Pilot UI instances cause misleading behavior

- Signature:
  - startup error:
    - `Refusing mixed Pilot UI versions ... conflicts with current version ...`
  - or UI behavior does not match latest code (old timeout message, missing dropdown updates).
- Cause:
  - multiple `pilot serve` instances running on different ports/builds, or browser serving stale JS bundle.
- Recovery:
  1. Stop old Pilot UI processes.
  2. Start a single latest instance:
     - `cargo run -p pilot -- serve --ws-url ws://127.0.0.1:9100 --room pilot --channel control --telemetry-channel telemetry --ui-port 7788 --ui-allow-mutations`
  3. Hard refresh browser.
  4. Verify current behavior:
     - Multi register success includes `execution_mode: "local_direct"`.
     - Settings `Target AGO` appears as dropdown, not free-text input.

## G-048: Local-direct command path fails on UI-injected scope metadata

- Signature:
  - `Local direct execution failed: Invalid pilot.multi.register payload: unknown field agorg_scope`
- Cause:
  - UI command guard injects `agorg_scope` for scope enforcement, but strict CLI payload schema for `pilot.multi.*` does not accept that field.
- Recovery:
  1. Use build where local execution sanitizes UI-only control fields before invoking CLI parser.
 2. Retry Multi register; expected result includes:
     - `"execution_mode": "local_direct"`

## G-049: Settings APIs return mixed JSON shapes (`ok` absent / arrays), causing false UI errors

- Signature:
  - Settings panel shows:
    - `Error: Failed to load exceptions`
  - even when backend returned valid exception list array.
- Cause:
  - `fetchJsonSafe` does not normalize HTTP status into `ok`.
  - Some settings endpoints return object payloads without `ok`, others return bare arrays.
  - UI code was treating anything without `ok: true` as failure.
- Recovery:
  1. Use build where settings handlers treat only `ok:false` or explicit `error` as failure.
  2. Accept valid array payload for exceptions endpoint.
  3. On malformed payload, surface explicit `malformed response payload` error.

## G-018: Widespread compilation failure after `evaluate_*_policy` signature changes

- Signature:
  - `error[E0061]: this function takes 7 arguments but 4 arguments were supplied`
  - impacts `serve_ui.rs` (API routes) and `main.rs` (CLI preview/scan).
- Cause:
  - Adding traceback or context metadata (like `policy_source_id` or `current_ago_path`) to the core evaluation engine (`eval.rs`) breaks loosely coupled call sites in the UI presentation layer and CLI.
- Recovery:
  1. Use `cargo check -p pilot` early when modifying `eval.rs`.
  2. Map all call sites in `serve_ui.rs` (in `branch_policy_violation` and `api_branch_run`).
  3. Map all call sites in `main.rs` (in `PolicyCommands::Preview` and `PolicyCommands::Scan`).
  4. Ensure arguments like `(&policy, path, &exceptions, &current_ago_path, &source_name, source_id)` are correctly passed to match the updated AST.

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
  - `cargo check` does NOT validate JavaScript. The Rust code compiles without error even when the embedded JS is syntactically broken.
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

## G-017: "Feature complete" claim but governance paths still stubbed

- Signature:
  - new CLI/API routes exist, but behaviors are static/non-contextual:
    - compliance scan returns constant zero counts
    - policy resolve ignores `repo_path`
    - CLI preview/scan/decisions output placeholder payloads
- Cause:
  - route/command scaffolding was merged without full integration to governance store + repo status evaluation.
- Detection:
  1. Run:
     - `cargo check -p pilot --locked`
     - `cargo test -p pilot --locked`
     - `node -c crates/pilot/src/pilot_ui.js`
  2. Spot-check runtime behavior:
     - `pilot policy scan --kind branch`
     - `pilot policy resolve --kind branch --repo-path <abs-path>`
     - `curl -sS http://127.0.0.1:7788/api/settings/compliance_scan -X POST ...`
  3. Fail if outputs remain static regardless of scope/repo state.
- Recovery:
  1. Wire handlers to real governance store/evaluator flows.
  2. Ensure resolve uses canonical `repo_path` override lookup before AGOrg fallback.
  3. Ensure scan/simulate evaluate current registry branch status under AGOrg scope.
  4. Re-run full evidence commands and record outputs in release evidence.

## G-018: Reconcile Apply blocked in UI with "read-only mode"

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

## G-019: Acceptance Matrix API returns 500 with "output was not valid JSON"

- Signature:
  - `POST /api/system/acceptance_matrix/run` returns:
    - `{"error":"acceptance matrix output was not valid JSON ...","ok":false}`
- Cause:
  - Script output may include non-JSON prefix lines before JSON payload.
  - Strict parse of full `stdout` fails even though trailing body is valid JSON.
- Recovery:
  1. Ensure parser uses mixed-output extraction path (`parse_json_from_mixed_output`).
  2. Re-run matrix:
     - `./scripts/wave_acceptance_matrix.sh --wave I --profile full`
  3. Confirm:
     - `ok=true`
     - `failed_checks=[]`

## G-020: Acceptance matrix/gate appears hung due concurrent stale runs

- Signature:
  - UI/API matrix call appears to hang indefinitely.
  - Multiple stale `prepush_gate.sh` / matrix processes are still active.
- Cause:
  - Overlapping matrix/gate invocations contend on shared resources and lock state.
- Recovery:
  1. Keep a single `pilot serve` instance active.
  2. Ensure only one matrix/gate run is active at a time.
  3. If needed, stop stale runs, then re-run one clean command:
     - `./scripts/wave_acceptance_matrix.sh --wave I --profile full`

## G-021: AGOrg looks clean but policy drift still exists

- Signature:
  - Discovery/import succeeds, but downstream operations still fail scope expectations.
  - Hidden issues include missing `pyproject.toml` or legacy off-policy entries.
- Recovery:
  1. Run:
     - `./scripts/pilot_local.sh agorg reconcile --agorg Arqon`
  2. Fix reported issues (or prune/reimport for off-policy paths).
  3. Re-run reconcile until issue count is acceptable.

## G-022: Scope guard rejects command as unscoped

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

## G-023: Live Event Stream shows `agorg_scope: null`

- Signature:
  - SSE telemetry payloads include `agorg_scope: null`.
- Cause:
  - No active AGOrg is selected, or event source emitted outside scoped command flows.
- Recovery:
 1. Set active AGOrg:
     - `./scripts/pilot_local.sh agorg use <id-or-name>`
 2. Confirm:
     - `./scripts/pilot_local.sh agorg show`

## G-024: Two UI windows show different active AGOrg/tab state

- Signature:
  - Separate UI windows disagree on active scope or restored tab/context.
- Cause:
  - State is isolated per `ui_instance_id` by design (Wave D).
- Recovery:
  1. Start with explicit instance id:
     - `pilot serve --ui-port 7788 --ui-instance-id pilot-main ...`
  2. Use same id to share state across windows, different ids to isolate intentionally.

## G-025: Temporary component state is unclear during incident triage

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

## G-026: `pilot serve` panics with settings exceptions route conflict

- Signature:
  - Startup panic from Axum route registration:
    - `Invalid route "/api/settings/exceptions/:id": insertion failed due to conflict with previously registered route: /api/settings/exceptions/:kind`
- Cause:
  - Dynamic route params names do not distinguish route shape in Axum.
  - Registering both `POST /api/settings/exceptions/:kind` and `POST /api/settings/exceptions/:id` causes a conflict.
- Recovery:
  1. Use non-conflicting delete route:
     - `POST /api/settings/exceptions/delete/:id`
  2. Keep add/list route unchanged:
     - `GET /api/settings/exceptions/:kind`
     - `POST /api/settings/exceptions/:kind`
  3. Rebuild/restart:
     - `cargo build -p pilot --locked`
     - `cargo run -p pilot -- serve ...`
- Prevention:
  - For Axum route design, treat `:param` as wildcard shape only; avoid sibling routes that differ only by param name.

## G-027: Policy tests flake from shared/long `PILOT_HOME` socket paths

- Signature:
  - Policy test commands fail with:
    - `FATAL: role "<user>" does not exist` (reused stale DB path)
    - `Unix-domain socket path ... is too long (maximum 107 bytes)`
    - `could not bind Unix address ... Operation not permitted`
- Cause:
  - Shared static `/tmp/pilotdb_*` paths leaked state across test runs.
  - Long nested test paths exceeded PostgreSQL unix socket path limits.
  - Some runtime/sandbox environments deny unix socket binding for test DB startup.
- Recovery:
  1. Use per-test unique, short `PILOT_HOME` paths under `/tmp/pilotdb_<suffix>`.
  2. Add explicit runtime-denial skip checks in tests for:
     - `Operation not permitted`
     - shared-memory denial signatures
     - unix socket bind/create failures.
  3. Re-run targeted tests:
     - `cargo test -p pilot --locked --test policy_adversarial_test -- --nocapture`
     - `cargo test -p pilot --locked --test policy_parity_integration_test -- --nocapture`
     - `cargo test -p pilot --locked --test policy_workflow_e2e_test -- --nocapture`

## G-028: Release bundle verify fails from missing/invalid bundle path

- Signature:
  - Release routine verify step fails with:
    - `bundle_path is required for release-verify-bundle`
    - `bundle_path contains unsupported characters`
    - `verify_bundle.sh: No such file or directory`
- Cause:
  - Verify step ran before collect-evidence filled bundle path.
  - Manual path value used unsupported shell characters.
  - Bundle directory does not contain generated `verify_bundle.sh`.
- Recovery:
  1. Run collect step first:
     - `release-collect-evidence` (UI) or
     - `./scripts/release_collect_evidence.sh --label <label>`
  2. Confirm bundle path and script:
     - `<bundle_path>/verify_bundle.sh`
  3. Re-run verify:
     - `release-verify-bundle` (UI) or
     - `<bundle_path>/verify_bundle.sh`

## G-029: CI watch fails because `gh` is unavailable or not authenticated

- Signature:
  - CI step fails with:
    - `GitHub CLI (gh) is not installed`
    - `gh authentication is not configured`
    - `no workflow run found for branch`
- Cause:
  - Local operator environment missing `gh`, missing auth, or monitoring wrong branch.
- Recovery:
  1. Install and authenticate:
     - `gh auth login`
     - `gh auth status -h github.com`
  2. Verify latest run exists for target branch:
     - `gh run list --branch <branch> --limit 5`
  3. Re-run CI watch:
     - `./scripts/gh_actions_watch_latest.sh --branch <branch> --timeout-sec 1800`

## Frozen Policy (Do Not Change)

- Core Rust lane: `1.82.0`
- Packaging Rust lane: `1.88.0`
- Protobuf: `4.25.8` / `protoc 25.8`
- Source of truth: `scripts/frozen_versions.sh`
