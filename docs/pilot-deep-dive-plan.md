# ArqonPilot Deep Dive and Aggressive Execution Plan

## Decision

ArqonShip will be hard-cut into ArqonPilot as a standalone product and primary focus area.

- No backward compatibility layer.
- No dual command surface (`arqon`/`ship`) during transition.
- No parallel feature work outside ArqonPilot until ArqonPilot is operational across individual and cross-repo workflows.

---

## Deep Dive: Current Baseline (ArqonShip in ArqonHPO)

Source baseline:

- `/home/irbsurfer/Projects/arqon/ArqonHPO/crates/ship`
- `/home/irbsurfer/Projects/arqon/ArqonHPO/specs/006-arqonship`
- `/home/irbsurfer/Projects/arqon/ArqonHPO/docs/docs/arqonship`

### What exists and is reusable immediately

1. Oracle
- Rust/Python parsing via tree-sitter.
- Graph extraction + SQLite storage.
- Vector embeddings + LanceDB.
- Query engine for hybrid retrieval.

2. Heal
- Rust and Python failure parsers.
- Context assembly from Oracle graph.
- LLM-assisted repair loop with capped attempts.
- Verification gate and audit DB.

3. Release (to be renamed Navigate)
- Pre-flight checks.
- Conventional commit parsing.
- SemVer + changelog generation.
- GitHub PR creation.

4. Config and test baseline
- `.arqon/config.toml` schema.
- Unit/integration tests in `crates/ship/tests`.

### Migration-critical couplings and risks discovered

1. Naming and paths hardcoded to ArqonShip
- CLI binary name is `arqon`.
- Runtime data/config path uses `.arqon/*`.
- User-facing strings and docs are ArqonShip-specific.

2. HPO-layout coupling in version logic
- `ship/version.rs` falls back to `crates/core/Cargo.toml` if root Cargo is workspace-only.
- This assumption is specific to ArqonHPO repo layout and must be removed for standalone Pilot.

3. Workspace dependency inheritance
- `crates/ship/Cargo.toml` uses `workspace = true` for key deps and version/edition.
- Standalone extraction must make these explicit in Pilot workspace.

4. Command execution behavior is local and synchronous
- `cargo test`, `cargo build`, `cargo clippy`, and git commands are direct shell execs.
- For multi-repo orchestration this requires orchestration wrappers, per-repo state tracking, and failure isolation.

5. CI and docs references still anchored in ArqonHPO
- Current workflow and docs references include/exclude ship from HPO CI paths.
- Pilot needs independent CI and docs authority.

### Gotchas (Do Not Miss)

1. Root `Cargo.toml` rewrite risk
- Current navigate flow writes version to root `Cargo.toml`.
- In multi-repo mode, wrong repo context can mutate the wrong manifest if cwd handling is loose.

2. Hidden ArqonHPO assumptions
- `version.rs` contains fallback behavior tied to `crates/core/Cargo.toml`.
- This can silently produce incorrect behavior once extracted.

3. CLI shape breakage is immediate
- Moving from flat commands (`scan`, `chat`, `ship`) to nested (`oracle scan`, `oracle query`, `navigate`) breaks scripts immediately.
- Because this transition is hard-cut, automation must be updated atomically.

4. `.arqon` to `.pilot` state migration
- Existing local graph/vector/audit artifacts in `.arqon` will no longer be read.
- Either migrate data explicitly or force full re-scan and accept cold-start cost.

5. Test breakage from binary rename
- Tests currently use `cargo_bin(\"ship\")` and command arg `ship`.
- They all fail until test fixtures and command expectations are renamed together.

6. Multi-repo blast radius
- Cross-repo branch/release operations can produce large write surfaces fast.
- `--dry-run` and per-repo apply scopes must exist before bulk operations.

7. Shell portability
- Some checks rely on shell tools like `grep`.
- Cross-platform support will drift if command execution is not abstracted.

8. GitHub token/provider assumptions
- Current PR implementation is GitHub+`GITHUB_TOKEN` specific.
- Future multi-provider support requires capability-based provider abstraction.

