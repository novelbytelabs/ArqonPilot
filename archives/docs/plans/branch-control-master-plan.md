# Arqon Pilot Branch Control Master Plan

> Archived detailed plan snapshot (2026-03-02): BC-1 through BC-6 are hard-closed. BC-7 and BC-8 remain open and are now tracked in `docs/PRODUCTIONIZE.md`.

## AI Handoff Summary (Copy/Paste First)

Arqon Pilot needs a **single, safe, secure, and powerful Branch Control system** that replaces fragmented Branch/Multi/Dashboard branch actions with one coherent control plane for AGOrg-scoped fleet operations.  
Core constraints are frozen: **Rust core lane 1.82.0**, **packaging lane 1.88.0**, **protobuf 4.25.8**.  
Implementation must be production-grade: no placeholders, no silent failures, no invisible operations, no irreversible destructive actions without explicit confirmation and audit evidence.

This plan is the execution source of truth for Branch Control hardening and unification.

## Why This Exists

Current branch operations are useful but fragmented and too easy to misuse. The operator needs one high-trust interface that:

- Sees fleet branch state instantly.
- Previews before mutating.
- Executes dependency-aware operations deterministically.
- Produces audit artifacts for every action.
- Surfaces actionable failures clearly.

## Scope

In scope:

- Branch tab redesign into a unified **Branch Control** system.
- Consolidation with overlap currently in Multi and Dashboard quick branch actions.
- AGOrg scope-aware targeting (fleet, group, tags, per-repo selection).
- Safety policy, confirmations, and rollback-ready evidence.
- Observability, timeline, and per-operation drill-down.

Out of scope for this plan:

- Non-branch Oracle/Heal feature expansion (covered elsewhere).
- Full CI architecture redesign (only branch-related CI hooks and parity checks here).

## Current State (Ground Truth)

Relevant files:

- UI frontend logic: `crates/pilot/src/pilot_ui.js`
- UI backend routes/contracts: `crates/pilot/src/serve_ui.rs`
- CLI command wiring: `crates/pilot/src/main.rs`
- Branch engine: `crates/pilot-branch/src/lib.rs`
- Multi-repo orchestration engine: `crates/pilot-multi/src/lib.rs`
- AGOrg plan and context: `docs/PRODUCTIONIZE.md`
- Gotchas registry: `docs/gotcha-registry.md`

Current issues to fix:

1. Branch actions are difficult to trust operationally because result visibility is inconsistent.
2. Branch, Multi, and Dashboard duplicate branch-adjacent actions and split operator intent.
3. Status fidelity is too low (needs ahead/behind/divergence/conflict risk context).
4. Safety guardrails for destructive actions are not explicit enough in UI flow.
5. Two-phase operation model (preview/execute) must be universal and explicit.

## Non-Negotiable Constraints

- Toolchain freeze:
  - Core lane Rust: `1.82.0`
  - Packaging lane Rust: `1.88.0`
  - Protobuf: `4.25.8`
- All mutating operations:
  - Preview first, execute second.
  - Emit artifact with structured summary.
  - Be attributable in activity timeline.
- AGOrg scope:
  - Branch operations must clearly show active scope and selected targets.
  - No hidden scope assumptions.
- No hidden shims/placeholders:
  - Any shim must be documented in runbook + gotcha registry with removal path.

## Design Principles

1. **One Surface, Many Depths**  
   One Branch Control home for 80% of branch work, with advanced sections expandable.

2. **Safe by Default**  
   Dry-run preview is default; destructive operations require explicit confirmation.

3. **Deterministic Execution**  
   Dependency-aware staged execution is first-class, not an afterthought.

4. **Operator Clarity**  
   Every action answers: what was targeted, what changed, what failed, what next.

5. **Evidence-Centric**  
   Every run produces machine-readable artifacts and timeline events.

## Target UX Architecture

### 1) Fleet Branch Matrix (Top Priority)

A live matrix for selected AGOrg scope:

