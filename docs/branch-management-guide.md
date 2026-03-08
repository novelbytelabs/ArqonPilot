# Arqon Ecosystem Branch Management Guide

This document establishes the branch management standards and workflows for all repositories in the Arqon Ecosystem.

## Branch Structure

### Permanent branches

| Branch | Purpose | Protection | Deploy status |
|--------|---------|------------|---------------|
| `main` | Production-ready code | Protected, requires PR | Always deployable |
| `dev` | Integration branch | Protected, requires PR | Development builds |

### Branch lifecycle

`feature/* -> dev -> main`

### Branch relationships

- `main` is the source of truth for production.
- `dev` is created from `main` and stays synchronized.
- Feature branches are created from `dev` and merged back to `dev`.
- `dev -> main` merges happen via PR when ready for release.

## Branch Naming Conventions

### Prefixes

| Prefix | Purpose | Example |
|--------|---------|---------|
| `feat/` | New feature | `feat/click-tones` |
| `fix/` | Bug fix | `fix/memory-leak` |
| `docs/` | Documentation only | `docs/api-reference` |
| `test/` | Test additions/changes | `test/integration-coverage` |
| `refactor/` | Code refactoring | `refactor/episode-parser` |
| `chore/` | Maintenance tasks | `chore/update-dependencies` |
| `perf/` | Performance improvements | `perf/query-optimization` |

### Naming rules

1. Use kebab-case after the prefix (`feat/my-feature`).
2. Be descriptive and concise (3-5 words after the prefix).
3. No issue numbers in branch names.
4. For cross-repo features, use the same branch name across all affected repos.

> [!NOTE]
> **Enforcement**: Arqon Pilot now actively enforces these rules via its **Governance Engine**. Trying to create or sync an out-of-policy branch from the Pilot UI or CLI will be explicitly blocked unless a signed Exception exists. Please refer to the [Governance & Policy Guide](governance-guide.md) for exception requests. 

### Branch TTL and cleanup SLA

- `feat/*`, `fix/*`, `refactor/*`, `test/*`, `perf/*`, `chore/*` branches should be merged, rebased, or closed within 14 days.
- Branches older than 14 days must be either:
  - refreshed against `dev` and actively used, or
  - closed and recreated later.

## Workflow Patterns

### Standard feature flow

1. `git checkout dev && git pull origin dev`
2. `git checkout -b feat/my-feature`
3. Develop and commit.
4. `git push -u origin feat/my-feature`
5. Create PR `feat/my-feature -> dev`
6. Merge PR.
7. Delete branch locally/remotely.

PR requirements:
- Include a `Related PRs` section for cross-repo work.
- Include dependency order and rollback note in PR description.

### Hotfix flow

1. `git checkout main && git pull origin main`
2. `git checkout -b fix/critical-bug`
3. Fix and PR to `main`.
4. Merge `main` back into `dev`.

### Release flow

1. Ensure `dev` is release-ready.
2. PR `dev -> main`.
3. Merge PR and tag from `main`.

Release tagging standard:
- Use annotated SemVer tags: `vX.Y.Z`
- Example: `git tag -a v1.2.0 -m "Release v1.2.0" && git push origin v1.2.0`

## Multi-Repository Coordination

### Required steps

1. Plan affected repos and dependency order.
2. Create consistent branch names across repos.
3. Implement changes in dependency order.
4. Create linked PRs with related references.
5. Merge in dependency order.
6. Update downstream dependencies after upstream merges.

Hard merge-order gate:
- Do not merge downstream repo PRs before upstream dependency PRs are merged.
- If order is violated, downstream PR must be revalidated after dependency update.

### Pilot command mapping

Use Arqon Pilot as the orchestration layer:

1. Register and inspect workspace
- `pilot multi register --path ... --group ... --tag ...`
- `pilot multi list`
- `pilot multi status`

2. Model dependency order
- `pilot multi deps set --repo <repo> --depends-on <upstream>`
- `pilot multi order`

3. Coordinate branches
- `pilot branch create <branch> --dry-run`
- `pilot branch sync --branch <branch> --dry-run`
- `pilot branch status`
- `pilot branch prune --dry-run`

4. Prepare linked release plan
- `pilot multi prs create --dry-run`
- `pilot navigate --multi --dry-run`

5. Enforce merge order and tracking
- Add linked PR references in each PR body.
- Merge PRs in `pilot multi order` sequence.

## Decision Points (Settled)

### Decision 1: Branch protection rules

- `main`: require PR, require review, require CI pass.
- `dev`: require PR, optional review, require CI pass.

Status: Confirmed.

### Decision 2: Direct commits to `dev`

- Policy: no direct commits; all changes via PR for traceability.

Status: Confirmed.

### Decision 3: Release branches

- Policy: no dedicated `release/*` branches by default; release from `main` tags.

Status: Confirmed.

### Decision 4: Documentation branch strategy

- Policy: hybrid.
- Minor docs can ship with feature branches; major docs can use dedicated `docs/*`.

Status: Confirmed.

### Decision 5: Multi-repo sync strategy

- Policy: manual synchronization now; automate once CI maturity is sufficient.

Status: Confirmed.

### Decision 6: Protected branch exceptions

- Policy: emergency bypass of branch protection is allowed only for production incidents.
- Requirements:
  - incident reference in commit/PR,
  - post-incident PR documenting final state,
  - retro entry in `pilot know record`.

Status: Confirmed.

## Rollback Policy

- Preferred rollback method: `git revert` (preserves audit history).
- Avoid history-rewriting rollback on shared branches.
- Before rollback:
  1. capture current SHA and failing SHA,
  2. record reason and impact,
  3. execute rollback in dependency order for cross-repo incidents.

Emergency-only:
- `git reset --hard` is local-recovery only and must not be used on shared/protected branches.

## Gotchas

1. Branch name mismatch across repos breaks linked-PR traceability.
2. Repo name collisions in multi registry can map dependency edges incorrectly.
3. Dirty local worktrees can produce false branch status or block safe apply flows.
4. Merging out of dependency order creates hidden downstream breakage.
5. Stale branches (>14 days) often miss upstream API or schema changes.
6. Direct commits to `dev` bypass review/CI intent and create drift.
7. Using reset-based rollback on shared branches destroys collaborative audit trail.
8. Unlinked cross-repo PRs make incident triage and release signoff much slower.

## Compliance Checks

- Weekly branch hygiene report:
  - stale branches >14 days,
  - direct commits to protected branches,
  - PRs missing linked references for cross-repo work,
  - out-of-order merges against dependency graph.

## Quick Reference

### Daily

- `git checkout dev && git pull origin dev`
- `git checkout -b feat/my-feature`
- `git status`
- `git branch -vv`

### Cleanup

- `git branch -d feat/my-feature`
- `git push origin --delete feat/my-feature`
- `git fetch --prune`

### Recovery

- `git reflog`
- `git checkout -b restored-branch <commit>`

## Document History

| Date | Author | Changes |
|------|--------|---------|
| 2026-02-24 | Arqon Team | Initial draft |
| 2026-02-25 | ArqonPilot | Integrated as canonical ecosystem branch guide |
