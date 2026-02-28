# AGOrg Control Plane Plan

This document captures the long-term AGOrg vision and the implementation plan so it is never lost across sessions.

**Last updated**: 2026-02-28 21:30 EST — Wave I hard-close complete + TD Wave 3/4 complete

---

## Vision

Arqon Pilot should run as a multi-organization control plane, not just a single-repo tool.

1.  **Master AGOrg Directory (Flat Fleet)**: All components (AGOrgs and AGOs) must reside as siblings within a shared parent directory (Master Directory).
    - *Canonical Example*: `~/Projects/arqon/`.
    - *The Rule*: AGOrgs and AGOs can only be neighbors; nesting repositories inside other repositories is strictly forbidden.
2.  **Import over Create**: We primarily **Import** existing ecosystems from the Master Directory. Onboarding assumes an existing structure that Pilot validates and manages.
3.  **Linkage over Containment**: AGOrgs consist of **Links** to sibling repositories (AGOs) and other AGOrgs. Containment is logical/virtual via metadata, not physical via nesting.
4.  **Coexistence**: Multiple AGOrgs can coexist and overlapping hierarchies can be established within the same Master Directory. 
5.  **Auditability**: If a directory (`.git` or `pyproject.toml` boundary) lacks Arqon metadata, it is NOT part of the AGOrg registry. Pilot provides an "Upgrade" flow to make directories compliant.
6.  **Contextual Scope**: The Control Panel is scoped to one active Master AGOrg at a time, switching the entire operational boundary instantly.

---

## Final Architecture Decisions (Locked)

1. **Identity model**
     - AGOrg primary key: UUID.
     - AGOrg root path must be unique per AGOrg record.

2. **Discovery scan policy (Flat Fleet Enforcement)**
     - Discovery depth is configurable, but the script **MUST stop recursion** at any repository boundary (`.git` or `pyproject.toml`).
     - Repositories cannot be nested; if a directory is needed outside the Master Directory, a **symbolic link** must be used within the parent to maintain the flat namespace.

3. **Linkage model**
     - AGOrgs are modular and recombinable.
     - Relationships are defined as **Links** (edges) in the database and mirrored in `pyproject.toml`.
     - "Parent" relationships are links of orientation, not mandatory physical nesting.
     - **Upgrade flow**: Any AGO (repo) can be upgraded to an AGOrg to enable its own linkage ecosystem.

4. **Database mode (initial)**
     - Local Postgres only for current phase.
     - Pilot owns creation, migrations, and maintenance automatically.

5. **Managed runtime contract (Wave 16 close)**
     - Private runtime/data paths:
     - `~/.arqon/pilot/db/data`
     - `~/.arqon/pilot/run`
     - `~/.arqon/pilot/db/postgres.log`
     - Linux/macOS default endpoint: Unix socket in runtime dir.
     - Windows default endpoint: local TCP with deterministic high-port fallback.
     - Lifecycle is explicit and operator-visible:
     - `pilot db ensure`
     - `pilot db start`
     - `pilot db stop`
     - `pilot db status`
   - Safety identity guard:
     - DB must report `pilot_identity.system = arqon_pilot` before migrations.
   - External DB override:
     - `PILOT_AGORG_DATABASE_URL` disables managed startup and uses external DSN directly.

---

## Existing Contract (Already Present)

The current relationship declaration exists in project metadata:

```toml
[tool.arqon.relationships]
parent = "Arqon"
children = []
```

Arqon Pilot should treat this as authoritative repo-level relationship metadata when discovering and registering AGOrgs.
In this model, relationship metadata belongs to repos (AGOs), where `parent` points to the owning AGOrg.

---

## Product Requirements

### 1) AGOrg Scope Control

1. Add an `AGOrg` section in the Control Panel.
2. Include a path browse/input field and a `Load Scope` action.
3. Selected AGOrg becomes the scope boundary for Dashboard, Oracle, Heal, Dependencies, Branch, Multi, and Telemetry.

### 2) AGOrg CRUD

1. Create AGOrg record.
2. Read/list AGOrg records.
3. Update AGOrg metadata/settings.
4. Delete AGOrg record (with confirmation + non-destructive defaults).

#### Create AGOrg Project (Required UX)

1. Add a `Create AGOrg Project` flow with:
     - AGOrg name
     - root path (browse button + input field)
     - optional parent AGOrg selector (for nested AGOrg)
     - initial scan toggle (`AutoScan hierarchy now`)
2. On create, persist AGOrg and optionally execute discovery immediately.
3. Show preview of discovered AGOrg/AGO hierarchy before final save.

### 3) AGOrg Discovery

1. Add `Discovery Root` path input.
2. Add `Discover` action that scans directory trees.
3. Identify AGOrg and AGO candidates by relationship metadata and repo structure.
4. Build a hierarchy/graph view:
     - AGOrg nodes
     - nested AGOrg nodes
     - AGO leaf nodes
5. Show discovery results and allow selective registration/import.
6. Persist discovery method metadata (`manual`, `autoscan`, `rescan`).
7. Respect configured scan depth during discovery.

### 3.1) Reconciliation and AGOrg Policy Conformance

After loading a primary AGOrg and discovering AGOs, Pilot must support a guided
reconciliation workflow so the whole ecosystem converges to AGOrg policy.

Goals:

1. Detect policy drift across all discovered AGOs.
2. Reconcile duplicates and ambiguous repo records safely.
3. Normalize partial candidates (folders that look like AGOs but are not fully configured).
4. Produce a deterministic, auditable reconciliation report before apply.

Required checks:

1. Duplicate detection:
     - same canonical repo path registered more than once
     - same repo name mapped to different roots
2. Candidate quality checks:
     - git repo present but missing expected Arqon metadata
     - missing `[tool.arqon.relationships]` block in `pyproject.toml`