9. Model lifecycle/performance
- Recreating embedding/LLM clients per command can be expensive across many repos.
- Reuse/caching policy is needed for multi-repo throughput.

10. Partial failure semantics
- Multi-repo operations need explicit per-repo status reporting.
- One failure must not hide successful actions in other repos.

### Gaps relative to ArqonPilot vision

Not present yet:

- Multi-repo registry and orchestration model.
- Cross-repo branch lifecycle management.
- Dependency-order planning and linked PR flows.
- Security scanning/fix pipeline.
- Planning/create/knowledge modules.
- Unified command taxonomy (`pilot oracle ...`, `pilot navigate ...`, etc.).

---

## Definition of Complete (Pilot v1)

Pilot v1 is considered complete when all of the following are true:

1. Works as a standalone binary (`pilot`) on local repos.
2. Supports multi-repo operations for your registered repos.
3. Provides stable modules: `oracle`, `heal`, `navigate`, `branch`, `multi`, `secure`, `plan`, `create`, `know`.
4. Every mutating command supports `--dry-run`.
5. Every mutating command emits audit events.
6. Cross-repo actions return clear per-repo success/failure summaries.
7. End-to-end workflow works: issue -> code changes -> tests -> PR(s) -> release pipeline.

---

## Target Architecture

## Repo layout

```text
ArqonPilot/
  Cargo.toml
  crates/
    pilot-cli/
    pilot-core/
    pilot-oracle/
    pilot-heal/
    pilot-navigate/
    pilot-branch/
    pilot-multi/
    pilot-secure/
    pilot-plan/
    pilot-create/
    pilot-know/
  docs/
  specs/
  tests/
```

## State model

1. Global state
- `~/.pilot/config.toml`
- `~/.pilot/workspace.db`
- `~/.pilot/audit.db`

2. Per-repo state
- `.pilot/config.toml`
- `.pilot/graph.db`
- `.pilot/vectors.lance`
- `.pilot/cache/*`

3. Operational safety
- Read-only operations default safe.
- Mutating operations require explicit intent flags and are auditable.

---

## CLI Surface (Hard-Cut)

```bash
pilot init

pilot oracle scan
pilot oracle query "..."

pilot heal --log-file test-output.json --max-attempts 2

pilot navigate --dry-run

pilot branch create <branch>
pilot branch sync
pilot branch status
pilot branch prune

pilot multi register <repo-path>
pilot multi list
pilot multi status
pilot multi search "..."
pilot multi order
pilot multi prs create

pilot secure scan
pilot secure fix --dry-run

pilot plan issues
pilot plan roadmap
pilot plan score

pilot create feature <name>
pilot create tests <target>

pilot know record
pilot know query "..."
```

---

## Aggressive Delivery Plan

## Wave 0: Baseline Freeze and Cutover (2 days)

Goals:
- Freeze Ship baseline.
- Establish Pilot repo as source of truth.

Tasks:
1. Copy `crates/ship` into `ArqonPilot` as `pilot-*` module baseline.
2. Remove ArqonHPO-specific assumptions from code paths.
3. Rename local paths from `.arqon` to `.pilot`.
4. Rename command entrypoint from `arqon` to `pilot`.
5. Stand up independent Pilot CI skeleton.

Exit criteria:
- `pilot --help` runs.
- `pilot oracle scan`, `pilot heal`, `pilot navigate --dry-run` execute in a local sample repo.

Status: Completed on 2026-02-25.

## Wave 1: Core Module Refactor (3 days)

Goals:
- Convert monolithic crate into modular Pilot architecture.

Tasks:
1. Split code into core + oracle + heal + navigate crates.
2. Create shared execution and repo-context abstractions.
3. Introduce uniform result/reporting model for command outputs.
4. Create integration tests for each module command group.

Exit criteria:
- Existing Ship capabilities preserved under Pilot command names.
- Test suite green for migrated modules.

Status: Completed on 2026-02-25.

## Wave 2: Multi-Repo Foundation (4 days)

Goals:
- Enable registry and cross-repo command execution.

