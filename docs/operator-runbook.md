# ArqonPilot Operator Runbook

This runbook defines the standard operating procedure for running Arqon Pilot across single-repo and multi-repo workflows.

## 1. Environment Baseline

- Rust toolchain pinned at `1.82.0` (`rust-toolchain.toml`).
- Packaging lane Rust pinned at `1.88.0` (PyPI workflow only).
- Protobuf/protoc pinned at `4.25.8` / `25.8`.
- Pilot binary built from current repo:
    - `cargo build -p pilot`
    - `./scripts/pilot_local.sh` for command execution without PATH ambiguity
- Workspace state path:
    - `~/.pilot/workspace.db`
    - `~/.pilot/audit.jsonl`
    - `~/.pilot/reports/*.json`

### Canonical Control Panel Start (Mutations Enabled)

Use this exact command when operating the full UI control plane:

```bash
cargo run -p pilot -- serve --ws-url ws://127.0.0.1:9100 --room pilot --channel control --telemetry-channel telemetry --ui-port 7788 --ui-allow-mutations
```

This is the standard operator launch command for local AGOrg/Branch/Dependencies actions from the UI.

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

6. Governance policy lifecycle (CLI)

   - `pilot policy list --kind branch [--ago-path <abs-path>] [--limit 25]`
   - `pilot policy get --kind branch`
   - `pilot policy set-draft --kind branch --file <policy.json>`
   - `pilot policy preview --kind branch --version <n>`
   - `pilot policy approve --kind branch --version <n> --simulation-artifact <path>`
   - `pilot policy activate --kind branch --version <n>`
   - `pilot policy delete --kind branch --version <n> [--ago-path <abs-path>]`
   - `pilot policy resolve --kind branch --repo-path <abs-path>`
   - `pilot policy scan --kind branch [--group <g>] [--tag <t>]`
   - `pilot policy exceptions list --kind branch`
   - `pilot policy decisions --kind branch --limit 100`

## 2.1 Pilot-for-Pilot Enforced Routine (ArqonPilot Repo)

This routine is now enforced by `./scripts/prepush_gate.sh` before toolchain/test checks.

Policy intent:

1. ArqonPilot pushes must originate from the correct repo boundary.
2. Pilot UI scope must be active before push.
3. Current repo (`ArqonPilot`) must be registered as an AGO in the active AGOrg.

Enforcement point:

1. `./scripts/pilot_discipline_gate.sh` (called as step `[0/4]` in `prepush_gate.sh`).

Standard local routine:

1. Start control plane:
   - `cargo run -p pilot -- serve --ws-url ws://127.0.0.1:9100 --room pilot --channel control --telemetry-channel telemetry --ui-port 7788 --ui-allow-mutations`
2. In UI, select active AGOrg in header chip (`AGOrg: ...`).
3. Confirm `ArqonPilot` appears as AGO under that AGOrg.
4. Run:
   - `./scripts/prepush_gate.sh`
   - `./scripts/push_main.sh`

Bypass controls (only for controlled emergencies):

1. Disable discipline gate for one run:
   - `PILOT_ENFORCE_AGORG_DISCIPLINE=0 ./scripts/prepush_gate.sh`
2. CI lane automatically skips discipline gate (`CI=true` path).

### Beginner Click-by-Click (No Assumptions)

If you are new, do this exactly in order:

1. Start UI:
   - `cargo run -p pilot -- serve --ws-url ws://127.0.0.1:9100 --room pilot --channel control --telemetry-channel telemetry --ui-port 7788 --ui-allow-mutations`
2. Open `http://127.0.0.1:7788`.
3. Click AGOrg chip (top-right), then confirm active scope is correct.
4. Click `Multi` tab, then in `Register Repo` fill:
   - Path: `/home/irbsurfer/Projects/arqon/ArqonPilot`
   - Name: `ArqonPilot`
   - Group: `core`
   - Tags: `apply-pilot,operator`
   - Click `Register`.
   - Expected success includes `"execution_mode": "local_direct"`.