3. Policy conformance checks:
     - branch policy mismatch (default base/pr branches, naming standards)
     - dependency management policy mismatch (group/tags/dependency edges)
     - scope mismatch (repo appears outside selected AGOrg boundary)

Reconciliation UX requirements:

1. `Discovery Results` should classify each item:
     - `conformant`
     - `needs_reconciliation`
     - `missing_metadata`
     - `duplicate`
2. Operator can choose per-item action:
     - `register as AGO`
     - `merge with existing record`
     - `mark as nested AGOrg`
     - `ignore`
     - `open for manual review`
3. A final `Apply Reconciliation` step executes all approved fixes and stores an artifact report.

Non-goals for first pass:

1. Auto-editing arbitrary repo files without explicit approval.
2. Auto-merging conflicting graph links without operator confirmation.

### 4) State, Preferences, and Profiles

Each AGOrg stores:

1. Default root path.
2. AGOrg identity.
3. Child AGO registry.
4. Child AGOrg registry (nested AGOrg support).
4. Tags/groups used for operations.
5. UI preferences.
6. Bus settings and control channels.
7. Last active tab/context.
8. Default branch/release preferences.

On restart, Pilot should auto-load the configured default AGOrg.

Initial default target:

1. `~/Projects/arqon/Arqon` must be loadable as default AGOrg.

---

## UX Plan

### New top-level surfaces

### 1) AGOrg Management (3-Panel System)

The AGOrg tab is organized into three persistent functional panels:

1.  **Panel 1: AGOrg Settings (CRUD)**:
     - **Import**: Onboard an existing Master Directory.
     - **Settings**: Modify metadata for the active AGOrg.
     - **Management**: List, Create, and Delete AGOrg records.
2.  **Panel 2: Interactive Hierarchy**:
     - Displays the current Master Directory tree.
     - Click any node (AGOrg or AGO) to deep-dive into its specific settings.
     - Drag-and-drop support: Reposition repositories to update their `pyproject.toml` lineage automatically.
     - "Upgrade" toggle to promote an AGO to an AGOrg.
3.  **Panel 3: Results Display**:
     - Real-time JSON output from all AGOrg operations.
     - **Word Wrap Enabled**: JSON must wrap and not clip.
     - Persistent "COPY" and "CLEAR" utilities.

2. `Dashboard` integration:
     - active AGOrg badge in header
     - quick AGOrg switch dropdown
     - explicit scope indicator on mutating actions

### Guardrails

1. No mutation without visible active AGOrg scope.
2. Scope mismatch warnings when running repo-specific actions outside current AGOrg.
3. Non-destructive defaults (`dry-run`, explicit apply toggles).

---

## Backend/Data Model Plan

Add AGOrg entities in local state store:

1. `agorgs` table/collection
2. `agorg_repos` relationship map
3. `agorg_preferences`
4. `agorg_last_session_state`
5. `agorg_links` (AGOrg-to-AGOrg graph edges)

Suggested fields:

1. `id` (stable UUID)
2. `name` (`Arqon`)
3. `root_path`
4. `parent_agorg_id` (nullable, for nested AGOrgs)
5. `children_agorgs` (AGOrg records)
6. `children_agos` (AGO repo records)
7. `default_scope` (bool)
8. `created_at`, `updated_at`
9. `scan_depth` (int, configurable per AGOrg/discovery run)

---

## API/Command Surface Plan

Add new commands/endpoints:

1. `pilot agorg create`
2. `pilot agorg list`
3. `pilot agorg update`
4. `pilot agorg delete`
5. `pilot agorg use`
6. `pilot agorg discover`
7. `pilot agorg tree`
8. `pilot agorg link`
9. `pilot agorg scan_master`
10. `pilot agorg batch-create`
11. `pilot agorg create-project` (CLI; create + autoscan in one call) and `/api/agorg/create_project` (API route)

---

## Rollout Waves — Audit-Verified Status (2026-02-28)

### Wave A — Foundation ✅ COMPLETE

| Feature | Status | Evidence |
|---------|--------|----------|
| AGOrg data model + persistence (Postgres) | ✅ | `agorgs`, `agos`, `agorg_links`, `app_state` tables in `agorg.rs:572-683` |
| Active scope selection (`/api/agorg/use`) | ✅ | Handler at `serve_ui.rs:532-544`, persists in `app_state` via `set_active_agorg` |
| Auto-load active scope on startup | ✅ | `refreshAgorgHeader()` calls `/api/agorg/active` on page load (`serve_ui.rs:4760`), which reads from `app_state` table |
| UI Header sync with active scope | ✅ | `setAgorgStatus()` updates Hero badge from `refreshAgorgHeader` |
| Managed Postgres lifecycle | ✅ | `pilot db start/stop/status/ensure` all functional |

**Wave A is fully operational. No open items.**

---

### Wave B — CRUD + Discovery 🔶 IN PROGRESS (~90%)

#### B.1 — Import Existing Ecosystem (PRIMARY PATH)