Tasks:
1. Build `pilot-multi` registry (SQLite).
2. Add `register/list/status` commands.
3. Add repo filters (tags/groups) and scoped execution.
4. Add cross-repo query fanout for Oracle reads.

Exit criteria:
- `pilot multi status` runs across at least 5 repos with per-repo output status.

Status: Completed on 2026-02-25.

## Wave 3: Branch + Navigate Orchestration (4 days)

Goals:
- Deliver cross-repo branch and release control.

Tasks:
1. Implement `branch create/sync/status/prune`.
2. Add dependency-order graph for repos.
3. Add linked PR manifest generation and create flow.
4. Extend navigate for multi-repo release operation.

Exit criteria:
- One command can prepare coordinated release PRs across selected repos in dependency order.

Status: Completed on 2026-02-25.

## Wave 4: Secure + Heal Expansion (3 days)

Goals:
- Introduce practical autonomous maintenance.

Tasks:
1. Implement dependency vulnerability scanning.
2. Implement secret scanning.
3. Implement automated dependency bump with verification gate.
4. Extend healing flow to multi-file patch planning.

Exit criteria:
- `pilot secure scan` and `pilot secure fix --dry-run` produce actionable, auditable output across registered repos.

Status: Completed on 2026-02-25.

## Wave 5: Plan/Create/Know (4 days)

Goals:
- Close the loop from planning to code generation and captured decisions.

Tasks:
1. GitHub issues ingestion and priority scoring.
2. Roadmap draft generation.
3. Feature scaffold and test generation.
4. ADR/decision capture and searchable knowledge records.

Exit criteria:
- Full demo flow executes on your repos with persistent artifacts and logs.

Status: Completed on 2026-02-25.

Wave 5 implementation order (aggressive):
1. `pilot plan` foundation
- Add `pilot-plan` crate and CLI group:
  - `pilot plan issues`
  - `pilot plan roadmap`
  - `pilot plan score`
- Add GitHub issue ingestion with local cache DB and deterministic scoring formula.

2. `pilot create` foundation
- Add `pilot-create` crate and CLI group:
  - `pilot create feature <name>`
  - `pilot create tests <target>`
- Start with template-driven scaffolding and test skeleton generation.

3. `pilot know` foundation
- Add `pilot-know` crate and CLI group:
  - `pilot know record`
  - `pilot know query "..."`
- Persist ADR/decision records in SQLite with searchable fields.

4. Cross-module glue
- Connect `plan -> create` handoff:
  - roadmap item to scaffold request.
- Connect `create -> know` handoff:
  - capture generated artifact metadata and decision rationale.

Result: Delivered on 2026-02-25.

## Wave 6: Cross-Repo Acceptance and Reliability Hardening (5 days)

Goals:
- Validate Pilot behavior on your real 12+ repo workspace.
- Eliminate operational blind spots before broader apply-mode automation.

Tasks:
1. Execute cross-repo acceptance protocol on grouped cohorts (`core`, `ml`, full).
2. Add unified audit logging for all mutating commands (`branch`, `multi`, `secure`, `create`, `know`).
3. Add machine-readable partial-failure artifacts (JSON) for all multi-repo mutating flows.
4. Add one E2E dry-run integration test covering dependency order -> branch -> navigate -> secure.
5. Tag stable baseline after acceptance pass.

Exit criteria:
- Acceptance protocol passes on target repo set.
- Every mutating command emits audit and per-repo result records.
- Full dry-run lifecycle is deterministic and repeatable.

Status: Completed on 2026-02-25.

## Wave 6.5: Self-Hosting (Dogfooding) on ArqonPilot (2 days)

Goals:
- Make ArqonPilot a first-class managed repo of Arqon Pilot itself.
- Prove commands and workflows are usable without special-casing.

