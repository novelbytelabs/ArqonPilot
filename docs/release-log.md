# Release Log

This log is the release audit trail. Every release (including alpha) must have a complete evidence set.

## Release Entry Template

Use this structure for every new release:

```md
## vX.Y.Z-alpha.N (YYYY-MM-DD)

- Git tag:
- Commit SHA:
- PyPI version:
- Release type: alpha

### Verification
- prepush gate:
- release readiness:
- Wave I matrix artifact:
- Wave J matrix artifact:
- UI smoke log:
- PyPI visibility check:
- clean venv install + `pilot --help`:

### CI/CD
- CI run ID:
- PyPI run ID:
- Docs run ID:
- GitHub release URL:

### Notes
- Key changes:
- Known limitations:
- Follow-up actions:
```

---

## v0.2.0-alpha.1 (2026-03-03)

- Git tag: `v0.2.0-alpha.1` (dry-run)
- Commit SHA: `76f9e9e150be3d8f9fd892b8892ef815013e2f4b`
- PyPI version: `0.1.6a1` (simulated)
- Release type: alpha

### Verification

- prepush gate: `prepush_gate_latest.log` (sha256: 6cb236f6...)
- release readiness: `scripts/release_readiness_check.sh` PASSED
- Wave I matrix artifact: `acceptance_matrix_wave_i_full_latest.json` (sha256: 243ebc11...)
- Wave J matrix artifact: `N/A` (Not run in dry-run)
- UI smoke log: `N/A` (Available in /tmp/pilot-reports/ but not captured in manifest)
- PyPI visibility check: `Simulated`
- clean venv install + `pilot --help`: `Passed (Manual)`
- Integrity manifest: `manifest.json` ✅ Verified via `verify_bundle.sh`

### CI/CD

- CI run ID: `dry-run`
- PyPI run ID: `dry-run`

### Notes

- Key changes: Phase P9 Release Train Hardening implemented. Added `channel-policy.md`, `migration-playbook.md`, `compatibility-matrix.md`, `slo-policy.md`, and `incident-runbook.md`.
- Known limitations: ArqonBus compatibility shim may be used in some local environments. `protoc` 25.8 missing in local env (G-014 violation).
- Follow-up actions: Finalize Wave L tech debt burn-down.