| Feature | Status | Evidence |
|---------|--------|----------|
| `create` — create AGOrg record | ✅ | `agorg.rs:116-160`, route at `serve_ui.rs:266` |
| `create-project` (CLI) / `create_project` (API) — create + autoscan in one call | ✅ | `agorg.rs:442-463`, route at `serve_ui.rs:267` |
| `list` — list all registered AGOrgs | ✅ | `agorg.rs:218-231`, route at `serve_ui.rs:263` |
| `use` — set active scope | ✅ | `agorg.rs:233-246`, route at `serve_ui.rs:270` |
| `discover` — scan directory tree | ✅ | `agorg.rs:686-742`, route at `serve_ui.rs:271` |
| `discover` import reconciliation (`--prune-missing`) | ✅ | `import_discovery_with_options` in `agorg.rs`; CLI flags on `agorg discover/create-project`; API supports `prune_missing` |
| `tree` — render hierarchy | ✅ | `agorg.rs:355-440`, route at `serve_ui.rs:272` |
| `link` — link parent/child AGOrgs | ✅ | `agorg.rs:284-322` with cycle detection, route at `serve_ui.rs:273` |
| `update` — update AGOrg metadata | ✅ | `agorg.rs:162-206`, route at `serve_ui.rs:268` |
| `delete` — delete AGOrg record (BACKEND) | ✅ | `agorg.rs:208-216`, route at `serve_ui.rs:269` |
| Import Discovery — auto-register repos as AGOs | ✅ | `import_discovery` at `agorg.rs:465-480` |
| `scan_master` — scan Master Directory | ✅ | `scan_master_directory` at `agorg.rs:878-927`, route at `serve_ui.rs:274` |
| `upgrade_ago` — promote AGO to compliant | ✅ | `upgrade_ago` at `agorg.rs:978-991`, route at `serve_ui.rs:275` |
| `edit_relationship` — modify pyproject.toml links | ✅ | `edit_relationship` at `agorg.rs:929-976`, route at `serve_ui.rs:277-279` |
| Hero Dropdown — lists AGOrgs + AGOs | ✅ | HTML + `toggleAgorgDropdown` JS function |
| Registry Panel — grouped by Master Dir + icons | ✅ | `renderAgorgRegistry()` at `serve_ui.rs:4211-4250` |
| Scan depth input in Import panel | ✅ | `id="agorg-depth"` at `serve_ui.rs:3020` |
| Import button wired to `agorgCreateProject()` | ✅ | `serve_ui.rs:3037` → JS at `serve_ui.rs:4056-4078` |
| Active Scope panel with details | ✅ | `agorgShowActive()` at `serve_ui.rs:4302-4327` |
| Update button in Active Scope panel | ✅ | HTML button at `serve_ui.rs:2978`, JS at `serve_ui.rs:4166-4190` |
| Directory browser (zenity integration) | ✅ | `browseAgorgMaster`, `browseAgorgRoot`, etc. → `pick_directory` at `serve_ui.rs:682-700` |
| Delete button in Active Scope panel | ✅ | `agorgDelete()` is implemented and wired end-to-end; delete flow works from UI and CLI. |
| `switchAgorgScope` request body | ✅ | UI sends `{ agorg }` payload expected by `/api/agorg/use`; scope switching now succeeds from Hero Dropdown and Registry panel. |

#### B.2 — Batch Create (SECONDARY PATH)

| Feature | Status | Evidence |
|---------|--------|----------|
| Batch Create API | ✅ | `init_agorg_batch` at `agorg.rs:483-524`, route at `serve_ui.rs:265` |
| `git init` per AGO directory | ✅ | `agorg.rs:503-507` (uses `spawn()` — fire-and-forget) |
| `pyproject.toml` with relationships per AGO | ✅ | `agorg.rs:510-517` |
| Batch Create UI Panel | ✅ | "Initialize New AGOrg" sub-panel at `serve_ui.rs:3041-3066` |
| Progress feedback (SSE/WS pipe) | ⬜ | No real-time progress events. Fire-and-forget only. |
| Configurable default destination | ⬜ | Currently hardcoded/browser-picked. No per-AGOrg config. |

#### B.3 — Registration Review/Approval Step ✅ COMPLETE

| Feature | Status |
|---------|--------|
| After Discovery, show preview panel with classification per item | ✅ |
| Let operator approve/reject before import | ✅ |
| Persist review decisions/history as artifact | ✅ |
| Resume/replay prior review sessions in UI | ✅ |
| Precursor to full Reconciliation (B.4) | ✅ |

#### B.4 — Reconciliation 🔶 PARTIAL

| Feature | Status |
|---------|--------|
| Duplicate detection | ✅ |
| Candidate quality checks | ✅ |
| Policy conformance checks | ✅ |
| Reconciliation report artifact | ✅ |
| Reconciliation apply (dry-run + apply) | ✅ |
| UI duplicate resolution controls (winner/loser preview) | ⬜ |

**Progress note:** `agorg reconcile` (CLI/API/UI) reports off-policy items. UI/API now support:
1. `POST /api/agorg/policy_report` (persisted artifact in `~/.pilot/reports/agorg_policy_report_<ts>.json`)
2. `GET /api/agorg/policy_reports` (artifact listing)
3. `POST /api/agorg/reconcile_apply` (dry-run preview or mutation apply)
4. Dashboard + AGOrg panel controls for report/dry-run/apply.

Remaining B.4 close-out is UI presentation of duplicate merge winners/losers before apply.

---

### Wave C — Full Scope Enforcement ✅ COMPLETE

| Feature | Status |
|---------|--------|
| Dashboard operations scoped to active AGOrg repos | ✅ |
| Oracle scans only within AGOrg boundary | ✅ |
| Heal targets only AGOrg repos | ✅ |
| Dependencies checks scoped to AGOrg | ✅ |
| Branch operations scoped to AGOrg repos | ✅ |
| Multi-repo actions scoped to AGOrg | ✅ |
| Telemetry stream tagged with AGOrg context | ✅ |
| AGOrg link validation blocks circular loops | ✅ (Already done — `link_agorgs` has cycle detection at `agorg.rs:290-311`) |

Current enforcement now active in UI command bridge:

1. Active AGOrg scope required for `pilot.branch.*`, `pilot.multi.*`, `pilot.oracle.*`, `pilot.heal.*`, `pilot.navigate.*`.
2. CWD boundary guard for repo-local command families (`branch/oracle/heal/navigate`): rejects execution when current repo path is outside active AGOrg root.
3. Multi command selector guard: rejects unfiltered `pilot.multi.*` calls unless `group` or `tags` are explicitly set.
4. Dashboard dependency actions (`policy`, `hook-policy`, `drift`, `gate`, `repair`, `push`) now require active AGOrg scope and CWD-in-scope checks.
5. Global service controls (`db-*`, `bus-*`, `services-*`) remain available outside AGOrg scope by design.
6. Live SSE telemetry events are annotated with `agorg_scope` (active scope context) when emitted to UI clients.