- Columns:
  - Repo
  - Current branch
  - Target branch alignment
  - Ahead/behind vs base
  - Working tree state (clean/dirty)
  - Protected branch flag
  - Selection checkbox
- Filters:
  - Group
  - Tags
  - Search by name/path
- Bulk selection:
  - Select all visible
  - Select by status (behind/dirty/off-target)

### 2) Operation Workbench

Operations grouped under tabs or segmented controls:

- Create Branch
- Switch/Checkout Fleet
- Sync (base into target)
- Prune (safe)
- Reconcile Branch Drift
- Dependency-Staged Apply (migrated from Multi)

Each operation has:

- Labeled fields and helper text.
- Preview button.
- Execute button (disabled until preview generated for matching payload hash).
- Optional continue-on-failure with explicit warning.

### 3) Dependency Graph + Stage Plan

- Embedded DAG view for selected cohort.
- Stage plan preview:
  - Stage 1, Stage 2, ...
  - Repos per stage.
  - Risk annotations (conflict likelihood, protected branch touch).

### 4) Activity Timeline + Drill-Down

- Chronological event stream (success/fail/running/artifact).
- Expandable row for full payload and execution summary.
- Direct links to artifact paths.
- One-click copy for command replay.

### 5) Macro-Ready Guided Workflow Rail (No Auto-Execute)

- Dashboard sequence rail must be clickable and keyboard-accessible.
- Clicking a rail item must:
  - open the relevant tab,
  - show guided next steps in output/timeline,
  - **not execute commands**.
- The rail acts as a future macro handoff surface (chatbot/macro engine), but current behavior remains hint-only.
- Initial guided rails:
  - `Status -> Bus Health -> Oracle Query -> Heal Plan -> Heal Run`
  - `Branch Preview -> Multi Status -> DAG -> Staged Apply`
  - `Push Safe -> Timeline Verify`

### 6) Accessibility + Intuitive UX Hardening

- Accessibility baseline:
  - all controls keyboard reachable (`tab`, `enter`, `space`) with visible focus.
  - icon-only controls require title/aria labeling.
  - status chips require textual state, not color-only semantics.
  - contrast and font size must meet practical readability thresholds on dark theme.
- Intuitive UX baseline:
  - remove duplicate/legacy inputs that can conflict with matrix targeting.
  - provide one source of truth for scope/filter/selection per operation family.
  - every panel should include plain-language helper text with expected outcome.
  - empty states must explain cause and next action (for example no scope, no repos, registry empty).
- Progressive disclosure:
  - basic flow visible first, advanced options collapsed by default.
  - destructive actions isolated behind explicit confirmation.
- Operator confidence:
  - always surface “what will happen” before execute and “what happened” after execute.
  - keep result summaries concise with artifact links for deep detail.

## Safety and Security Model

### Guardrails

- Protected branch policy (`main`, `dev`, release patterns).
- Destructive operation confirmation:
  - Type-to-confirm for prune/delete/reset-class actions.
- Mutation mode control:
  - UI read-only unless explicit mutation flag is enabled at serve startup.
- Scope guard:
  - Must display active AGOrg and operation target set before execute.

### Audit and Evidence

Every execution writes:

- `command`
- `dry_run` or `apply`
- `target selection`
- `stage model` (if staged)
- `failures`
- `artifact path`
- `timestamp`

Evidence is surfaced in timeline and exportable.

### Threat/Failure Classes (Branch Control)

- Wrong-scope mutation.
- Protected branch destructive action.
- Partial fleet mutation without isolation reporting.
- Invisible failures due to output routing bugs.
- Local pass / CI fail mismatch due to lock/toolchain drift.

## Gotcha Mapping (Must Be Enforced in Branch Control)

- `G-001`, `G-002`, `G-005`: Lock/toolchain drift handling.
- `G-003`, `G-013`: DNS flaps and retry expectations.
- `G-004`, `G-008`: Opaque push errors and auth challenge interpretation.
- `G-015`: JS syntax fragility; enforce JS syntax checks in branch-control UI changes.
- `G-018`, `G-019` (scope-related): avoid hidden scope rejections and null-scope blind spots.

