# ArqonBus Integration Guide

Arqon Pilot can run as an ArqonBus command bridge so external tools (including UI and AI agents)
can issue namespaced Pilot commands and receive structured telemetry.

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

Run one message and exit:

```bash
pilot serve --once
```

## Environment Variables

- `ARQONBUS_WS_URL`: default WebSocket URL for ArqonBus.
- `ARQONBUS_AUTH_JWT`: JWT used by `authenticate`.
- `PILOT_BUS_ROOM`: default room used by `pilot serve`.

## Currently Supported Bus Commands

- `pilot.branch.create`
- `pilot.branch.sync`
- `pilot.branch.status`
- `pilot.multi.register`
- `pilot.multi.list`
- `pilot.multi.status`

## Payload Shapes

### `pilot.branch.create`

```json
{
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