5. Still in `Multi`, click `List` or `Status` and verify `ArqonPilot` appears.
6. Go to `Dashboard`, click in order:
   - `Policy` -> `Hook Policy` -> `Drift` -> `Gate`
7. Run gate from terminal:
   - `./scripts/prepush_gate.sh`

If anything fails, use the full beginner tutorial:
- `docs/pilot-for-pilot-tutorial.md`

If `Register` still shows a generic timeout:
- restart `pilot serve`,
- hard refresh browser,
- retry register.

## 3. Dashboard-First Operational Flow

Use the UI (`pilot serve --ui-port 7788`) as the primary control surface.

### Tab Interop Quick Reference (Dependencies/Branch/Multi)

| Task | Primary Tab | Secondary Tab | Why |
|---|---|---|---|
| Toolchain/hook/gate/drift diagnosis | Dependencies | Dashboard (System Status) | Dependencies is authoritative for policy/gate/push readiness. |
| Fleet branch create/sync/prune/status | Branch | Dashboard (shortcut only) | Branch is authoritative for branch lifecycle operations. |
| Dependency-aware staged branch apply | Branch | Multi | Branch owns branch mutation UX; Multi supports orchestration primitives. |
| DAG/order/PR plan without branch mutation | Multi | Branch | Multi is authoritative for non-branch orchestration planning. |
| Push-safe decision after branch operations | Dependencies | Branch | Dependencies computes gate/push readiness used by Branch workflow. |

Branch tab BC-2 workflow:
- `Refresh Matrix` -> inspect branch health/ahead/behind.
- Select target repos directly in matrix (or use group/tags filter).
- Run `Create/Sync/Prune` as `Preview` first, then `Execute`.
- Use `Branch Action Output` + timeline for evidence and failures.
- Execute requires a fresh preview token; changing scope/filters/selection invalidates stale previews.
- `Prune Execute` requires typed confirmation (`PRUNE`) before mutation.
- `Matrix Source` chip explains row origin:
  - `registry`: loaded directly from workspace registry
  - `bootstrapped`: registry was auto-seeded from AGOrg AGO records
  - `autodiscovered`: AGOrg auto-discovery/import ran, then seeded registry
  - `empty` / `error`: no rows or request failure
- Matrix scope behavior:
  - scope includes AGOrg `root_path` plus `master_path` (if present) so sibling AGO repos are considered in-scope.
  - Branch tab auto-loads matrix on tab activation; no manual refresh required for first render.
- Targeting behavior:
  - matrix header filter fields are the only source of group/tags.
  - selected repo IDs are authoritative when any rows are selected (no forced selection/filter intersection).
- Branch output is now an HTML activity log:
  - `Max logs` is stateful and capped to `100`.
  - `Clear Logs` clears Branch log entries immediately.
  - each entry has `Show JSON` + `COPY JSON`, and `OPEN ARTIFACT` when present.
- Branch tab BC-4 orchestration:
  - use `DAG Preview` before staged branch runs.
  - use `Staged Apply Preview` then `Staged Apply Execute` for dependency-aware fleet branch motion.
- Branch tab BC-6 policy guardrails:
  - protected branches (`main|master|dev|release*`) are blocked as mutate targets.
  - branch name policy is enforced for mutate executes:
    - `(feat|fix|docs|test|refactor|chore|perf)/kebab-case`.

1. `Dashboard -> System Status`

    - Run `Policy`, `Hook Policy`, and `Gate` before branch/release operations.
    - Use `Push Safe` instead of raw push for root-cause summaries.
    - In `Push Safe` summary, `auth_challenge_events` counts expected GitHub HTTPS auth handshakes (`HTTP/2 401` before credential retry); this is informational, not a failure.
    - Use `Start Bus` / `Stop Bus` / `Bus Status` for ArqonBus control.
    - Use `Export Evidence` to snapshot policy status, recent audit history, report index, and gate-log tails into `~/.pilot/reports/evidence_bundle_<timestamp>.json`.