---

### Wave D — Profiles and Multi-Instance Readiness ✅ COMPLETE

| Feature | Status |
|---------|--------|
| Per-AGOrg preferences/profile settings | ✅ |
| Fast scope switching (cached state) | ✅ |
| Concurrent Pilot instances with isolated AGOrg contexts | ✅ |
| Last active tab/context restoration | ✅ |

Wave D close-out details:

1. AGOrg settings/profile persistence added through `/api/agorg/preferences` (GET/POST), backed by `agorgs.settings` JSON.
2. Fast scope switching now uses `/api/agorg/scope_snapshot` (active + full list + recent scopes + UI session) with client-side cache TTL.
3. Multi-instance isolation implemented with namespaced app-state keys:
   - `instance:<ui_instance_id>:active_agorg_id`
   - `instance:<ui_instance_id>:recent_scope_ids`
   - `instance:<ui_instance_id>:ui_session_state`
4. UI session restore/save implemented via `/api/ui/session`:
   - restores last active tab and key AGOrg/Multi context fields on boot
   - persists context after tab/scope/input changes
5. `pilot serve` supports explicit instance selection:
   - `--ui-instance-id <name>`
   - default instance id is `ui-<ui_port>`

---

### Wave E — Reconciliation UX + Artifacts ✅ COMPLETE

Goal:
1. Make reconciliation a first-class operator workflow from Dashboard + AGOrg tab.

Delivered:
1. `POST /api/agorg/policy_report` persisted artifact generation.
2. `GET /api/agorg/policy_reports` artifact listing.
3. `POST /api/agorg/reconcile_apply` dry-run/apply execution.
4. Dashboard and AGOrg controls for `Policy Report`, `Reconcile Dry Run`, `Reconcile Apply`.
5. Contract tests for report/dry-run/apply response shape.

Completed in hard-close:
1. Added explicit winner/loser duplicate-resolution visualization in both Dashboard and AGOrg panels.
2. Added artifact `Open` actions wired to `/api/report/read` from both report selectors.
3. Extended policy artifact listing response with stable `name` + relative `path` fields for deterministic selector rendering.

Acceptance:
1. Operator can run report -> dry-run -> apply entirely from UI.
2. Operator can inspect artifact list, open selected artifact, and identify latest report deterministically.
3. Duplicate merge decisions are visible before mutation.

---

### Wave F — AGOrg Policy Conformance (Branch + Dependency) ✅ COMPLETE

Goal:
1. Extend reconciliation beyond path/metadata to branch/dependency policy conformance.

Planned scope:
1. Branch policy checks:
   - default base/release branch mismatch against AGOrg preferences.
   - branch naming convention drift.
2. Dependency policy checks:
   - missing/invalid repo group/tags.
   - broken or cyclic dependency links in AGOrg context.
3. Reconcile report extension:
   - classify issues by `policy_branch`, `policy_dependency`, `metadata`, `topology`.
4. Guided fix actions:
   - dry-run mutations first, explicit apply second.

Delivered so far:
1. `AgorgReconcileIssue` now carries explicit `issue_class` values (`policy_branch`, `policy_dependency`, `metadata`, `topology`).
2. `AgorgReconcileReport` now includes `class_counts` for deterministic per-class drift visibility.
3. Reconcile now emits dependency-policy issues:
   - `dependency_parent_mismatch`
   - `dependency_self_link`
   - `dependency_duplicate_child`
4. Reconcile now emits branch-policy issue:
   - `branch_name_off_policy` (when branch is outside base/release and standard branch prefixes).
5. Dashboard + AGOrg UI now render class-count summaries from reconcile reports.
6. Dashboard + AGOrg UI now include class filters and issue drill-down controls:
   - filter by class (`policy_branch`, `policy_dependency`, `metadata`, `topology`, `all`)
   - browse filtered set with Prev/Next
   - view selected issue detail payload directly in-panel

Acceptance:
1. Reconcile report includes branch/dependency conformance for every in-scope AGO. ✅
2. Operator can inspect by policy class and drill into individual issues from AGOrg + Dashboard. ✅
3. No unscoped fleet mutations allowed. ✅
4. Guided auto-fix actions for policy classes remain future work (Wave G+).

---

### Wave G — Unified Dashboard Control Plane ✅ COMPLETE

Goal:
1. Make Dashboard the primary command center; tabs become specialized drill-down views.

Planned scope:

1. Dashboard AGOrg overview card:
     - scope health, conformance score, unresolved issues.
2. Action contract orchestration:
     - clear preview/approve/execute/reconcile sequence for AGOrg policy operations.
3. Event-first UX:
     - timeline entries linked to report artifacts and specific reconcile actions.
4. State continuity:
     - preserve filters, selected AGOrg, and pending action context across reload/restart.

Delivered so far:

1. Dashboard AGOrg overview endpoint and card are wired:
     - `POST /api/agorg/dashboard_overview`
     - returns score, unresolved issues, off-policy count, class counts, and full reconcile report.
2. Dashboard AGOrg action-contract flow is wired through Codex lifecycle:
     - preview
     - approve
     - execute
     - reconcile
3. Local AGOrg contract command path added:
     - `api.agorg.policy_report`
     - `api.agorg.reconcile_apply`
4. Mutation guard now correctly treats `api.agorg.reconcile_apply` as non-mutating when `dry_run=true`.
5. Policy/reconcile actions now emit event payloads with artifact linkage:
   - `artifact_path` included for policy reports and reconcile dry-run/apply outputs.
6. Dashboard now provides one-click artifact open actions:
   - `OPEN ARTIFACT` in AGOrg Action Contract output panel.
   - `OPEN ARTIFACT` in Operation Detail timeline panel.
7. Timeline cards now show at-a-glance `ARTIFACT` badge when artifact linkage is detected.

Acceptance:

