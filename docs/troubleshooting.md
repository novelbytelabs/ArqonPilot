# Troubleshooting

This page captures high-impact operational issues and exact fixes.

## 1) Linux/Conda: `libssl-*.so.10` or `libcrypto-*.so.10` not found

Symptom:

```bash
pilot: error while loading shared libraries: libssl-....so.10: cannot open shared object file
```

Cause: the `arqon-pilot` wheel bundles runtime libs in `site-packages/arqon_pilot.libs`, but the env runtime loader path does not include that directory.

Fix (recommended): add conda activation/deactivation hooks for the current environment only.

```bash
mkdir -p "$CONDA_PREFIX/etc/conda/activate.d" "$CONDA_PREFIX/etc/conda/deactivate.d"

cat > "$CONDA_PREFIX/etc/conda/activate.d/arqon_pilot_libs.sh" <<'EOF'
export _ARQONPILOT_OLD_LD_LIBRARY_PATH="${LD_LIBRARY_PATH-}"
export LD_LIBRARY_PATH="$CONDA_PREFIX/lib/python3.10/site-packages/arqon_pilot.libs:${LD_LIBRARY_PATH-}"
EOF

cat > "$CONDA_PREFIX/etc/conda/deactivate.d/arqon_pilot_libs.sh" <<'EOF'
export LD_LIBRARY_PATH="${_ARQONPILOT_OLD_LD_LIBRARY_PATH-}"
unset _ARQONPILOT_OLD_LD_LIBRARY_PATH
EOF
```

Then reactivate:

```bash
conda deactivate
conda activate helios-gpu-118
pilot --help
```

Important:
- Do not put this in global `.bashrc`.
- Keep it env-scoped.

## 2) `pilot serve` command missing

Symptom:
- `pilot --help` does not list `serve`.

Cause:
- You are running an older installed package (for example `arqon-pilot==0.1.1`) that predates this command.

Fix options:
1. Run local source directly:

```bash
cargo run -p pilot -- --help
cargo run -p pilot -- serve --help
```

2. Install an updated package version that includes `serve`.

## 3) ArqonBus reachable but no responses in UI

Checks:
1. Verify Bus URL:

```bash
echo "$ARQONBUS_WS_URL"
```

2. Confirm Pilot bridge and UI are running:

```bash
pilot serve --ws-url ws://127.0.0.1:9100 --room pilot --channel control --telemetry-channel telemetry --ui-port 7788
```

3. Open the UI:
- `http://127.0.0.1:7788`

4. If auth is enabled, ensure JWT exists:

```bash
echo "$ARQONBUS_AUTH_JWT" | wc -c
```

If bus shows `DISCONNECTED` and nothing is listening on `9100`, start the local compatibility shim
from ArqonPilot (no ArqonBus source changes required):

```bash
./scripts/arqonbus_shim.sh start
./scripts/arqonbus_shim.sh status
```

Then restart Pilot UI:

```bash
pilot serve --ws-url ws://127.0.0.1:9100 --room pilot --channel control --telemetry-channel telemetry --ui-port 7788
```

If needed:

```bash
./scripts/arqonbus_shim.sh logs
./scripts/arqonbus_shim.sh stop
```

If the bus disconnects after ~10-20 seconds:

1. Use tmp report path for shim logs/state:

```bash
PILOT_REPORT_DIR=/tmp/pilot-reports ./scripts/arqonbus_shim.sh start
PILOT_REPORT_DIR=/tmp/pilot-reports ./scripts/arqonbus_shim.sh status
```

2. In UI `System Status`, use:
- `Bus Status`
- `Start Bus`
- `Stop Bus`

3. Verify listener directly:

```bash
ss -ltnp | rg ':9100'
```

## 4) Command rejected by Bus bridge

The bridge currently only accepts namespaced `pilot.*` commands and strict contract payloads with:

```json
{ "schema_version": 1, ... }
```

Unknown fields and wrong schema versions are rejected by design.

## 5) Pre-push fails with `edition2024` dependency errors

Symptom (examples):

```text
feature `edition2024` is required
The package requires the Cargo feature called `edition2024`
```

or

```text
error: failed to parse manifest ... comfy-table-7.2.2/Cargo.toml
```

Cause:
- `Cargo.lock` drifted to dependencies requiring Rust 1.85+ while core lane is pinned to Rust/Cargo `1.82.0`.

Canonical recovery flow:

```bash
./scripts/repair_lock_182.sh --no-gate
./scripts/prepush_gate.sh
git push origin main
```

Why this works:
1. `repair_lock_182.sh` restores or force-pins Rust-1.82-compatible lock state.
2. `prepush_gate.sh` validates policy + locked compile + targeted locked tests.
3. Push proceeds only after the gate passes.

If push still fails after gate success:
- Treat it as a normal git transport/state issue first (for example non-fast-forward or auth), not a guardrail issue.
- Run:

```bash
git status -sb
git fetch origin
git pull --rebase origin main
git push origin main:main
```

Gotchas:
1. Ambiguous package names during pinning:
- Use exact package IDs: `name@from_version` (for example `getrandom@0.4.1`).

2. Transitive chain issues:
- A crate can reintroduce Rust-2024 dependencies indirectly (`uuid -> getrandom -> wasip3 -> wit-bindgen`).
- Another known drift source is `constant_time_eq 0.4.x` (also requires `edition2024`).
- `constant_time_eq` cannot always be pinned directly; in some lock states it is constrained by `blake3` (`^0.4.2`), so `blake3` must be downgraded first.
- Another known drift source is `globset 0.4.18+` (requires `edition2024`).

