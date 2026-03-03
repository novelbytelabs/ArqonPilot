# Arqon Pilot Migration & Rollback Playbook

## 1. Database Schema Migrations

Arqon Pilot manages its own Postgres schema under `~/.arqon/pilot/db/`.
The system uses an idempotent initialization flow: `initialize()` → `run_migration()`.

### Safe Migrations (Additions)
- **Adding a new table**: Add a `CREATE TABLE IF NOT EXISTS` block in the relevant store module (e.g., `agorg.rs`, `governance/store.rs`).
- **Adding a new column**: Use `ALTER TABLE ... ADD COLUMN IF NOT EXISTS ...`.
- **Validation**: Test by starting a clean instance and verifying the schema via `psql` or `pilot db status`.

### Breaking Migrations (Modifications/Deletions)
- **Never perform destructive changes silently.**
- **Procedure**:
  1. Add the new field/table first.
  2. Implement dual-write or lazy-migration logic in the application.
  3. Deprecate the old field in the next release.
  4. Remove the old field only after a full stable release cycle has passed.

---

## 2. Artifact Format Versioning

All JSON artifacts persisted to `~/.pilot/reports/` must follow versioning rules:

- **Schema Version**: Include a `schema_version` integer in the top-level JSON.
- **Forward Compatibility**: New binaries must be able to parse older schema versions (graceful degradation).
- **Backward Compatibility**: If a new version is unreadable by old binaries, it must be documented as a "Breaking Artifact Change" in the release notes.

---

## 3. Rollback Procedures

### Binary Rollback
If a regression is found after a release:
1. **Identify**: Find the last known-good tag (e.g., `v0.1.9-alpha.4`).
2. **Revert**: `git checkout v0.1.9-alpha.4`.
3. **Build**: `cargo build -p pilot --locked`.
4. **Deploy**: Reinstall via `pip install arqon-pilot==0.1.9a4`.
5. **Audit**: Record the incident and recovery in `docs/gotcha-registry.md`.

### Database State Rollback
If a schema migration corrupts state or prevents startup:
1. **Stop**: `./scripts/pilot_local.sh db stop`.
2. **Snapshot**: Create a safety backup:
   ```bash
   pg_dump -h /tmp/.arqon-pilot -p 9132 pilot > /tmp/pilot_pre_rollback.sql
   ```
3. **Revert Binary**: Follow the Binary Rollback procedure above.
4. **Reinitialize (Extreme Case)**: If the schema is strictly incompatible and data loss is tolerable (typical in Alpha/Beta):
   ```bash
   rm -rf ~/.arqon/pilot/db
   ./scripts/pilot_local.sh db start
   ```
   *Warning: This deletes all AGOrg registry and governance history.*

---

## 4. Migration Smoke Test

Before tagging a release with schema changes, run:
```bash
./scripts/migration_smoke_test.sh
```
This script validates:
1. Clean startup (no DB).
2. Upgrade startup (existing DB).
3. Data accessibility post-migration.