2. `Dashboard` quick cards

    - Oracle + Heal quick actions for fast triage and plan/repair loops.
    - Branch + Multi quick actions for cohort branch and status operations.
    - `Post-Commit Routine` is now the primary Dashboard control deck for pilot-for-pilot flow:
      - set `group/tags`, `branch`, `remote`, and mutation toggles directly in the card
      - review `Resolve` and `Plan` before mutation
      - run the routine from the deck instead of manually stitching Dashboard + Multi + Push controls
      - use the stage workspace to inspect `Resolve`, `Plan`, `Multi`, `Gates`, `Push`, `CI`, `Evidence`, and `Reconcile`
      - use `Continuous Integration Observatory` to inspect discovered workflow files/jobs from `.github/workflows`, current run posture, and missing required CI coverage
      - use `Quick Edit Policy` in the card for dashboard-native `operator_routine` draft/simulate/activate flow
    - Temporary Components Inventory shows active bridge/shim state and exit criteria (`Refresh Inventory`).
    - Use `Run Checklist` for deterministic Wave H pass/fail gates.
    - Use `Export Inventory Artifact` to persist temporary-component evidence under `~/.pilot/reports/`.
    - Use `Wave Acceptance Matrix` (`quick` or `full`) to execute deterministic wave closure checks and produce an artifact.
    - Follow the in-panel `Recommended Sequence` strips:
      - Dashboard: `Status -> Bus Health -> Oracle Query -> Heal Plan -> Heal Run -> Push Safe`
      - Oracle: `Scan Index -> Run Query -> Open Report`
      - Heal: `Plan Only -> Review Response/Timeline -> Run Heal`
      - Multi: `Register -> List/Status/Order -> DAG/PR Plan -> Staged Apply`

3. `Operations Timeline` + `Operation Detail`

    - Filter failures, inspect payloads, and use artifact paths for one-click debugging.
    - Timeline cards now show `ARTIFACT` badge when linked report evidence is available.
    - Temporary inventory exports are timeline-linked with `artifact_path` for immediate review.
    - Acceptance matrix runs are timeline-linked and artifact-backed for audit replay.

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

7. Wave acceptance matrix (Wave I + Wave J close path)

    - CLI quick/full:
      - `./scripts/wave_acceptance_matrix.sh --wave I --profile quick`
      - `./scripts/wave_acceptance_matrix.sh --wave I --profile full`
      - `./scripts/wave_acceptance_matrix.sh --wave J --profile quick`
      - `./scripts/wave_acceptance_matrix.sh --wave J --profile full`
    - UI/API:
      - Dashboard card: `Wave Acceptance Matrix -> Run Matrix`
      - API: `POST /api/system/acceptance_matrix/run`
    - Artifacts:
      - `~/.pilot/reports/acceptance_matrix_wave_i_<profile>_<ts>.json`
      - `~/.pilot/reports/acceptance_matrix_wave_j_<profile>_<ts>.json`
    - Operator guardrails:
      - Keep only one active `pilot serve` instance for that UI port.
      - Do not launch overlapping matrix/gate runs; run one at a time.

5. `Codex` tab (contract-driven operations)

    - `Preview Contract` builds a normalized action contract (intent, command, payload, expected effect, rollback strategy) and generates `contract_id`.
    - `Approve Contract` transitions contract to executable state.
    - `Execute Contract` runs the approved contract through the Bus bridge with telemetry and response capture.
    - `Reconcile Contract` records verification outcome and closure notes.
    - Use `verify_command` to define post-action verification intent in the contract record.
    - Use `Contracts (Resume / Replay)` to reload prior contracts, inspect status, and retry failed contracts.
    - Contract history persists at `~/.pilot/reports/codex_contracts.jsonl` and is restored when UI restarts.