See canonical registry: `docs/gotcha-registry.md`.

## Tab Interop Contract (Dependencies/Branch/Multi)

This contract defines how `Dependencies`, `Branch`, and `Multi` cooperate without duplicating authority.

### Ownership Boundaries

1. `Branch` (authoritative for branch lifecycle):
   - Fleet branch status matrix.
   - Branch create/switch/sync/prune/reconcile.
   - Dependency-staged branch apply UX (DAG/stages) as the primary branch mutation path.
2. `Dependencies` (authoritative for toolchain/policy/gate/push readiness):
   - Policy verification, hook verification, lock drift diagnostics/repair.
   - Pre-push gate orchestration and push-safe readiness status.
   - Build/test/toolchain drift evidence.
3. `Multi` (authoritative for non-branch cross-repo orchestration):
   - Cohort registration/listing.
   - Non-branch DAG/order/PR planning and orchestration.
   - Cross-repo coordination primitives not tied to branch mutation.

### Handoff Rules

1. `Branch -> Dependencies`:
   - After any mutating branch operation, Branch emits a readiness request for Dependencies to compute:
     - policy
     - hook
     - gate
     - push readiness.
2. `Dependencies -> Branch`:
   - Dependencies returns machine-readable readiness state used to gate “Push Safe” from Branch.
3. `Multi -> Branch`:
   - If a Multi flow results in branch mutation intent, Multi must route user to Branch staged apply flow (or call same backend contract).
4. `Branch -> Multi`:
   - If operator requests non-branch orchestration (for example PR planning-only), Branch links to Multi without duplicating execution logic.

### Shared Contract Rules

1. Shared payload model fields:
   - `agorg_scope`
   - `group`
   - `tags`
   - `selected_repos`
   - `dry_run` / `apply`
   - `command`
2. Shared event schema:
   - `status` (`running|completed|failed`)
   - `summary`
   - `artifact_path`
   - `failures`
   - `timestamp`
3. Shared preview/execute model:
   - Mutations require preview token/hash before execute.
   - Scope change invalidates stale execute token.

### UI Consistency Rules

1. Same status chips semantics across tabs (`unknown|running|pass|fail`).
2. Same artifact drill-down interaction pattern.
3. Same explicit scope banner and selected-target summary before mutation.
4. No “silent run” buttons; every action must surface result state in tab-local output/timeline.

### De-duplication Policy

1. If an action is implemented in Branch, do not create a separate mutating implementation in Multi/Dependencies.
2. Multi/Dependencies may expose shortcuts, but they must call the same backend contracts and emit the same evidence schema.
3. Dashboard quick actions are orchestration shortcuts only; they cannot introduce divergent payload shape or safety bypass.

## Implementation Roadmap

### Wave BC-1: Foundation Consolidation

- Merge duplicated branch actions into one Branch Control backend surface.
- Ensure all existing branch/multi/dashboard branch actions route through shared execution helpers.
- Add explicit operation response schema (status, summary, failures, artifact).

### Wave BC-2: Matrix + Selection Layer

- Implement Fleet Branch Matrix with filters and per-repo selection.
- Add selection model to payloads (selected repos override group/tag).
- Add alignment and working-tree indicators.

### Wave BC-3: Universal Preview/Execute Contract

- Enforce preview hash requirement before execute for all mutations.
- Add execute guard if active scope changed since preview.
- Add confirmation flow for destructive actions.

### Wave BC-4: Dependency-Aware Stage Control

- Inline DAG render + stage controls in Branch Control.
- Move staged apply UX from Multi into Branch Control.
- Keep Multi for non-branch orchestration only (or deprecate after parity).

### Wave BC-5: Timeline, Artifacts, and Drill-Down

- Add durable timeline view scoped by AGOrg and operation family.
- Add event detail drawer with payload/result/artifact links.
- Add export bundle for branch operation evidence.