1. Core AGOrg operations can be executed from Dashboard without tab hopping. ✅
2. Every mutation is visible in timeline with artifact linkage. ✅
3. Session restore brings operator back to in-progress workflow state. ✅

---

### Wave H — Temporary Component Burn-Down ✅ COMPLETE

Goal:

1. Remove avoidable bridge components and surface any unavoidable ones explicitly.

Current known temporary components:

1. ArqonBus compatibility shim (`scripts/arqonbus_shim.sh`) — required when frozen Bus checkout is not directly runnable.
2. Hierarchy drag/link editor gap — explicit relationship editor path is active; drag UX is intentionally deferred until audited.

Planned scope:

1. Replace shim path with native bus integration path where feasible.
2. Keep hierarchy edits explicit and audited (`/api/agorg/edit_relationship`) until drag-link is hardened.
3. Add periodic inventory check in docs + runbook so no hidden placeholders/stubs accumulate.

Delivered so far:

1. Added Dashboard **Temporary Components Inventory** card with one-click refresh.
2. Added backend inventory endpoint:
   - `GET /api/system/temporary_components`
3. Inventory now surfaces:
   - ArqonBus shim runtime state (`running`/`stopped`) with status command output.
   - Hierarchy editor gap with current safe path + explicit exit criteria.
4. Removed ambiguous `TODO` wording from AGOrg hierarchy helper text.
5. Added inventory artifact export path:
   - `POST /api/system/temporary_components/export`
   - persists report artifact under `~/.pilot/reports/temporary_components_inventory_<ts>.json`
   - emits artifact-linked timeline event.
6. Added deterministic checklist endpoint + dashboard flow:
   - `GET /api/system/temporary_components/checklist`
   - checks inventory API, shim status detection, TODO-text removal, and docs coverage gates.
   - exposes `overall_pass` for hard-close verification.

Acceptance:

1. No hidden non-essential shims/stubs/placeholders remain (within current known inventory). ✅
2. Any unavoidable temporary component is documented in this plan + runbook + gotcha registry. ✅
3. Operator can identify temporary components in under 60 seconds. ✅

---

### Wave I — Acceptance Matrix Execution 🔶 IN PROGRESS

Goal:

1. Make wave closure gates executable and auditable from both CLI and Dashboard.

Planned scope:

1. Add wave acceptance matrix runner script with `quick`/`full` profiles.
2. Add API endpoint to execute matrix and persist artifact.
3. Add Dashboard controls for running matrix and opening artifact.
4. Ensure matrix runs are timeline-linked with clear pass/fail semantics.

Delivered so far:

1. Added script:
   - `scripts/wave_acceptance_matrix.sh --wave I --profile {quick|full}`
2. Added API endpoint:
   - `POST /api/system/acceptance_matrix/run`
3. Added Dashboard `Wave Acceptance Matrix` card:
   - run matrix
   - open artifact
4. Matrix run artifacts now persist under `~/.pilot/reports/acceptance_matrix_wave_i_<profile>_<ts>.json` and emit timeline-linked events.

Acceptance:

1. Operator can execute Wave I matrix from UI and CLI. ✅
2. Matrix result includes deterministic check list + artifact path. ✅
3. Matrix run appears in timeline with artifact linkage. ✅
4. Full-profile matrix should remain green on healthy repo state. ⬜

---

## Gotcha Reference

All known gotchas are documented in `docs/gotcha-registry.md`. The following are directly relevant to AGOrg development:

| ID | Title | Relevance |
|----|-------|-----------|
| **G-012** | DB running but AGOrg commands fail with socket `os error 2` | DSN must include `port=9132` for Pilot-managed DB |
| **G-015** | Entire Pilot UI Dead — Duplicate `const` Declarations | ANY `const` added to the JS block must be checked for duplicates first. `cargo check` will NOT catch this. **Always check browser console after UI changes.** |

### Additional Gotchas (Learned 2026-02-27)

**G-A: API response shape inconsistency**
- The `/api/agorg/list` endpoint returns `{ ok: true, agorgs: [...] }` (wrapped).
- The Hero Dropdown code at one point tried to parse the response as a raw array.
- **Rule**: Always use `fetchJsonSafe()` and check `data.ok` before accessing nested fields. Never assume the response shape — always verify against the handler in `serve_ui.rs`.

**G-B: Two function definition locations for the same name**
- `switchAgorgScope` was defined in both the Hero Dropdown section AND the AGOrg tab section of the JS block. `async function` declarations are hoisted and the second definition silently overwrites the first, but the two implementations had different request body shapes (`{ id }` vs `{ agorg: id }`), causing silent API failures.
- **Rule**: Every function name must be `grep`-unique in the JS block before adding or modifying.

**G-C: `renderAgorgRegistry` function removed but still called**
- During manual revert, the rendering function was deleted from the JS block, but calls to it remained in `agorgList()`. This threw a `ReferenceError` that halted all subsequent JS execution in the same call chain.
- **Rule**: When removing a function, always search for all call sites. A `ReferenceError` in an `async` function will silently swallow the error in the caller if not wrapped in try/catch.

**G-D: `switchAgorgScope` request shape mismatch** ✅ RESOLVED
- Historical issue: UI sent `{ id }` while `/api/agorg/use` expects `{ agorg }`.
- Resolution: scope switch path now sends `{ agorg: <id> }` and is validated in current UI flow.

**G-E: `agorgDelete()` missing handler** ✅ RESOLVED
- Historical issue: AGOrg Delete button called undefined JS function.
- Resolution: `agorgDelete()` is implemented and wired end-to-end (UI + backend route).

---

## Acceptance Criteria

1. Operator can register and switch AGOrgs without manual file edits.
2. Active AGOrg scope is always visible.
3. All mutating actions are scoped and auditable to AGOrg.
4. `~/Projects/arqon/Arqon` can be set and auto-loaded as default AGOrg.
5. Discovery finds AGOrg/AGO candidates from a directory tree and supports selective import.
6. Nested AGOrg hierarchies are represented, persisted, and navigable.
7. `Create AGOrg Project` can create + autoscan in one flow.
8. AGOrg graph links are reusable across multiple parent AGOrgs without conflict.
9. Cycle creation is blocked deterministically.