6. AGOrg scope controls (Wave 16 foundation)

    - Click the header chip (`AGOrg: ...`) to open AGOrg controls.
    - Use `Create AGOrg Project` to define scope + autoscan hierarchy.
    - Use `Discover Preview` to load candidates into the review panel before mutation.
    - Use `Approve All` / `Reject All` and per-row checkboxes in Discovery Review.
    - Use `Refresh Reviews` / `Load Review` to resume previous review sessions.
    - Use `Import Approved` to import only selected AGO candidates.
    - Use `Policy Report` to detect off-policy paths and metadata drift.
    - Use `Reconcile Dry Run` before mutation to preview prune impact.
    - Use `Reconcile Apply` only after reviewing dry-run output.
    - Use `Refresh Policy Artifacts` to list persisted reconciliation artifacts.
    - Use `pilot settings branch --show` on the CLI to inspect the active policy engine logic.
    - Settings tab governance APIs now include:
      - `/api/settings/policy/resolve`
      - `/api/settings/compliance_scan`
      - `/api/settings/decisions`
    - Use `List AGOrgs` and `Show Active` to verify current scope.
    - Use `Use Scope` to switch Control Panel context.
    - Use `Discover` to scan a root path with configurable depth.
    - Use `Discover + Import + Prune` for deterministic reconciliation of AGO rows.
    - Use `Tree` to inspect AGOrg/AGO graph structure.
    - Use `Link` for modular AGOrg composition (cycle-safe enforcement).
    - Use `Scope Profile Preferences` in Active Scope to persist branch/release defaults per AGOrg.
    - Use `Load Prefs` / `Save Prefs` after switching scope.
    - Use System Status `DB Status`, `DB Start`, `DB Stop` to control managed AGOrg datastore.

Wave D multi-instance operation:
- Start separate UI instances with isolated scope/session state:
  - `pilot serve --ui-port 7788 --ui-instance-id pilot-main ...`
  - `pilot serve --ui-port 7789 --ui-instance-id pilot-lab ...`

7. Managed DB operations (CLI)

    - `./scripts/pilot_local.sh db ensure`
    - `./scripts/pilot_local.sh db status`
    - `./scripts/pilot_local.sh db start`
    - `./scripts/pilot_local.sh db stop`

Managed DB defaults:
- data: `~/.arqon/pilot/db/data`
- logs: `~/.arqon/pilot/db/postgres.log`
- endpoint: Unix socket on Linux/macOS, local TCP fallback on Windows
- identity guard: Pilot refuses migration if DB identity is not `arqon_pilot`

AGOrg reconciliation artifacts:
- `~/.pilot/reports/agorg_policy_report_<timestamp>.json`
- UI/API support:
  - `POST /api/agorg/policy_report`
  - `GET /api/agorg/policy_reports`
  - `POST /api/agorg/reconcile_apply`

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

### Managed DB Rollback

If a migration fails or data is corrupted:

1. Stop Pilot: `pilot db stop`
2. Drop current DB: `dropdb -h /tmp/.arqon-pilot -p 9132 pilot_local`
3. Recreate DB: `createdb -h /tmp/.arqon-pilot -p 9132 pilot_local`
4. Restore from snapshot: `psql -h /tmp/.arqon-pilot -p 9132 pilot_local < /tmp/pilot_backup.sql`
5. Restart: `pilot db start`

Avoid:

- history rewriting on shared branches (`reset --hard`, force-push) except emergency local recovery with explicit approval.

## 6. Incident Triage

When a multi-repo operation fails:

1. open latest report artifact from `~/.pilot/reports/`
2. isolate failing repo(s)
3. re-run command scoped to failed repo only
4. record root cause and corrective action in `pilot know`

## 7. Alpha Release Gate

Before cutting an alpha tag (example: `v0.2.0-alpha.1`):

1. `./scripts/prepush_gate.sh`
2. `./scripts/release_readiness_check.sh`
3. `./scripts/wave_acceptance_matrix.sh --wave I --profile full`
4. `./scripts/wave_acceptance_matrix.sh --wave J --profile full`
5. `./scripts/ui_smoke_check.sh`
6. if publishing, verify index visibility:
   - `./scripts/verify_pypi_release.sh --index pypi --version <X.Y.ZaN>`
7. collect release evidence:
   - `./scripts/release_collect_evidence.sh --label <X.Y.ZaN>`

Authoritative release docs:

1. `docs/release-playbook.md`
2. `docs/release-log.md`

Execution order is defined in:

1. `docs/release-playbook.md` -> `0) Full Release Order (Do Not Reorder)`
