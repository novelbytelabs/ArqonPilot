# Alpha Release Playbook

This is the canonical end-to-end release process for Arqon Pilot alpha builds.
It covers packaging, publish order, docs publish, and audit evidence.

Current policy:

1. Core lane Rust/Cargo: `1.82.0` (frozen)
2. Packaging lane Rust: `1.88.0` (frozen)
3. Protobuf/protoc: `4.25.8` / `25.8` (frozen)

Versioning policy:

1. Git tag: `vX.Y.Z-alpha.N` (example: `v0.2.0-alpha.1`)
2. PyPI version: `X.Y.ZaN` (example: `0.2.0a1`)
3. Never publish alpha from unverified local state.

## 0) Full Release Order (Do Not Reorder)

Run these phases in this exact order:

1. Preflight and gate checks
2. Version bump
3. Release docs update
4. Commit
5. Push branch safely
6. Create and push annotated tag
7. PyPI publish verification
8. Clean environment install smoke test
9. Docs site publish verification
10. GitHub release entry
11. Evidence bundle + release log finalization

---

## 1) Preflight and Gate Checks

1. Ensure branch is clean and synchronized:

```bash
git status -sb
git fetch origin
git pull --ff-only origin main
```

2. Verify frozen policy:

```bash
./scripts/verify_toolchain_policy.sh
```

3. Run mandatory gates:

```bash
./scripts/prepush_gate.sh
./scripts/release_readiness_check.sh
./scripts/migration_smoke_test.sh
```

4. Run Clean Operator Proof (G-044 boundary + no tribal deps):

```bash
env -i HOME="$HOME" PATH="/usr/bin:/bin:/usr/local/bin" bash -lc './scripts/release_readiness_check.sh'
```

5. Run matrix closure checks + UI smoke:

```bash
./scripts/wave_acceptance_matrix.sh --wave I --profile full
./scripts/wave_acceptance_matrix.sh --wave J --profile full
./scripts/ui_smoke_check.sh
```

## 2) Version Bump

Update both versions together:

1. `pyproject.toml` version (`0.2.0a1`)
2. workspace `Cargo.toml` version (`0.2.0-alpha.1`)

Then verify:

```bash
rg -n '^version = ' pyproject.toml Cargo.toml
```

## 3) Release Docs Update (Before Commit)

Update these docs before cutting tag:

1. `docs/releases/<version>.md` (fill highlights/evidence placeholders)
2. `docs/release-log.md` (add release entry block)
3. Optional: summary lines in roadmap/plan docs if this release closes a wave

If docs are published via MkDocs, verify nav contains this release page.

## 4) Commit

```bash
git add pyproject.toml Cargo.toml docs/ mkdocs.yml
git commit -m "chore(release): cut v0.2.0-alpha.1"
```

## 5) Push Branch Safely

```bash
./scripts/push_main.sh main
```

## 6) Create and Push Tag

```bash
git tag -a v0.2.0-alpha.1 -m "Arqon Pilot alpha release v0.2.0-alpha.1"
git push origin v0.2.0-alpha.1
```

Tag push triggers `.github/workflows/pypi.yml`.
Alternative manual path:

```bash
gh workflow run pypi.yml -f target=pypi
```

## 7) Post-Publish Verification (PyPI)

1. Verify index visibility:

```bash
./scripts/verify_pypi_release.sh --index pypi --version 0.2.0a1
```

2. Record PyPI workflow run ID and final URL:

```bash
gh run list --workflow pypi.yml --limit 5
```

## 8) Clean Environment Install Smoke

```bash
python -m venv /tmp/arqon-pilot-alpha-smoke
source /tmp/arqon-pilot-alpha-smoke/bin/activate
pip install -U pip
pip install arqon-pilot==0.2.0a1
pilot --help
deactivate
rm -rf /tmp/arqon-pilot-alpha-smoke
```

## 9) Docs Publish Verification

1. Ensure docs workflow succeeds:

```bash
gh run list --workflow docs.yml --limit 5
```

2. Confirm docs site includes release pages:
   - `release-playbook.md`
   - `release-log.md`
   - `releases/<version>.md`

## 10) GitHub Release Entry

Create GitHub release for tag with short notes and links:

1. Release tag/version
2. PyPI project URL for the version
3. Docs release page URL
4. Key evidence artifact paths

## 11) Evidence Bundle + Final Log

```bash
./scripts/release_collect_evidence.sh --label 0.2.0a1
```

Then finalize:

1. `docs/release-log.md` entry (replace all `TBD`)
2. `docs/releases/<version>.md` evidence section (replace all `TBD`)

## Required Evidence (Mandatory)

Every alpha release must include:

1. `prepush_gate` log path
2. `release_readiness_check` output or log reference
3. Wave I/J matrix artifact paths
4. `ui_smoke_check` log path
5. PyPI visibility verification output
6. clean-venv smoke install output (`pilot --help`)
7. Git tag + commit SHA
8. workflow run IDs (`ci`, `pypi`)

Record these in both:

1. `docs/release-log.md` (human-readable release journal)
2. `docs/releases/<version>.md` (version-specific release notes)

## Rollback/Hotfix

If publish is bad:

1. Do not delete published artifacts.
2. Cut a new alpha patch (`0.2.0a2`), document incident, publish fix.
3. Record root cause and preventive control in `docs/gotcha-registry.md`.

## Rollback Drill (Mandatory Evidence)

Before every major alpha milestone, execute a simulation rollback drill:

1. Snapshot Managed DB:
   `pg_dump -h /tmp/.arqon-pilot -p 9132 pilot_local > /tmp/pilot_pre_rollback.sql`
2. Binary Revert Test:
   `git checkout HEAD~1 && cargo build -p pilot --locked`
3. Restoration:
   `git checkout - && cargo build -p pilot --locked`
4. Verify Restoration:
   `pilot agorg list`
