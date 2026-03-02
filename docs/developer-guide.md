# Developer Guide

ArqonPilot is the local control plane for Arqon's multi-repo ecosystem. It provides
monorepo-like developer flow (single control surface, coordinated actions) while keeping
repositories independent for security and lifecycle isolation.

## Canonical References

1. Unified roadmap/program plan: `docs/PRODUCTIONIZE.md`
2. AGOrg control-plane content is consolidated into: `docs/PRODUCTIONIZE.md`
3. Branch Control detailed historical plan: `archives/docs/plans/branch-control-master-plan.md`
4. Gotcha/failure registry: `docs/gotcha-registry.md`

## Prerequisites

- Rust toolchain pinned by `rust-toolchain.toml`
- `git`
- Python 3.10+ for packaging smoke checks
- Frozen policy file: `scripts/frozen_versions.sh`
  - core lane Rust: `1.82.0`
  - packaging lane Rust: `1.88.0`
  - protobuf: `4.25.8` (`protoc` `25.8`)

## Memory Anchors (Immutable for this project)

- Core lane is permanently frozen: Rust `1.82.0`.
- Packaging lane is permanently frozen: Rust `1.88.0`.
- Protobuf/protoc are permanently frozen: `4.25.8` / `25.8`.
- Never "fix" CI by bumping these pins. Repair lockfiles and lane parity instead.

## Build

```bash
cargo check -p pilot --locked
```

## Run

```bash
cargo run -p pilot -- --help
```

If your shell `pilot` points to an older installed binary, use the local wrapper to force the repo build:

```bash
./scripts/pilot_local.sh --help
./scripts/pilot_local.sh db status
```

## ArqonBus Bridge

Use `pilot serve` to expose Branch and Multi operations through ArqonBus command lanes:

```bash
pilot serve --ws-url ws://127.0.0.1:9100 --room pilot --channel control --telemetry-channel telemetry
```

Canonical full-control launch command (repo-local build, mutations enabled):

```bash
cargo run -p pilot -- serve --ws-url ws://127.0.0.1:9100 --room pilot --channel control --telemetry-channel telemetry --ui-port 7788 --ui-allow-mutations
```

If ArqonBus is frozen and its default module entrypoint is incompatible in your checkout,
start the compatibility shim from this repo instead of editing ArqonBus:

```bash
./scripts/arqonbus_shim.sh start
./scripts/arqonbus_shim.sh status
```

Then run Pilot UI:

```bash
pilot serve --ws-url ws://127.0.0.1:9100 --room pilot --channel control --telemetry-channel telemetry --ui-port 7788
```

Shim controls:

```bash
./scripts/arqonbus_shim.sh stop
./scripts/arqonbus_shim.sh logs
```

Control Panel System Status actions now include:
- `Start Bus`
- `Stop Bus`
- `Bus Status`
- `Drift` (known dependency drift families)

JWT auth is optional and read from `ARQONBUS_AUTH_JWT` by default.

For a local operator panel (Oracle, Heal, Dependencies, Branch, Multi, Telemetry), run:

```bash
pilot serve --ws-url ws://127.0.0.1:9100 --room pilot --channel control --telemetry-channel telemetry --ui-port 7788
```

Safety flags:

```bash
# allow mutations from UI/API only when explicitly intended
pilot serve ... --ui-port 7788 --ui-allow-mutations

# optional allowlist for UI/API commands
pilot serve ... --ui-port 7788 --ui-allow-command pilot.branch.status --ui-allow-command pilot.multi.status
```

## AGOrg Foundation (Wave 16)

AGOrg state now uses a Pilot-managed isolated local Postgres runtime by default.

Managed runtime defaults:
- Pilot home: `~/.arqon/pilot/`
- Data dir: `~/.arqon/pilot/db/data`
- Runtime dir: `~/.arqon/pilot/run`
- Log file: `~/.arqon/pilot/db/postgres.log`
- DB name: `pilot_local`
- Endpoint mode:
  - Linux/macOS: Unix socket directory (`~/.arqon/pilot/run`) with socket file `.s.PGSQL.9132`
  - Windows: local TCP (`127.0.0.1:9132` default with deterministic fallback)

Managed DB commands:

```bash
./scripts/pilot_local.sh db ensure
./scripts/pilot_local.sh db status
./scripts/pilot_local.sh db start
./scripts/pilot_local.sh db stop
```

Safety guard:
- Pilot writes/validates a DB identity marker (`pilot_identity.system = arqon_pilot`).
- If identity mismatches, schema migration is refused.

Advanced override (disables managed runtime auto-start):
- `PILOT_AGORG_DATABASE_URL`

Core AGOrg commands:

```bash
./scripts/pilot_local.sh agorg create --name Arqon --root /home/irbsurfer/Projects/arqon/Arqon --default-scope
./scripts/pilot_local.sh agorg list
./scripts/pilot_local.sh agorg show
./scripts/pilot_local.sh agorg use Arqon
./scripts/pilot_local.sh agorg discover --root /home/irbsurfer/Projects/arqon --depth 4
./scripts/pilot_local.sh agorg discover --root /home/irbsurfer/Projects/arqon --depth 4 --import-to Arqon --prune-missing
./scripts/pilot_local.sh agorg reconcile --agorg Arqon
./scripts/pilot_local.sh agorg tree
./scripts/pilot_local.sh agorg create-project --name Arqon --root /home/irbsurfer/Projects/arqon/Arqon --autoscan --import --prune-missing --default-scope
```

Discovery policy notes:
- Default discovery is flat-fleet: nested repos and `archive/` are skipped.
- To include nested repositories explicitly, set:
  - `PILOT_AGORG_ALLOW_NESTED_REPOS=1`
- Use `--prune-missing` during import to reconcile and remove stale AGO rows not present in the current discovery set.

UI review/import flow:
- In AGOrg panel (`Import Existing`):
  1. `Discover Preview` to fetch candidates.
  2. Approve/reject candidates in `Discovery Review`.
  3. `Refresh Reviews` / `Load Review` to resume a prior review session.
  4. `Import Approved` to apply selected AGO candidates only.
  5. Keep `prune stale AGO rows` enabled for deterministic reconciliation.
  6. Run `Policy Report` to surface off-policy entries and metadata gaps.
  7. Run `Reconcile Dry Run` to preview prune candidates.
  8. Run `Reconcile Apply` to execute approved reconciliation.
  9. Use `Refresh Policy Artifacts` to reload persisted reports.

Review artifact:
- persisted at `~/.pilot/reports/agorg_reviews.jsonl`
- each entry includes review id, selected approvals, and import summary (if applied).

Scope enforcement (Wave C in progress):
Scope enforcement (Wave C complete):
- UI command bridge now requires an active AGOrg for:
  - `pilot.branch.*`, `pilot.multi.*`, `pilot.oracle.*`, `pilot.heal.*`, `pilot.navigate.*`
- Repo-local command families (`branch`, `oracle`, `heal`, `navigate`) are blocked if current working directory is outside active AGOrg root.
- `pilot.multi.*` calls require explicit `group` or `tags` selector (unfiltered fleet calls are rejected).
- Dashboard dependency actions (`policy`, `hook-policy`, `drift`, `gate`, `repair`, `push`) are now AGOrg-scoped and require CWD within active AGOrg root.
- Service controls (`db-*`, `bus-*`, `services-*`) intentionally remain global so operators can recover infra before scope is set.
- Live Event Stream now emits `agorg_scope` context on each SSE event (or `null` if no scope is active) for consistent dashboard telemetry correlation.

AGOrg policy endpoints (Wave E foundation):
- `POST /api/agorg/policy_report` (returns `report` + persisted `artifact_path`)
- `GET /api/agorg/policy_reports?limit=50` (returns artifact list)
- `POST /api/agorg/reconcile_apply` (`dry_run=true|false`)

Wave D (profiles + multi-instance + restore):
- Per-AGOrg profile/preferences are persisted in AGOrg settings and editable from AGOrg panel:
  - profile name
  - default branch
  - release branch
  - auto-prune preference
- Scope snapshot endpoint provides fast startup/switch:
  - `GET /api/agorg/scope_snapshot`
- UI session state is persisted per UI instance:
  - `GET /api/ui/session`
  - `POST /api/ui/session`
- Multi-instance isolation:
  - `pilot serve --ui-port 7788 --ui-instance-id pilot-main`
  - each instance keeps independent active scope + session state.

## Critical Linux/Conda Runtime Step

If `pilot` fails with a shared-library error like `libssl-*.so.10` not found, add the
packaged runtime library directory to `LD_LIBRARY_PATH` via conda hooks.

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

Reactivate env and verify:

```bash
conda deactivate
conda activate helios-gpu-118
pilot --help
```

## Test

```bash
./scripts/test_matrix.sh all
```

## Mandatory Pre-Push Gate

Run this before every commit/push:

```bash
./scripts/prepush_gate.sh
```

This gate includes `cargo check -p pilot --locked` and targeted locked CLI tests.
To enforce it automatically on every `git push`, run once per clone:

```bash
./scripts/install_git_hooks.sh
```

CI validates the hook/gate contract with:

```bash
./scripts/verify_git_hook_policy.sh
```