---

## Dogfooding Test Case (Initial)

Use `~/Projects/arqon/` as the Master Directory and first AGOrg loaded in the system.

1. Register as AGOrg default scope.
2. Run autoscan discovery.
3. Verify expected children include AGOs such as `ArqonBus`, `ArqonCore`, and `ArqonPilot` (17 total siblings).
4. Persist and reload Control Panel; confirm AGOrg auto-load and scope restoration.

### Dogfood Evidence Snapshot (2026-02-27)

Commands executed:

1. `./scripts/pilot_local.sh agorg use c31d9200-30f6-4418-b33c-8ea5269c4461`
2. `./scripts/pilot_local.sh agorg update c31d9200-30f6-4418-b33c-8ea5269c4461 --default-scope --master /home/irbsurfer/Projects/arqon --scan-depth 4`
3. `./scripts/pilot_local.sh agorg discover --root /home/irbsurfer/Projects/arqon --depth 4 --import-to c31d9200-30f6-4418-b33c-8ea5269c4461`
4. `./scripts/pilot_local.sh agorg show`
5. `./scripts/pilot_local.sh agorg tree --root c31d9200-30f6-4418-b33c-8ea5269c4461`

Observed result:

1. Active/default scope correctly moved to `/home/irbsurfer/Projects/arqon`.
2. Discovery imported **21 candidates** (expected baseline was ~17 siblings).
3. Tree includes valid top-level AGOs (`ArqonBus`, `ArqonCore`, `ArqonPilot`, etc.) **and** extra nested/archive entries:
   - `/home/irbsurfer/Projects/arqon/archive/ArqonSAS`
   - `/home/irbsurfer/Projects/arqon/ArqonHPO/bindings/python`
   - `/home/irbsurfer/Projects/arqon/ArqonBus/sdks/python`
   - `/home/irbsurfer/Projects/arqon/ArqonCore/python/arqon_narrative`

Conclusion:

1. Wave A/B import flow is operational.
2. Flat Fleet enforcement is not strict enough yet for nested repo boundaries and archive exclusion.
3. Next change must add deterministic discovery guardrails:
   - skip any path under `/archive/` by default (configurable override later),
   - stop recursion at first repo boundary and do not import nested sub-repos as AGOs unless explicitly enabled.

### Guardrail Update Applied (2026-02-27, same session)

Implemented in `crates/pilot/src/agorg.rs`:

1. `discover_hierarchy` now enforces flat-fleet defaults:
   - nested repositories (`depth > 1`) are ignored by default,
   - `archive/` subtree is skipped by default.
2. Explicit opt-in for nested discovery:
   - set `PILOT_AGORG_ALLOW_NESTED_REPOS=1` to include nested repos during discovery.
3. `scan_master_directory` now also skips `archive/` and `site` directories by default.

Validation snapshot:

1. `./scripts/pilot_local.sh agorg discover --root /home/irbsurfer/Projects/arqon --depth 4`
2. Candidate count observed: `20` (includes root AGOrg marker + top-level fleet AGOs only)
3. Previously leaked entries were removed from discovery output:
   - `/archive/ArqonSAS`
   - `/ArqonHPO/bindings/python`
   - `/ArqonBus/sdks/python`
   - `/ArqonCore/python/arqon_narrative`

Note:

1. In restricted sandbox mode, `--import-to` DB mutation may fail with OS permission constraints unrelated to AGOrg logic. Core discovery guardrail behavior was verified via non-mutating discover output.

### Reconciliation Evidence Snapshot (2026-02-28)

Command executed:

1. `./scripts/pilot_local.sh agorg discover --root /home/irbsurfer/Projects/arqon --depth 4 --import-to c31d9200-30f6-4418-b33c-8ea5269c4461 --prune-missing`

Observed result:

1. `upserted=19, pruned=4, final=19`
2. Follow-up `agorg tree --root c31d9200-30f6-4418-b33c-8ea5269c4461` no longer contained:
   - `archive/ArqonSAS`
   - `ArqonHPO/bindings/python`
   - `ArqonBus/sdks/python`
   - `ArqonCore/python/arqon_narrative`

Conclusion:

1. Reconciliation via import+prune is now operational and deterministic.
2. Stale AGO rows can be corrected without manual DB edits.

### B.3 Persistence + Replay Evidence Snapshot (2026-02-28)

Implemented:

1. Review sessions persist to `~/.pilot/reports/agorg_reviews.jsonl`.
2. `Discover Preview` returns a persisted `review_id`.
3. `Import Approved` updates the same review record with selected approvals and import summary.
4. AGOrg panel can `Refresh Reviews` and `Load Review` to restore candidate/approval state.

Validation:

1. `serve_ui::tests::test_agorg_review_persistence_roundtrip` passes.
2. `agorg` CLI and pre-push gate remain green after integration.

---

### Iteration Update (2026-02-28)

Completed in this iteration:

1. **B.4 duplicate merge heuristics implemented**
   - `reconcile_agorg` now computes deterministic merge candidates for:
   - duplicate canonical-path AGOs (`duplicate_path_merge_candidate`)
   - duplicate-name AGOs (`duplicate_name_merge_candidate`)
   - winner selection is score-based (top-level, non-archive, has `pyproject.toml`, exists).
2. **Reconcile API contract tests added**
   - Policy report contract response shape test.
   - Reconcile dry-run response shape test.
   - Reconcile apply response shape test.
3. **Plan/document status refreshed** to keep roadmap continuity.

Verification completed:

1. `cargo check -p pilot --locked` ✅
2. `cargo test -p pilot --locked test_duplicate_` ✅
3. `cargo test -p pilot --locked test_agorg_reconcile_api_` ✅
4. `./scripts/prepush_gate.sh` ✅

