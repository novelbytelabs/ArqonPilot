# AGOrg Control Plane Plan

This document captures the long-term AGOrg vision and the implementation plan so it is never lost across sessions.

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

## Existing Contract (Already Present)

The current relationship declaration exists in project metadata:

```toml
[tool.arqon.relationships]
parent = "Arqon"
children = []
```

Arqon Pilot should treat this as authoritative repo-level relationship metadata when discovering and registering AGOrgs.
In this model, relationship metadata belongs to repos (AGOs), where `parent` points to the owning AGOrg.

## Product Requirements

## 1) AGOrg Scope Control

1. Add an `AGOrg` section in the Control Panel.
2. Include a path browse/input field and a `Load Scope` action.
3. Selected AGOrg becomes the scope boundary for Dashboard, Oracle, Heal, Dependencies, Branch, Multi, and Telemetry.

## 2) AGOrg CRUD

1. Create AGOrg record.
2. Read/list AGOrg records.
3. Update AGOrg metadata/settings.
4. Delete AGOrg record (with confirmation + non-destructive defaults).

### Create AGOrg Project (Required UX)

1. Add a `Create AGOrg Project` flow with:
     - AGOrg name
     - root path (browse button + input field)
     - optional parent AGOrg selector (for nested AGOrg)
     - initial scan toggle (`AutoScan hierarchy now`)
2. On create, persist AGOrg and optionally execute discovery immediately.
3. Show preview of discovered AGOrg/AGO hierarchy before final save.

## 3) AGOrg Discovery

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

## 3.1) Reconciliation and AGOrg Policy Conformance (Tomorrow Focus)

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

## 4) State, Preferences, and Profiles

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

## UX Plan

## New top-level surfaces

## 1) AGOrg Management (3-Panel System)

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

## Guardrails

1. No mutation without visible active AGOrg scope.
2. Scope mismatch warnings when running repo-specific actions outside current AGOrg.
3. Non-destructive defaults (`dry-run`, explicit apply toggles).

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

## API/Command Surface Plan

Add new commands/endpoints:

1. `pilot agorg create`
2. `pilot agorg list`
3. `pilot agorg update`
4. `pilot agorg delete`
5. `pilot agorg use`
6. `pilot agorg discover --root <path>`
7. `pilot agorg show --active`
8. `pilot agorg tree --root <agorg-id|name>`
9. `pilot agorg create-project --name <name> --root <path> [--parent <agorg>] [--autoscan]`

Bus/UI mirrors:

1. `/api/agorg/*` endpoints
2. `pilot.agorg.*` bus contract namespace

## Rollout Waves

## Wave A - Foundation

1. AGOrg data model + persistence.
2. Active scope selection.
3. Default AGOrg load on startup.

## Wave B - CRUD + Discovery

1. Full AGOrg CRUD in UI + CLI.
2. Directory discovery flow.
3. Registration review/approval step.
4. Create AGOrg Project wizard with optional autoscan.
5. Tree view for AGOrg/AGO hierarchy.
6. Scan depth controls in AGOrg UI and discovery APIs.

## Wave C - Full Scope Enforcement

1. All tabs execute inside active AGOrg scope.
2. Scope-aware branch/multi/dependency operations.
3. Header and telemetry include AGOrg context.
4. AGOrg link validation blocks circular loops.

## Wave D - Profiles and Multi-Instance Readiness

1. Per-AGOrg preferences/profile settings.
2. Fast scope switching.
3. Concurrent Pilot instances with isolated AGOrg contexts.

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

## Dogfooding Test Case (Initial)

Use `~/Projects/arqon/Arqon` as the first AGOrg loaded in the system.

1. Register as AGOrg default scope.
2. Run autoscan discovery.
3. Verify expected children include AGOs such as `ArqonBus`, `ArqonCore`, and `ArqonPilot`.
4. Persist and reload Control Panel; confirm AGOrg auto-load and scope restoration.
