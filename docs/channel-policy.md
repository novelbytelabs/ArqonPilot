# Arqon Pilot Channel Policy

This document defines the entry/exit criteria and gate requirements for Arqon Pilot release channels.

## 1. Alpha Channel

**Tag Format**: `vX.Y.Z-alpha.N`  
**PyPI Format**: `X.Y.ZaN`

### Entry Criteria
- All standard `cargo test` and `cargo check` pass.
- `ui_smoke_check.sh` passes on latest Chrome/Firefox.
- Unified waves `P1..P8` are marked `CLOSED` in `PRODUCTIONIZE.md`.

### Exit Criteria (to Beta)
- At least 2 independent dogfood cycles completed with no P0 (critical) bugs.
- No P1 (major) bugs are open.
- Multi-repo reconciliation logic verified across at least 5 repositories.

### Mandated Gates
- `verify_toolchain_policy.sh`
- `prepush_gate.sh`
- `ui_smoke_check.sh`
- `wave_acceptance_matrix.sh` (Waves I, J)
- `check_duplicate_consts.py` (G-015 prevention)

---

## 2. Beta Channel

**Tag Format**: `vX.Y.Z-beta.N`  
**PyPI Format**: `X.Y.ZbN`

### Entry Criteria
- Alpha exit criteria met.
- Compatibility matrix verified for all target platforms (Ubuntu 22.04, 24.04).
- Artifact integrity manifest (`verify_bundle.sh`) passes for the release candidate.

### Exit Criteria (to Stable)
- 0 known P0/P1 bugs.
- SLO baseline measured and within error budget for 7 consecutive days of usage.
- Performance regressions verified below 5% threshold for common operations (scan/reconcile).

### Mandated Gates
- All Alpha gates.
- `compat_matrix_smoke.sh`
- `slo_baseline_check.sh` (if applicable)

---

## 3. Stable Channel

**Tag Format**: `vX.Y.Z`  
**PyPI Format**: `X.Y.Z`

### Entry Criteria
- Beta exit criteria met.
- Migration playbook dry-run completed on production-scale database.
- Incident runbook verified for all P0/P1 scenarios in `gotcha-registry.md`.

### Exit Criteria
- One clean release cycle completed with no hotfixes (patch releases) required within 48 hours of publish.

### Mandated Gates
- All Beta gates.
- `migration_smoke_test.sh`
- `verify_bundle.sh` (Integrity check)

---

## 4. Severity Ladder Definition

| Severity | Definition |
|----------|------------|
| **P0** | Data loss, scope enforcement bypass, silent failure in production path, or total UI death (G-015). |
| **P1** | Core operation broken for all users with no accessible workaround. |
| **P2** | Core operation degraded; workaround is documented and accessible. |
| **P3** | Minor UX issue, cosmetic bug, or non-blocking request for enhancement. |
