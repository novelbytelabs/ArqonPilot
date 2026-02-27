# ArqonPilot Operator Runbook

This runbook defines the standard operating procedure for running Arqon Pilot across single-repo and multi-repo workflows.

## 1. Environment Baseline

- Rust toolchain pinned at `1.82.0` (`rust-toolchain.toml`).
- Packaging lane Rust pinned at `1.88.0` (PyPI workflow only).
- Protobuf/protoc pinned at `4.25.8` / `25.8`.
- Pilot binary built from current repo:
    - `cargo build -p pilot`
- Workspace state path:
    - `~/.pilot/workspace.db`
    - `~/.pilot/audit.jsonl`
    - `~/.pilot/reports/*.json`

## 2. Daily Safe Workflow (Dry-Run First)

1. Registry and health

    - `pilot multi list`
    - `pilot multi status`

2. Dependency order and branch planning

    - `pilot multi order --group <group>`
    - `pilot multi dag --group <group> --dry-run`
    - `pilot branch create feat/<name> --group <group> --base-branch dev --dry-run`
    - `pilot branch sync --branch feat/<name> --group <group> --dry-run`
    - `pilot multi apply --branch feat/<name> --group <group> --stage-size 2` (dry-run default)

3. Release planning
    - `pilot multi prs create --group <group> --dry-run`
    - `pilot navigate --multi --group <group> --dry-run`

4. Security and maintenance
    - `pilot secure scan --group <group>`
    - `pilot secure fix --group <group>` (dry-run default)

5. Cross-repo staged orchestration

    - Dry-run staged orchestration:
      - `pilot multi apply --branch feat/<name> --group <group> --stage-size 2`
    - Execute staged orchestration:
      - `pilot multi apply --branch feat/<name> --group <group> --stage-size 2 --apply`
    - Optional failure behavior:
      - add `--continue-on-failure` to continue later batches/stages.

5. Planning and knowledge loop
  
    - `pilot plan issues --input <issues.json>`
    - `pilot plan score`
    - `pilot plan roadmap`
    - `pilot create feature <name> --dry-run` 
    - `pilot know record --title ... --context ... --decision ...`

## 3. Dashboard-First Operational Flow

Use the UI (`pilot serve --ui-port 7788`) as the primary control surface.

1. `Dashboard -> System Status`

    - Run `Policy`, `Hook Policy`, and `Gate` before branch/release operations.
    - Use `Push Safe` instead of raw push for root-cause summaries.
    - In `Push Safe` summary, `auth_challenge_events` counts expected GitHub HTTPS auth handshakes (`HTTP/2 401` before credential retry); this is informational, not a failure.
    - Use `Start Bus` / `Stop Bus` / `Bus Status` for ArqonBus control.
    - Use `Export Evidence` to snapshot policy status, recent audit history, report index, and gate-log tails into `~/.pilot/reports/evidence_bundle_<timestamp>.json`.

2. `Dashboard` quick cards

    - Oracle + Heal quick actions for fast triage and plan/repair loops.
    - Branch + Multi quick actions for cohort branch and status operations.
    - Follow the in-panel `Recommended Sequence` strips:
      - Dashboard: `Status -> Bus Health -> Oracle Query -> Heal Plan -> Heal Run -> Push Safe`
      - Oracle: `Scan Index -> Run Query -> Open Report`
      - Heal: `Plan Only -> Review Response/Timeline -> Run Heal`
      - Multi: `Register -> List/Status/Order -> DAG/PR Plan -> Staged Apply`

3. `Operations Timeline` + `Operation Detail`

    - Filter failures, inspect payloads, and use artifact paths for one-click debugging.

4. `Live Event Stream`

    - Pinned at the bottom of Dashboard for long-running monitoring without losing context.

6. UI/API smoke verification (post-change)

    - `./scripts/ui_smoke_check.sh`
    - Starts/uses ArqonBus shim, starts `pilot serve`, and validates key UI/API paths:
      - dashboard HTML load
      - history/report/log endpoints
      - dependency action endpoint
    - Optional full command-lane checks:
      - `PILOT_UI_SMOKE_INCLUDE_COMMANDS=1 ./scripts/ui_smoke_check.sh`
      - includes `pilot.multi.status`, `pilot.multi.dag` (dry-run), and `pilot.multi.apply` (dry-run) via `/api/command`

5. `Codex` tab (contract-driven operations)

    - `Preview Contract` builds a normalized action contract (intent, command, payload, expected effect, rollback strategy) and generates `contract_id`.
    - `Approve Contract` transitions contract to executable state.
    - `Execute Contract` runs the approved contract through the Bus bridge with telemetry and response capture.
    - `Reconcile Contract` records verification outcome and closure notes.
    - Use `verify_command` to define post-action verification intent in the contract record.
    - Use `Contracts (Resume / Replay)` to reload prior contracts, inspect status, and retry failed contracts.
    - Contract history persists at `~/.pilot/reports/codex_contracts.jsonl` and is restored when UI restarts.

6. `AGOrg` tab (Wave 16 foundation)

    - Use `Create AGOrg Project` to define scope + autoscan hierarchy.
    - Use `List AGOrgs` and `Show Active` to verify current scope.
    - Use `Use Scope` to switch Control Panel context.
    - Use `Discover` to scan a root path with configurable depth.
    - Use `Tree` to inspect AGOrg/AGO graph structure.
    - Use `Link` for modular AGOrg composition (cycle-safe enforcement).
    - Use System Status `DB Status`, `DB Start`, `DB Stop` to control managed AGOrg datastore.

7. Managed DB operations (CLI)

    - `pilot db ensure`
    - `pilot db status`
    - `pilot db start`
    - `pilot db stop`

Managed DB defaults:
- data: `~/.arqon/pilot/db/data`
- logs: `~/.arqon/pilot/db/postgres.log`
- endpoint: Unix socket on Linux/macOS, local TCP fallback on Windows
- identity guard: Pilot refuses migration if DB identity is not `arqon_pilot`

## 4. Controlled Apply Workflow

Apply mode is allowed only for explicitly tagged pilot cohorts.

1. Pilot cohort registration

    - `pilot multi register --path <repo> --tag apply-pilot --tag wave7`

2. Preflight

    - repo clean, on expected base branch, CI green, credentials available.

3. Apply branch operation

    - `pilot branch create feat/<name> --tag apply-pilot --base-branch dev`

4. Apply secure fix (if desired)

   - `pilot secure fix --tag apply-pilot --apply`

5. Post-apply checks

    - `pilot branch status --tag apply-pilot`
    - repo tests and lint
    - inspect `~/.pilot/audit.jsonl` and `~/.pilot/reports/*.json`

## 5. Rollback Procedure

Preferred rollback:

- `git revert <bad_commit_sha>`

After rollback:

1. verify clean status
2. run tests
3. record incident and resolution via `pilot know record`

Avoid:

- history rewriting on shared branches (`reset --hard`, force-push) except emergency local recovery with explicit approval.

## 6. Incident Triage

When a multi-repo operation fails:

1. open latest report artifact from `~/.pilot/reports/`
2. isolate failing repo(s)
3. re-run command scoped to failed repo only
4. record root cause and corrective action in `pilot know`

## 7. Release Candidate Gate (v1-rc1)

Before cutting `pilot-v1-rc1`:

1. `cargo check -p pilot --locked`
2. full CLI suite green
3. wave acceptance docs updated
4. dogfooding + controlled apply evidence documented
5. audit/report artifacts present and interpretable
6. if publishing, verify index visibility:
    - `./scripts/verify_pypi_release.sh --index pypi`