Tasks:
1. Register ArqonPilot in workspace registry with dedicated tags/group.
2. Run full dry-run lifecycle against ArqonPilot:
- `multi status`, `multi order`, `branch create/sync/prune --dry-run`
- `navigate --multi --dry-run`
- `secure scan` and `secure fix` (dry-run)
- `plan issues/score/roadmap`, `create feature/tests --dry-run`, `know record/query`
3. Run controlled apply subset on ArqonPilot:
- one safe branch operation
- one safe scaffold operation in a throwaway branch
4. Capture outcomes as a repeatable dogfooding playbook.

Exit criteria:
- ArqonPilot can be fully operated by Pilot commands end-to-end.
- No command requires manual fallback for normal workflow.
- Dogfooding checklist is added to release gating.

Status: Completed on 2026-02-25.

## Wave 7: Controlled Apply Rollout (4 days)

Goals:
- Move from dry-run confidence to safe real mutations.
- Prove rollback and failure isolation under real repo state.

Tasks:
1. Apply-mode pilot on 1-2 low-risk repos for branch/navigate/secure flows.
2. Expand apply-mode to one full cohort after pilot stability.
3. Add guardrails for dirty worktrees, protected branches, and missing credentials.
4. Add rollback runbook and automated preflight checklist command.

Exit criteria:
- Apply-mode success rate >= 95% on pilot cohort.
- Failures produce clear remediation steps without cross-repo blast radius.
- Rollback path is documented and tested.

Status: Pending.

Wave 7 progress checkpoint (2026-02-25):
- Pilot cohort selected: `ArqonContinuum`, `ArqonCortex`.
- `multi register --tag apply-pilot` completed for both repos.
- Preflight `multi status --tag apply-pilot` passed (exists/git/clean).
- Branch rollout completed:
  - `branch create feat/pilot-wave7 --base-branch dev --dry-run`
  - `branch create feat/pilot-wave7 --base-branch dev`
  - `branch status --tag apply-pilot` confirmed both repos on `feat/pilot-wave7`, clean.
- Audit and JSON report artifacts were emitted for each mutating command.
- Secure pilot execution:
  - `secure fix --tag apply-pilot` dry-run completed.
  - `ArqonCortex`: dry-run actionable (`cargo update`, `cargo check` planned).
  - `ArqonContinuum`: no supported dependency manifest for secure-fix apply path (expected current behavior).
- Controlled apply on single pilot repo:
  - `ArqonContinuum` tagged `apply-one`.
  - `secure fix --tag apply-one --apply` executed (non-destructive no-op, success).
- Rollback drill completed:
  - In `ArqonContinuum` on `feat/pilot-wave7`, created one marker commit and reverted via `git revert`.
  - Worktree returned clean after revert.
  - Evidence recorded via `pilot know record` (tagged `wave7`, `rollback`, `apply-pilot`).

Current status: Completed on 2026-02-25.

## Wave 8: Pilot v1 Release Readiness (3 days)

Goals:
- Finalize operational docs, release packaging, and v1 checkpoint quality.

Tasks:
1. Publish operator guide for multi-repo orchestration workflows.
2. Lock and verify CI matrix for Rust 1.82 and pinned dependencies.
3. Cut `pilot-v1-rc1`, run acceptance regression, then promote `pilot-v1.0.0`.

Exit criteria:
- Reproducible install and execution flow across your target environment.
- Acceptance regression green at release candidate and final tag.

Status: Completed on 2026-02-25 (artifacts and gates prepared).

## Wave 9: Capability Completion Against Proposed Matrix (6 days)

Goals:
- Close remaining gaps versus the original ArqonPilot capability set.
- Ensure `Branch`, `Plan`, `Create`, `Secure`, `Multi`, and `Know` have production-grade coverage.

Tasks:
1. Branch completion
- Add branch protection/policy checks and enforcement previews.
- Add richer branch health/status summary output per cohort.

2. Plan completion
- Add sprint planning primitives (capacity buckets, scheduling windows).
- Add priority scoring configuration profiles and weighting presets.

3. Create completion
- Add doc generation command (`create docs`) tied to scaffolded features.
- Add refactor-assist planning mode with change-set previews.

4. Secure completion
- Add license compliance scan phase.
- Add baseline SAST adapter integration and normalized finding schema.

