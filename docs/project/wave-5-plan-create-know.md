# Wave 5 Plan + Create + Know

Date: 2026-02-25

## Objective

Complete the planning-to-scaffolding-to-knowledge loop for autonomous repo operations.

## Completed

1. New module crates
- `pilot-plan`: issue ingestion, scoring, roadmap generation.
- `pilot-create`: feature/test scaffold generation with dry-run preview.
- `pilot-know`: ADR/decision recording and query with SQLite persistence.

2. CLI surface delivered
- `pilot plan issues [--input ...|--github-repo owner/repo] [--output ...]`
- `pilot plan score [--input ...] [--output ...]`
- `pilot plan roadmap [--input ...] [--output ...] [--top-n N]`
- `pilot create feature <name> [--output-dir ...] [--dry-run]`
- `pilot create tests <target> [--output-dir ...] [--dry-run]`
- `pilot know record --title ... --context ... --decision ... [--status ...] [--tag ...]`
- `pilot know query --query ... [--limit N]`

3. Persistence and artifacts
- Plan cache defaults:
  - `~/.pilot/plan/issues.json`
  - `~/.pilot/plan/scored.json`
  - `~/.pilot/plan/roadmap.md`
- Know DB:
  - `~/.pilot/know.db`

4. Tests
- Added CLI tests:
  - `tests/plan_cli_test.rs`
  - `tests/create_cli_test.rs`
  - `tests/know_cli_test.rs`
- Added unit tests:
  - scoring behavior in `pilot-plan`

## Gotchas Addressed

- GitHub issue ingestion requires `owner/repo` format and may require `GITHUB_TOKEN`.
- `create` commands are non-destructive for existing files and support `--dry-run`.
- `know` data path is global (`~/.pilot/know.db`); tests isolate via `HOME` override.

## Exit Criteria Mapping

- Planning, creation, and knowledge command groups are operational with persistent artifacts.
- End-to-end flow now supports: ingest issues -> score/prioritize -> generate roadmap -> scaffold -> record decisions.
