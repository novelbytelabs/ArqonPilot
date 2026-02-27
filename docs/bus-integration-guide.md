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
- `POST /api/dependencies/run` run dependency guardrail actions (`policy`, `hook-policy`, `gate`, `repair`, `push`)
- `GET /api/dependencies/logs` read recent pre-push gate logs
- `POST /api/evidence/export` snapshot policy/history/reports/gate logs into an evidence bundle
- `POST /api/codex/action` contract-driven lifecycle API (`preview`, `approve`, `execute`, `reconcile`) for `pilot.*` commands
- `GET /api/codex/contracts` list persisted contract records (optional `status`, `limit`)
- `GET /api/codex/contract?contract_id=...` fetch one contract for resume/replay
- `GET /api/stream` live telemetry stream (SSE)

Control model:
- Dashboard is central command and can run Oracle/Heal/Dependencies/Branch/Multi actions.
- Specialist tabs provide deep controls, but day-to-day operations can be executed from Dashboard.

Dashboard telemetry includes:
- raw live event stream
- operations timeline grouped by `operation_id`
- timeline filters: failed-only, command contains, and text search (op id/summary)
- stream controls: pause/resume
- export: filtered timeline JSON
- bus status chip: connected/disconnected
- operation detail drill-down: click timeline item for full payload and artifact hint
- Live Event Stream is pinned to the bottom of Dashboard for persistent monitoring.

Telemetry tab includes:
- mirrored tail view for quick telemetry inspection

The Oracle tab includes:
- `pilot.oracle.scan` trigger
- `pilot.oracle.query` interactive query
- report browser/viewer for `~/.pilot/reports`

The Heal tab includes:
- `pilot.heal.run` controls (`log_file`, `max_attempts`, `target`, `verbose`, `plan_only`, `max_files`)
- safe default behavior in read-only UI mode (`plan_only=true` is enforced)

The Dependencies tab includes:
- policy check trigger
- hook policy check trigger
- pre-push gate trigger
- lock repair trigger (`repair_lock_182.sh --no-gate`, mutations required)
- safe push trigger (`push_main.sh <branch> <remote>`)
- recent gate-log viewer
- status cards powered by script `--json` outputs for policy/hook checks

The Multi tab includes:
- `DAG`: dependency graph + stage preview (`pilot.multi.dag` dry-run)
- `Staged Apply (Dry Run / Execute)`: dependency-aware staged branch orchestration (`pilot.multi.apply`)

System Status panel controls:
- `Policy`: runs frozen-policy verification.
- `Hook Policy`: validates local pre-push hook policy.
- `Drift`: runs lock drift diagnostics.
- `Gate`: runs mandatory pre-push gate.
- `Repair`: runs lock repair workflow.
- `Push Safe`: runs guarded push with classification summary.
- `Start Bus` / `Stop Bus` / `Bus Status`: manage local ArqonBus shim lifecycle.
- `Export Evidence`: write evidence bundle to `~/.pilot/reports/evidence_bundle_<timestamp>.json`.

Codex contract tab:
- `Preview Contract`: validate and normalize a command contract without executing.
- `Approve Contract`: lock in a previewed contract before execution.
- `Execute Contract`: run an approved contract (subject to UI mutation and allowlist policy) with telemetry events.
- `Reconcile Contract`: capture post-execution verification and notes for auditable closure.
- `Contracts (Resume / Replay)`: load persisted contracts, inspect state, and retry failed contracts (`approve -> execute`).
- Contract persistence file: `~/.pilot/reports/codex_contracts.jsonl` (restored at UI start).

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

Validation command after hook setup:

```bash
pilot --help
pilot serve --help
```

## Currently Supported Bus Commands

- `pilot.branch.create`
- `pilot.branch.sync`
- `pilot.branch.status`
- `pilot.branch.prune`
- `pilot.multi.register`
- `pilot.multi.list`
- `pilot.multi.status`
- `pilot.multi.order`
- `pilot.multi.dag`
- `pilot.multi.apply`
- `pilot.multi.prs.create`
- `pilot.oracle.scan`
- `pilot.oracle.query`
- `pilot.heal.run`

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

### `pilot.heal.run`

```json
{
  "schema_version": 1,
  "log_file": "test_output.json",
  "max_attempts": 2,
  "target": "crates/pilot/src/main.rs",
  "verbose": false,
  "plan_only": true,
  "max_files": 5
}
```

## Safety Defaults

- `pilot.branch.create` and `pilot.branch.sync` default to `dry_run=true` unless explicitly set.
- Unsupported commands are rejected with an error response.
- Existing CLI behavior is unchanged; `pilot serve` is additive.
