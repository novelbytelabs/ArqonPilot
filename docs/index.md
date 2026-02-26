# ArqonPilot

ArqonPilot is a production CLI for single-repo and cross-repo engineering operations:
Oracle indexing/query, healing, branch orchestration, release navigation, security workflows,
planning, scaffolding, and knowledge capture.

ArqonPilot is designed for high-control operation:
- dry-run first for mutating commands
- explicit repo targeting (group/tag filters)
- auditable output for cross-repo actions

## Get Running in 5 Minutes

## 1) Install

```bash
pip install arqon-pilot
pilot --help
```

## 2) Initialize and register repos

```bash
pilot init
pilot multi register --path /path/to/ArqonContinuum --group core --tag apply-pilot
pilot multi register --path /path/to/ArqonCortex --group core --tag apply-pilot
pilot multi status --tag apply-pilot
```

## 3) Run safe first operations

```bash
pilot branch create feat/pilot-rollout --group core --dry-run
pilot navigate --multi --group core --dry-run
pilot secure scan --group core
pilot secure fix --group core --dry-run
```

## Core Workflows

## Cross-repo branch orchestration

```bash
pilot multi order --group core
pilot branch create feat/my-change --group core --dry-run
pilot branch create feat/my-change --group core
pilot branch status --group core
```

## Coordinated release dry-run

```bash
pilot navigate --multi --group core --dry-run
```

## Planning -> scaffold -> knowledge capture

```bash
pilot plan issues --input /tmp/issues.json
pilot plan score --input /tmp/issues.json --output /tmp/scored.json
pilot plan roadmap --input /tmp/scored.json --output /tmp/roadmap.md --top-n 10
pilot create feature checkout --dry-run
pilot know record --title "Wave decision" --context "Why" --decision "What" --tag wave
```

## Safety Model

- Run mutating commands with `--dry-run` before apply mode.
- Use group/tag filters to reduce blast radius.
- Keep worktrees clean before branch/navigate operations.
- Review per-repo outcomes for partial failures before retrying.

## Documentation Map

- [Developer Guide](developer-guide.md)
- [Testing Strategy](testing-strategy.md)
- [Operator Runbook](operator-runbook.md)
- [Branch Management Guide](branch-management-guide.md)

## Packaging and Release Notes

- Package: `arqon-pilot` on PyPI
- Python support: `>=3.10`
- Linux wheel compatibility is tuned for broader Ubuntu support
- CI workflows:
  - `.github/workflows/ci.yml`
  - `.github/workflows/pypi.yml`
  - `.github/workflows/docs.yml`

## Common Gotchas

1. Dirty worktrees can block mutating cross-repo commands.
2. Missing external tools (for specific scans/fixes) can reduce functionality.
3. Dependency cycles in repo graph will fail ordered operations.
4. If packaging fails, confirm `Cargo.lock` is committed and up to date.