If `Cargo.lock` drifts and pre-push fails with `edition2024` parser errors, run:

```bash
./scripts/repair_lock_182.sh
```

This restores a compatible core lockfile (or applies exact-version fallback transitions) and can re-run the gate.
After lock repair, always run lane parity:

```bash
./scripts/ci_parity_check.sh
```

If VS Code shows only a generic push failure message, run:

```bash
./scripts/push_main.sh
```

This captures verbose git transport diagnostics and writes a timestamped push log.
By default it pushes your current checked-out branch; pass an explicit branch when needed:
`./scripts/push_main.sh main`.

## Pre-Check Scripts Reference

These scripts are the required guardrail layer before commit/push:

1. `./scripts/prepush_gate.sh`
- Runs policy checks, locked compile, targeted locked CLI tests, and help-surface smoke check.
- Includes automatic retries for transient crates.io/DNS failures on cargo steps.
- Writes a timestamped log file to `~/.pilot/reports/` (or `/tmp/pilot-reports/` fallback).

2. `./scripts/verify_toolchain_policy.sh`
- Verifies Rust lane pins (`1.82.0` core, `1.88.0` packaging), lockfile policy wiring, and lockfile compatibility checks for core lane.
- Verifies PyPI workflow protobuf/protoc freeze pin (`4.25.8` / `25.8`) and rejects unpinned `protobuf-compiler` apt usage.
- Fails fast with explicit incompatible dependencies (for example `time 0.3.47`, `wit-bindgen 0.51.0`).
- Supports machine-readable mode: `./scripts/verify_toolchain_policy.sh --json`

3. `./scripts/verify_git_hook_policy.sh`
- Ensures `.githooks/pre-push` exists and calls `./scripts/prepush_gate.sh`.
- Ensures hook installer and mandatory locked compile gate are in place.
- Supports machine-readable mode: `./scripts/verify_git_hook_policy.sh --json`

4. `./scripts/install_git_hooks.sh`
- Sets `core.hooksPath=.githooks` so pushes run the gate automatically.

5. `./scripts/repair_lock_182.sh`
6. `./scripts/drift_report.sh`
- Recovery script for lockfile drift in Rust `1.82.0` lane.

6. `./scripts/packaging_lane_check.sh`
- Runs packaging-lane validation using Rust `1.88.0` and `Cargo.lock.packaging`.
- Temporarily swaps `Cargo.lock` with `Cargo.lock.packaging` and restores it automatically.

7. `./scripts/ci_parity_check.sh`
- Runs full lane parity checks locally (`1.82.0` core + `1.88.0` packaging).
- Use before merge when touching CI, lockfiles, or packaging logic.
- Attempts compatible lock restore from git history; falls back to exact-version pin transitions.

8. `./scripts/drift_report.sh`
- Scans `Cargo.lock` for known frozen-lane drift families.
- Text mode is human-readable and exits nonzero when drift is found.
- JSON mode (`--json`) is used by Dependencies UI for machine-readable diagnosis.

## Guardrail Gotchas

Canonical registry: `docs/gotcha-registry.md`

1. If `git push` fails before upload, the pre-push hook blocked it intentionally for safety.
2. `edition2024` parser errors indicate lockfile drift for Rust `1.82.0`, not necessarily source-code regressions.
3. `cargo update -p <crate>` can be ambiguous; use exact IDs like `getrandom@0.4.1`.
4. A green local build with newer toolchain does not guarantee core-lane compatibility.
5. `prepush_gate.sh` writes logs to `~/.pilot/reports/` and falls back to `/tmp/pilot-reports/` if needed.
6. Some crates must be downgraded via upstream constraints first (for example `blake3` before `constant_time_eq`).
7. If push still fails after gate passes, check for normal git remote-state errors (non-fast-forward/auth), not policy failures.

## Release Readiness

```bash
./scripts/release_readiness_check.sh
```

## Toolchain Drift Prevention

Arqon Pilot enforces a dual-lane policy:
- core dev/test lane: Rust `1.82.0` + `Cargo.lock`
- packaging lane: Rust `1.88.0` + `Cargo.lock.packaging`
- protobuf/protoc lane: `4.25.8` / `25.8` (pinned in `.github/workflows/pypi.yml`)

Validate policy locally:

```bash
./scripts/verify_toolchain_policy.sh
```

## Packaging Smoke

```bash
python3 -m pip install maturin
maturin build --release --locked --out dist
./scripts/pypi_smoke_check.sh
```

## Non-Destructive Operating Pattern

Use `--dry-run` for any mutating command first:

```bash
pilot branch create feat/x --group core --dry-run
pilot navigate --multi --group core --dry-run
pilot secure fix --group core --dry-run
```