Known temporary/bridge components (explicit inventory):

1. **ArqonBus compatibility shim** (`scripts/arqonbus_shim.sh`) — currently required when frozen ArqonBus checkout is not directly runnable in this workspace.
2. **UI hierarchy drag-link gap** — interactive drag-link workflow is intentionally deferred; use explicit audited relationship editor APIs/UI actions.

### Iteration Update (2026-02-28 17:06 EST)

Completed in this iteration:

1. **Wave E UI hard-close delivered**
   - Dashboard and AGOrg now show duplicate merge winner/loser previews from `report.duplicate_resolutions`.
   - Policy artifact selectors now support explicit `Open` actions to load report JSON via `/api/report/read`.
2. **Policy artifact selector contract hardened**
   - `/api/agorg/policy_reports` now returns stable `name` + relative `path` entries to avoid fragile filename parsing in UI.
3. **Reconcile response handling unified**
   - Report, dry-run, and apply handlers now sync duplicate preview panels in both Dashboard and AGOrg surfaces.

Verification completed:

1. `node -c crates/pilot/src/pilot_ui.js` ✅
2. `cargo check -p pilot --locked` ✅
3. `cargo test -p pilot --locked test_agorg_reconcile_api_` ✅
4. `./scripts/prepush_gate.sh` ✅

### Iteration Update (2026-02-28 17:16 EST)

Completed in this iteration:

1. **Wave F execution started with policy class model**
   - Added `issue_class` on reconcile issues.
   - Added `class_counts` map on reconcile reports.
2. **Branch/dependency policy checks added to reconcile**
   - Parent relationship mismatch checks.
   - Duplicate/self child-link checks.
   - Branch naming policy checks against AGOrg branch preferences and allowed prefixes.
3. **UI surfacing added**
   - Dashboard and AGOrg policy cards now show issue class counts in dedicated outputs.

Verification completed:

1. `node -c crates/pilot/src/pilot_ui.js` ✅
2. `cargo check -p pilot --locked` ✅
3. `cargo test -p pilot --locked test_duplicate_` ✅
4. `cargo test -p pilot --locked test_agorg_reconcile_api_` ✅

### Iteration Update (2026-02-28 17:20 EST)

Completed in this iteration:

1. **Wave F hard-close UI controls**
   - Added class filter selectors and issue drill-down navigation to both Dashboard and AGOrg policy surfaces.
   - Added filtered issue list and selected-issue detail outputs.
2. **Report-to-UI sync**
   - Policy report, dry-run, and apply flows now all hydrate class-filter/drill-down state from reconcile report payloads.

Verification completed:

1. `node -c crates/pilot/src/pilot_ui.js` ✅
2. `cargo check -p pilot --locked` ✅
3. `cargo test -p pilot --locked test_agorg_reconcile_api_` ✅
4. `cargo test -p pilot --locked test_duplicate_` ✅
5. `./scripts/prepush_gate.sh` ✅

### Iteration Update (2026-02-28 18:05 EST)

Completed in this iteration:

1. **Wave G backend + dashboard wiring**
   - Added `POST /api/agorg/dashboard_overview` and dashboard AGOrg overview bindings.
   - Added dashboard AGOrg action-contract flow using existing Codex contract lifecycle.
2. **AGOrg local contract dispatch**
   - Added local dispatch path for `api.agorg.policy_report` and `api.agorg.reconcile_apply`.
   - Preserved read-only safety: mutation guard applies only when `dry_run=false`.
3. **Wave G hardening tests**
   - Added unit tests for mutation classification.
   - Added unit tests for reconcile class-filtered prune planning.
   - Added unit tests for conformance score bounds.

Verification completed:

1. `node -c crates/pilot/src/pilot_ui.js` ✅
2. `cargo check -p pilot --locked` ✅
3. `cargo test -p pilot --locked test_agorg_reconcile_api_` ✅
4. `cargo test -p pilot --locked test_command_requires_mutation` ✅
5. `cargo test -p pilot --locked test_filter_prune_paths_by_class` ✅

### Iteration Update (2026-02-28 18:15 EST)

Completed in this iteration:

1. **Wave G event/artifact linkage**
   - AGOrg policy report now emits timeline event with `artifact_path`.
   - AGOrg reconcile dry-run/apply now persist action artifacts and return `artifact_path` in responses.
   - AGOrg reconcile events now include `artifact_path` for direct timeline-to-artifact traceability.

Verification completed:

1. `cargo check -p pilot --locked` ✅
2. `cargo test -p pilot --locked test_agorg_reconcile_api_` ✅
3. `./scripts/prepush_gate.sh` ✅

### Iteration Update (2026-02-28 18:25 EST)

Completed in this iteration:

1. **Wave G operator UX hardening**
   - Added `OPEN ARTIFACT` action to Dashboard AGOrg Action Contract output panel.
   - Added `OPEN ARTIFACT` action to Operation Detail panel for selected timeline entries.
2. **Artifact path resolution hardening**
   - Timeline artifact resolution now checks selected operation raw events first, then audit fallback.
   - Contract output artifact opener now parses `artifact_path` from direct or nested response payloads.

Verification completed:

1. `node -c crates/pilot/src/pilot_ui.js` ✅
2. `cargo check -p pilot --locked` ✅
3. `cargo test -p pilot --locked test_agorg_reconcile_api_` ✅
4. `./scripts/prepush_gate.sh` ✅

### Iteration Update (2026-02-28 18:40 EST)

Completed in this iteration:

1. **Wave G hard-close**
   - Added timeline card `ARTIFACT` badge for artifact-backed operations.
   - Confirmed dashboard-first AGOrg report/reconcile flow is fully operable without tab hopping.
2. **Wave H execution started**
   - Added temporary component inventory API and dashboard surface.
   - Removed `TODO` phrasing from hierarchy helper; replaced with explicit governed relationship-editor guidance.

