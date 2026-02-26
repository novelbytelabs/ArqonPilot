# ArqonBus Integration Guide

Arqon Pilot can run as an Arqon Bus command bridge so external tools (including UI and AI agents) can issue namespaced Pilot commands and receive structured telemetry.

## What `pilot serve` Does

- Connects to ArqonBus over WebSocket.
- Optionally authenticates using a JWT from an environment variable.
- Joins a control channel for inbound commands.
- Emits lifecycle events to a telemetry channel:
    - `pilot.op.started`
    - `pilot.op.completed`
    - `pilot.op.failed`

## Command

```bash
pilot serve \
  --ws-url ws://127.0.0.1:9100 \
  --room pilot \
  --channel control \
  --telemetry-channel telemetry
```

Run with local control-panel UI:

```bash
pilot serve \
  --ws-url ws://127.0.0.1:9100 \
  --room pilot \
  --channel control \
  --telemetry-channel telemetry \
  --ui-port 7788
```

Safer default UI mode (recommended):
- UI/API mutation commands are blocked unless `--ui-allow-mutations` is set.
- You can restrict commands further with:
  - `--ui-allow-command pilot.branch.status`
  - `--ui-allow-command pilot.multi.status`

UI endpoints:
- `GET /` control panel
- `POST /api/command` execute `pilot.*` command over ArqonBus
- `GET /api/history` recent audit history
- `GET /api/reports` list recent `~/.pilot/reports` artifacts
- `GET /api/report?path=...` read one report file (bounded, path-validated)
- `GET /api/stream` live telemetry stream (SSE)

The Telemetry tab includes:
- raw live event stream
- operations timeline grouped by `operation_id`
- timeline filters: failed-only, command contains, and text search (op id/summary)
- stream controls: pause/resume
- export: filtered timeline JSON
- bus status chip: connected/disconnected
- operation detail drill-down: click timeline item for full payload and artifact hint

The Oracle tab includes:
- `pilot.oracle.scan` trigger
- `pilot.oracle.query` interactive query
- report browser/viewer for `~/.pilot/reports`

Run one message and exit:

```bash
pilot serve --once
```

## Environment Variables

- `ARQONBUS_WS_URL`: default WebSocket URL for ArqonBus.
- `ARQONBUS_AUTH_JWT`: JWT used by `authenticate`.
- `PILOT_BUS_ROOM`: default room used by `pilot serve`.

## Critical Linux/Conda Runtime Step

If `pilot` fails with `libssl-*.so.10` / `libcrypto-*.so.10` missing, configure conda hooks
so the runtime can find packaged shared libraries.

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

Do not put this in global `.bashrc`; keep it scoped to the target conda environment.

## Currently Supported Bus Commands

- `pilot.branch.create`
- `pilot.branch.sync`
- `pilot.branch.status`
- `pilot.branch.prune`
- `pilot.multi.register`
- `pilot.multi.list`
- `pilot.multi.status`
- `pilot.multi.order`
- `pilot.multi.prs.create`
- `pilot.oracle.scan`
- `pilot.oracle.query`

## Contract Rules (Strict v1)

- Every `pilot.*` command payload must include `"schema_version": 1`.
- Unknown fields are rejected.
- Wrong schema versions are rejected with a clear error response.

## Payload Shapes

### `pilot.branch.create`

```json
{
  "schema_version": 1,
  "branch": "feat/pilot-wave7",
  "base_branch": "main",
  "group": "core",
  "tags": ["apply-pilot"],
  "dry_run": true
}
```

### `pilot.multi.register`

```json
{
  "schema_version": 1,
  "path": "/home/irbsurfer/Projects/arqon/ArqonContinuum",
  "name": "ArqonContinuum",
  "group": "core",
  "tags": ["apply-pilot"]
}
```

## Safety Defaults

- `pilot.branch.create` and `pilot.branch.sync` default to `dry_run=true` unless explicitly set.
- Unsupported commands are rejected with an error response.
- Existing CLI behavior is unchanged; `pilot serve` is additive.