### Wave BC-6: Policy and Security Hardening

- Protected branch policy UI and enforcement.
- Branch template policy (naming conventions and lint checks).
- Mandatory confirmation rules for destructive ops.

### Wave BC-7: CI/Push Parity Integration

- Integrate gate/push status chips with explicit state machine:
  - Unknown / Running / Pass / Fail.
- Add one-click transition from branch operation to push-safe pipeline.
- Add branch operation “ready for push” diagnostic checklist.

### Wave BC-8: Final Hard-Close

- Acceptance matrix for Branch Control across:
  - single repo
  - multi repo
  - staged dependency operations
  - destructive op confirmation flow
  - failure isolation and evidence
- Documentation hard-close and operator runbook updates.

## Testing Requirements

### Unit Tests

- Payload validation, preview hash matching, scope guards, protected branch rules.

### Integration Tests

- Branch operations across mock or fixture multi-repo sets.
- Stage execution ordering and failure isolation.

### End-to-End Tests

- UI-driven preview/execute flows, timeline evidence checks.

### Regression Tests

- Existing branch/multi/dashboard workflows remain functional or intentionally migrated.

### Adversarial Tests

- Wrong-scope mutation attempts.
- Protected branch destructive attempts.
- Mid-run repo failure and continue-on-failure behavior.

## Definition of Done (Excellent / Release-Grade)

Branch Control is only done when **all** are true:

1. **Unified UX**
   - Branch tab is the single authoritative branch operations surface.
   - Multi/Dashboard duplication is removed or explicitly delegated with clear links.

2. **Operational Clarity**
   - Every operation has labeled inputs, helper text, preview, execute, and visible result.
   - Operator can identify affected repos before mutation.

3. **Safety Guarantees**
   - Mutations require preview.
   - Destructive actions require explicit confirmation.
   - Protected branch policy enforced and test-covered.

4. **Dependency Intelligence**
   - DAG/stage-aware branch operations are available in Branch Control.
   - Stage plan is visible before execute.

5. **Evidence + Audit**
   - Every execution emits an artifact and timeline entry with drill-down.
   - Failures are isolated per repo/stage with actionable messages.

6. **Scope Correctness**
   - AGOrg scope is always visible.
   - Scope changes invalidate stale preview executes.

7. **Quality Gates**
   - Unit, integration, e2e, regression, adversarial tests pass for Branch Control pathways.
   - JS syntax check included for `pilot_ui.js` changes.

8. **Docs Complete**
   - Developer guide updated with architecture and contracts.
   - Operator runbook updated with workflows and failure recovery.
   - Gotcha registry updated for any new branch-control failure class.

9. **No Hidden Technical Debt**
   - No placeholders/stubs for core Branch Control flows.
   - Any remaining shim is documented with reason and removal plan.
10. **Usability Without Docs**
   - New operator can complete core branch workflow by using in-product guidance rails and inline helper text only.
11. **Accessibility Baseline**
   - Core branch and dashboard flows are keyboard-usable with visible focus and non-color-only status cues.

## Relevant File Map

- `crates/pilot/src/pilot_ui.js` (Branch Control frontend behaviors)
- `crates/pilot/src/serve_ui.rs` (UI API handlers, contracts, timeline/event plumbing)
- `crates/pilot/src/main.rs` (command routing/serve setup)
- `crates/pilot-branch/src/lib.rs` (branch primitives)
- `crates/pilot-multi/src/lib.rs` (dependency-aware orchestration primitives)
- `docs/gotcha-registry.md` (failure signatures + recoveries)
- `docs/operator-runbook.md` (operator workflows)
- `docs/developer-guide.md` (internal architecture/extension points)
- `docs/testing-strategy.md` (test matrix and policy)
- `docs/PRODUCTIONIZE.md` (program-level roadmap tie-in)

## Recommended Next Session Priority

