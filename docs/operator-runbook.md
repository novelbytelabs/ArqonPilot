# ArqonPilot Operator Runbook

This runbook defines the standard operating procedure for running Arqon Pilot across single-repo and multi-repo workflows.

## 1. Environment Baseline

- Rust toolchain pinned at `1.82.0` (`rust-toolchain.toml`).
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
- `pilot branch create feat/<name> --group <group> --base-branch dev --dry-run`
- `pilot branch sync --branch feat/<name> --group <group> --dry-run`

3. Release planning
- `pilot multi prs create --group <group> --dry-run`
- `pilot navigate --multi --group <group> --dry-run`

4. Security and maintenance
- `pilot secure scan --group <group>`
- `pilot secure fix --group <group>` (dry-run default)

5. Planning and knowledge loop
- `pilot plan issues --input <issues.json>`
- `pilot plan score`
- `pilot plan roadmap`
- `pilot create feature <name> --dry-run`
- `pilot know record --title ... --context ... --decision ...`

## 3. Controlled Apply Workflow

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

## 4. Rollback Procedure

Preferred rollback:
- `git revert <bad_commit_sha>`

After rollback:
1. verify clean status
2. run tests
3. record incident and resolution via `pilot know record`

Avoid:
- history rewriting on shared branches (`reset --hard`, force-push) except emergency local recovery with explicit approval.

## 5. Incident Triage

When a multi-repo operation fails:
1. open latest report artifact from `~/.pilot/reports/`
2. isolate failing repo(s)
3. re-run command scoped to failed repo only
4. record root cause and corrective action in `pilot know`

## 6. Release Candidate Gate (v1-rc1)

Before cutting `pilot-v1-rc1`:
1. `cargo check -p pilot --locked`
2. full CLI suite green
3. wave acceptance docs updated
4. dogfooding + controlled apply evidence documented
5. audit/report artifacts present and interpretable