3. Logging location:
- Gate log is written to `~/.pilot/reports/`.
- If that directory is not writable, gate falls back to `/tmp/pilot-reports/`.

4. Packaging lane is separate:
- PyPI packaging lane can use newer toolchain and `Cargo.lock.packaging`.
- Core lane still must satisfy Rust `1.82.0` with `Cargo.lock`.
- Frozen versions are enforced by policy checks:
  - core Rust `1.82.0`
  - packaging Rust `1.88.0`
  - protobuf `4.25.8` (`protoc` `25.8`)

5. Packaging lock drift:
- If CI fails with `Cargo.lock needs to be updated but --locked was passed` in `packaging-parity`,
  `Cargo.lock.packaging` is stale versus current manifests.
- Fix by re-syncing packaging lock after dependency changes:

```bash
cp Cargo.lock Cargo.lock.packaging
./scripts/packaging_lane_check.sh
```

6. Missing `rg` on CI runner:
- If `release_readiness_check.sh` fails with `rg: command not found`, use scripts that include
  `grep` fallback (current default). Do not assume `rg` exists on every runner image.

## 6) Pre-push fails due transient DNS/crates.io access

Symptoms (examples):

```text
Could not resolve host: index.crates.io
Temporary failure in name resolution
failed to download from https://index.crates.io
```

What ArqonPilot now does:
1. `prepush_gate.sh` retries `cargo check`, `cargo test`, and help-surface checks up to 3 times.
2. It prints DNS diagnostics (`getent hosts index.crates.io` and `static.crates.io`) in the gate log on transient network failures.

Manual checks:

```bash
getent hosts index.crates.io
getent hosts static.crates.io
```

If DNS checks fail locally, fix network/DNS first and rerun:

```bash
./scripts/prepush_gate.sh
```

If DNS checks pass but policy fails, run lock recovery:

```bash
./scripts/repair_lock_182.sh --no-gate
./scripts/prepush_gate.sh
```

## 7) VS Code shows generic push failure with no useful reason

Symptom:

```text
git push origin main:main
error: failed to push some refs to 'https://github.com/...'
```

Use the diagnostic wrapper instead of raw push:

```bash
./scripts/push_main.sh
```

The summary now includes:
1. classified cause (`prepush_gate_failed`, `auth_or_token`, `non_fast_forward_or_remote_ahead`, `dns_or_name_resolution`, etc.)
2. explicit remediation steps based on that classification.
3. `auth_challenge_events` for expected GitHub HTTPS auth handshakes (`HTTP/2 401` pre-credential retry).

## 8) Drift diagnosis from Dependencies tab

Use `Drift Report` in Dashboard/Dependencies to scan `Cargo.lock` for known frozen-lane drift families:

- `time/comfy-table/wit-bindgen`
- `blake3/constant_time_eq`
- `globset`
- `icu_* 2.1.x`

CLI equivalent:

```bash
./scripts/drift_report.sh
./scripts/drift_report.sh --json
```

What it does:
1. Fetches remote state.
2. Runs the mandatory pre-push gate.
3. Runs push with `GIT_TRACE=1` and `GIT_CURL_VERBOSE=1`.
4. Writes a full log to `~/.pilot/reports/push_main_<timestamp>.log` (fallback: `/tmp/pilot-reports/`).
5. Prints a final compact summary: result, duration, gate/push return codes, warning/error counts, divergence state, likely cause, and full log path.

Defaults:
- Push target defaults to your current checked-out branch.
- To push `main` explicitly: `./scripts/push_main.sh main`.

Interpretation note:
- `auth_challenge_events > 0` with `git_push_rc=0` and `result=SUCCESS` is expected behavior and not an error condition.

## 9) PyPI workflow succeeded but new version is not visible

Symptom:
- GitHub Actions publish job reports success, but `pip install arqon-pilot==<new_version>` fails.
- PyPI JSON still shows an older latest version.

Verify directly:

```bash
python3 - <<'PY'
import json, urllib.request
data = json.load(urllib.request.urlopen("https://pypi.org/pypi/arqon-pilot/json", timeout=20))
print("latest:", data["info"]["version"])
print("releases:", sorted(data["releases"].keys()))
PY
```

Checks:
1. Confirm workflow actually uploaded expected files/version (not `skip-existing` on older version).
2. Confirm publish target was `pypi` (not `testpypi`).
3. Confirm package version in source (`pyproject.toml`) matches intended release.
4. Retry install after short propagation delay and clear pip cache:

```bash
python -m pip install --no-cache-dir --index-url https://pypi.org/simple arqon-pilot==<new_version>
```

If still not visible:
- Treat as publish-index mismatch incident and record in `docs/gotcha-registry.md`.
- Re-run publish with corrected versioning and explicit verification step.

## 10) Codex contract action fails from UI

Symptom:
- `Codex` tab returns validation error or `400` response.

Common causes:
1. `intent` is empty.
2. `command` is not `pilot.*`.
3. payload JSON is invalid.
4. mutating command attempted in read-only UI mode.

Recovery:
1. Use `Preview Contract` first.
2. Ensure payload is valid JSON object.
3. For mutation execution, run `pilot serve ... --ui-allow-mutations`.
4. If allowlist is configured, ensure command is in `--ui-allow-command`.