1. Implement Wave BC-1 and BC-2 first (consolidation + matrix + selection).
2. Add preview/execute hash contract and destructive confirmations (BC-3).
3. Immediately wire timeline drill-down and artifacts (BC-5) to avoid invisible failures.
4. Only then proceed with DAG/stage UX hardening (BC-4) and policy hardening (BC-6).

## Implementation Status (Live)

- 2026-03-02: Started BC-1 UX hardening slice.
  - Branch tab now has labeled controls.
  - Branch create/sync/prune now expose explicit `Preview` and `Execute` actions.
  - Branch status has its own operation chip.
  - Branch actions now write to a branch-local output panel (`branch-out`) instead of relying on hidden/global output only.
  - `run()` now supports per-action output targets (`opts.outputEl`) to reduce cross-tab output ambiguity.
- 2026-03-02: BC-2 hard close completed.
  - Added Fleet Branch Matrix UI with filters (`group`, `tags`, `search`, `base branch`), ahead/behind visibility, clean/dirty, and on-target indicator.
  - Added per-repo selection model (checkbox rows, select visible, clear selection).
  - Added backend APIs:
    - `POST /api/branch/matrix`
    - `POST /api/branch/run`
  - Branch operations now support `selected_repo_ids` and execute on exact repo subsets (within active AGOrg scope).
  - Active AGOrg scope is enforced for matrix and run endpoints; out-of-scope repos are excluded.
- 2026-03-02: BC-3 hard close completed.
  - Server now enforces preview->execute contract for mutating branch actions:
    - execute requires valid `preview_token`
    - token binds to active AGOrg scope + canonical execute payload
    - token mismatch/expiry returns precondition failure with remediation
  - UI invalidates stale previews when filters/selection/input scope changes.
  - Prune execute is gated by typed confirmation (`PRUNE`) before mutation.
- 2026-03-02: BC-5 hard close completed.
  - Branch output replaced with structured HTML activity log entries (no raw JSON-only text output).
  - Each log item supports drill-down (`Show JSON`) and artifact-open action when `artifact_path` is present.
  - Added explicit `Clear Logs` control and stateful log retention limit (`1..100`, persisted).
- 2026-03-02: BC-4 hard close completed.
  - Added DAG preview and staged apply controls directly in Branch tab as primary branch orchestration surface.
  - Branch tab can now run:
    - `pilot.multi.dag` (preview)
    - `pilot.multi.apply` (preview + execute)
  - Branch-scoped filters (`group`/`tags`) drive DAG/staged apply execution context.
- 2026-03-02: BC-6 hard close completed.
  - Protected branch policy enforcement:
    - blocks create/apply execute against `main|master|dev|release*`.
  - Branch naming policy enforcement for mutating create/apply executes:
    - required format: `(feat|fix|docs|test|refactor|chore|perf)/kebab-case`.
  - Destructive confirmation enforcement:
    - prune execute requires typed `PRUNE` in UI modal and backend `confirm_phrase=PRUNE`.
  - Added targeted unit tests for policy helpers/violations.
- 2026-03-02: BC-2 post-hard-close stabilization completed.
  - Removed legacy Create card filter inputs (`branch-group`, `branch-tags`) to enforce single filter source.
  - Matrix filters now come only from matrix header (`group`, `tags`, `search`, `base`).
  - Added matrix source metadata and UI chip (`registry|bootstrapped|autodiscovered|empty|error`).
  - Added auto-fallback chain when matrix initially resolves to zero rows:
    1. bootstrap from AGOrg AGO records
    2. AGOrg discover+import
    3. bootstrap retry
  - Resolved scope-root trap for sibling AGOs by using both AGOrg `root_path` and `master_path` for in-scope checks.
  - Resolved selection/filter intersection trap: explicit row selection is authoritative target set.
- 2026-03-02: UX guidance rail enhancement completed (non-executing).
  - Dashboard workflow strip is now clickable and keyboard-accessible (button semantics).
  - Click behavior is hint-only (tab jump + guidance output), with no command execution.
  - Intended as macro/chatbot handoff surface for future automation.