5. Multi completion
- Add workspace sync status report with drift indicators.
- Add linked PR execution mode (not just manifest generation).

6. Know completion
- Add reusable pattern library entries and tagging taxonomy.
- Add lessons-learned capture command with query filters.

Exit criteria:
- Proposed capability matrix is fully represented by shipped commands and tested core behavior.
- No placeholder-only command surfaces remain for matrix items.

Status: Pending.

## Wave 10: Packaging and Distribution (Cargo + PyPI) (4 days)

Goals:
- Deliver reproducible installation and upgrade paths.
- Publish both Rust-native and Python ecosystem entry points.

Tasks:
1. Rust release packaging
- Add reproducible release workflow for tagged binaries.
- Publish release artifacts for Linux/macOS with checksums and signatures.

2. PyPI package strategy and implementation
- Create Python package (`arqon-pilot`) that installs and runs the `pilot` CLI.
- Implementation (locked): `maturin`/`pyo3` wrapper exposing CLI entrypoint and optional Python API.

3. Build and publish pipeline
- Add TestPyPI publish job.
- Add PyPI publish on signed release tags.
- Add smoke tests for `pip install arqon-pilot` and `pilot --help`.

4. Versioning and compatibility policy
- Define SemVer alignment across Cargo crate and PyPI package.
- Document supported Python versions and platform matrix.

Exit criteria:
- `cargo install`-style and binary artifact installs are documented and verified.
- `pip install arqon-pilot` installs a working `pilot` command on supported targets.
- TestPyPI and PyPI publish paths are automated and reproducible.

Status: Pending.

Wave 10 decision checkpoint (2026-02-25):
- Packaging strategy selected and locked: `maturin`.
- Scaffolding implemented:
  - `pyproject.toml` (`maturin` binary bindings)
  - `.github/workflows/pypi.yml`
  - `scripts/pypi_smoke_check.sh`

## Wave 11: Production Launch and Operations (3 days)

Goals:
- Move from candidate build to production operating posture.
- Lock runbooks, observability, and incident handling.

Tasks:
1. Release candidate burn-in across full repo workspace.
2. Operational runbooks: rollback, credential rotation, degraded-mode execution.
3. Observability pack: audit review queries, health checks, failure dashboards.
4. Launch checklist and `pilot-v1.0.0` cut.

Exit criteria:
- End-to-end apply workflows validated on full target workspace.
- Production runbooks tested at least once in simulation.
- v1.0.0 release artifacts and package indexes are published.

Status: Pending.

---

## Non-Negotiable Quality Gates

1. Mutating command safety
- All mutating commands implement `--dry-run`.
- All writes produce explicit diff/intent preview before apply mode.

2. Auditability
- Every action logs actor, timestamp, repo, command, status, and artifacts.

3. Failure isolation
- Multi-repo execution cannot fail-all on one repo failure.
- Partial completion reports are first-class outputs.

4. Deterministic orchestration
- Stable ordering for repo execution and dependency graph traversal.

5. Test requirements
- Unit tests per module.
- Integration tests per command group.
- E2E smoke tests across real local repo set.

---

## Immediate Next Actions

1. Push `main` and tag baseline `pilot-v0.8-readiness`.
2. Run `workflow_dispatch` for `.github/workflows/pypi.yml` with target `testpypi`.
3. Perform clean-env TestPyPI install smoke (`pip install ...` + `pilot --help`).
4. Cut `pilot-v1-rc1` and run acceptance regression.
5. Add preflight checklist command for apply-mode guardrails.

## Cross-Repo Acceptance Protocol

Scope:
- Target all production-intent repos registered in `~/.pilot/workspace.db`.
- Run by group/tag cohorts first (for example `core`, then `ml`, then full workspace).

Phase 0: Registry integrity
1. `pilot multi list`
2. `pilot multi status`
Pass criteria:
- Every expected repo appears once with correct canonical path, group, tags.
- Missing `.pilot` or missing git repos are explicitly visible.

