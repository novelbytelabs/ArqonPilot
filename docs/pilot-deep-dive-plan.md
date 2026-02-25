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

1. Create `ArqonPilot` Rust workspace and initial crate skeleton.
2. Import current Ship code as baseline module set.
3. Execute Wave 0 renames (`arqon` -> `pilot`, `.arqon` -> `.pilot`, `ship` command -> `navigate`).
4. Remove HPO-specific version fallback logic.
5. Add first-pass integration tests for `pilot oracle`, `pilot heal`, `pilot navigate`.

---

## Explicitly Deferred

These are deferred only until their planned wave, not dropped:

- Advanced AI release-note synthesis.
- Full SAST integrations beyond baseline scanners.
- Sophisticated planning heuristics beyond initial priority scoring.