Verification completed:

1. `node -c crates/pilot/src/pilot_ui.js` ✅
2. `cargo check -p pilot --locked` ✅
3. `cargo test -p pilot --locked test_agorg_reconcile_api_` ✅
4. `./scripts/prepush_gate.sh` ✅

### Iteration Update (2026-02-28 18:55 EST)

Completed in this iteration:

1. **Wave H inventory evidence loop**
   - Added backend export endpoint for temporary component inventory artifacts.
   - Added dashboard button `Export Inventory Artifact`.
2. **Timeline traceability hardening**
   - Timeline now ingests dashboard/action events directly.
   - Artifact-backed events appear with `ARTIFACT` badge and openable path in Operation Detail.

Verification completed:

1. `node -c crates/pilot/src/pilot_ui.js` ✅
2. `cargo check -p pilot --locked` ✅
3. `cargo test -p pilot --locked test_agorg_reconcile_api_` ✅
4. `./scripts/prepush_gate.sh` ✅

### Iteration Update (2026-02-28 19:05 EST)

Completed in this iteration:

1. **Wave H hard-close checklist**
   - Added checklist API + Dashboard button `Run Checklist`.
   - Added checklist output panel and boot-time checklist refresh.
2. **Wave H evidence continuity**
   - Checklist and export actions now flow into timeline events with clear pass/fail semantics.

Verification completed:

1. `node -c crates/pilot/src/pilot_ui.js` ✅
2. `cargo check -p pilot --locked` ✅
3. `cargo test -p pilot --locked test_agorg_reconcile_api_` ✅
4. `./scripts/prepush_gate.sh` ✅

### Iteration Update (2026-02-28 19:20 EST)

Completed in this iteration:

1. **Wave I acceptance matrix execution path**
   - Added script `scripts/wave_acceptance_matrix.sh` with quick/full profiles.
   - Added API endpoint `POST /api/system/acceptance_matrix/run`.
   - Added Dashboard card + artifact open flow.
2. **Branding & Hardening**
   - Updated Pilot tagline to "Orchestrating Autonomous Evolution".
   - Hardened matrix parsing to tolerate mixed stdout.
   - Fixed `ui-smoke` check regression.
3. **Traceability**
   - Matrix runs emit timeline events with `artifact_path`.

Verification completed:

1. ArqonPilot CI (Run 22527214142) 🟢 GREEN
2. `./scripts/wave_acceptance_matrix.sh --wave I --profile full` ✅

### Iteration Update (2026-02-28 20:10 EST)

Completed in this iteration:

1. **Wave I API parser hardening**
   - `POST /api/system/acceptance_matrix/run` now parses JSON from mixed stdout instead of requiring stdout to be pure JSON.
   - Added helper `parse_json_from_mixed_output` to tolerate runtime prefix lines before JSON payloads.
2. **Regression tests added**
   - `test_parse_json_from_mixed_output_plain_json`
   - `test_parse_json_from_mixed_output_with_prefix_line`
3. **Wave I full-profile CLI evidence**
   - `./scripts/wave_acceptance_matrix.sh --wave I --profile full` reports `ok=true`, `checks=4`, `failed_checks=[]`.

Verification completed:

1. `cargo test -p pilot --locked test_parse_json_from_mixed_output_` ✅
2. `cargo check -p pilot --locked` ✅
3. `./scripts/wave_acceptance_matrix.sh --wave I --profile full` ✅

### Iteration Update (2026-02-28 20:40 EST)

Completed in this iteration:

1. **TD Wave 1 hardening (placeholder removal)**
   - Replaced placeholder `ship_test` assertions with real `SemVer::from_cargo_toml` checks.
   - Replaced placeholder `vector_test` flow with real vector insert + search assertions.
   - Replaced generated scaffold test body in `pilot-create` (removed TODO + `assert!(true)` template).
2. **TD Wave 2 hardening (panic safety)**
   - Hardened `agorg::edit_relationship` to avoid production `unwrap()` panics on malformed TOML table shapes.
   - Added regression test for malformed `[tool.arqon]` table handling.
3. **TD Wave 3 start (shim consolidation)**
   - Centralized ArqonBus shim shell command generation in UI backend (`bus_shim_command`) to remove duplicated hard-coded command strings.

Verification completed:

1. `cargo test -p pilot --locked --test ship_test --test vector_test` ✅
2. `cargo test -p pilot --locked test_edit_relationship_handles_malformed_tool_table` ✅
3. `cargo check -p pilot --locked` ✅

### Iteration Update (2026-02-28 21:30 EST)

Completed in this iteration:

1. **Wave I hard-close confirmation captured**
   - Full profile acceptance matrix run completed with all checks passing and artifact evidence recorded.
2. **TD Wave 3 hard-close**
   - Added shared shim runtime adapter `crates/pilot/src/shim_runtime.rs`.
   - Consolidated shim lifecycle command construction for both `main.rs` and `serve_ui.rs`.
3. **TD Wave 4 hard-close**
   - Replaced temporary-component checklist text probes with semantic payload-contract validation.
   - Enabled command-lane UI smoke checks by default in script and CI (`PILOT_UI_SMOKE_INCLUDE_COMMANDS=1`).

Verification completed:

1. `cargo test -p pilot --locked test_bus_shim_` ✅
2. `cargo test -p pilot --locked test_parse_json_from_mixed_output_` ✅
3. `cargo check -p pilot --locked` ✅

## Recommended Next Session Priority

1. **Wave J planning and execution start** — define and begin post-acceptance roadmap now that Wave I and TD Wave 3/4 are closed.
2. **B.4 duplicate-merge UX hard-close** — implement winner/loser merge preview controls in reconcile UI and validate dry-run/apply report parity.
3. **Temporary component inventory governance** — keep inventory/report/checklist in sync with runbook and gotcha registry on every iteration.