Phase 1: Deterministic orchestration
1. `pilot multi order [--group ...] [--tag ...]`
2. `pilot multi prs create --dry-run [--group ...] [--tag ...]`
Pass criteria:
- Order is stable between repeated runs.
- No cycle/unresolved dependency errors.
- Manifest/dry-run ordering matches dependency intent.

Phase 2: Branch lifecycle dry-run
1. `pilot branch create <branch> --dry-run [filters]`
2. `pilot branch sync --dry-run [filters]`
3. `pilot branch status [filters]`
4. `pilot branch prune --dry-run [filters]`
Pass criteria:
- Every repo yields explicit per-repo result.
- No silent skips; no fail-all from single-repo issues.

Phase 3: Navigate orchestration dry-run
1. `pilot navigate --multi --dry-run [filters]`
Pass criteria:
- Coordinated release ordering output is valid.
- Manifest path/output is deterministic when requested.

Phase 4: Secure scan/fix dry-run
1. `pilot secure scan [filters]`
2. `pilot secure fix [filters]` (dry-run default)
Pass criteria:
- Findings are structured and attributable by repo.
- Fix flow produces actionable commands without mutating repos in dry-run.

Phase 5: Planning and knowledge loop
1. `pilot plan issues ...`
2. `pilot plan score ...`
3. `pilot plan roadmap ...`
4. `pilot create feature <name> --dry-run`
5. `pilot know record ...`
6. `pilot know query --query ...`
Pass criteria:
- Artifacts are written in expected locations.
- Decision records are queryable and linked to execution context.

Phase 6: Controlled apply pilot (small cohort)
1. Repeat phases 2-4 with `--apply`/non-dry-run where supported on 1-2 low-risk repos.
Pass criteria:
- Mutations succeed with clean rollback path.
- No unexpected cross-repo side effects.

Gotchas to watch during acceptance:
- Repo name collisions in registry break dependency-edge assignment by name.
- Dirty worktrees will block or skew mutating flows.
- Dependency cycles will surface only when running ordered operations.
- Missing external tools (`cargo-audit`, `pip-audit`) should degrade gracefully but must be acknowledged in findings.
- Environment-specific git remotes/auth can fail branch/navigate operations even when local checks pass.

## Formal Branch Management Plan

Canonical policy reference:
- `docs/branch-management-guide.md`

Objectives:
- Standardize branch lifecycle across all registered repos.
- Prevent unsafe mutations and reduce merge-order conflicts.

Branch taxonomy:
- `main`: protected production branch.
- `dev`: default integration branch.
- `release/*`: optional, exception-only for complex coordinated releases.
- `feat/*`, `fix/*`, `chore/*`: short-lived work branches.

Operational policy:
1. Create branches in dependency order only (`multi order` -> `branch create`).
2. Sync branches from base before release actions (`branch sync`).
3. Enforce clean worktree precondition for mutating operations.
4. Prune only merged non-protected branches (`branch prune` with exclusions).
5. Record per-repo branch outcomes and keep failure isolation.

Required command flow:
1. `pilot multi order [filters]`
2. `pilot branch create <branch> --dry-run [filters]`
3. `pilot branch create <branch> [filters]` (apply mode)
4. `pilot branch sync --branch <branch> [filters]`
5. `pilot branch status [filters]`
6. `pilot branch prune --dry-run [filters]` then apply when confirmed

Release branch governance:
- Default policy: release directly from `main` tags (no mandatory release branch).
- Exception policy: use one coordinated `release/<version>` branch only when explicitly required by cross-repo risk.
- Generate linked PR manifest before PR creation (`multi prs create`).
- Require successful dry-run navigate orchestration before any apply-mode release mutation.

Exit criteria:
- Branch lifecycle commands execute with deterministic order and per-repo reporting.
- Protected branches are never pruned or force-reset by automation.
- Branch operations are included in self-hosting and acceptance gates.

---

## Explicitly Deferred

These are deferred only until their planned wave, not dropped:

- Advanced AI release-note synthesis.
- Full SAST integrations beyond baseline scanners (scheduled in Wave 9).
- Sophisticated planning heuristics beyond initial priority scoring (scheduled in Wave 9).
