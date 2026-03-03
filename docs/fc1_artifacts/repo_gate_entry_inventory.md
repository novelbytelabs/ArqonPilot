# Repo-by-Repo Gate Entry Inventory

**Version**: 1.0
**Generated**: 2026-03-03T14:32:00Z
**Scope**: ArqonPilot, ArqonBus, ArqonLattice, ArqonStudio, ArqonHPO

## Overview

This inventory documents the current gate entry points for each scope repository in the Arqon federation. It maps scripts, CI jobs, and governance hooks to their respective gates.

---

## 1. ArqonPilot

**Path**: `/home/irbsurfer/Projects/arqon/ArqonPilot`
**Type**: Primary control plane

### Gate Scripts

| Gate | Script | Status | Coverage |
|------|--------|--------|----------|
| Toolchain Policy | `scripts/verify_toolchain_policy.sh` | ✅ ACTIVE | Full |
| Pre-push Gate | `scripts/prepush_gate.sh` | ✅ ACTIVE | Full |
| Push Safe | `scripts/push_main.sh` | ✅ ACTIVE | Full |
| Lock Repair | `scripts/repair_lock_182.sh` | ✅ ACTIVE | Full |
| CI Parity | `scripts/ci_parity_check.sh` | ✅ ACTIVE | Full |
| Packaging Lane | `scripts/packaging_lane_check.sh` | ✅ ACTIVE | Full |
| UI Smoke | `scripts/ui_smoke_check.sh` | ✅ ACTIVE | Full |
| Wave Acceptance | `scripts/wave_acceptance_matrix.sh` | ✅ ACTIVE | Full |
| Release Readiness | `scripts/release_readiness_check.sh` | ✅ ACTIVE | Full |
| Release Evidence | `scripts/release_collect_evidence.sh` | ✅ ACTIVE | Full |
| PyPI Verify | `scripts/verify_pypi_release.sh` | ✅ ACTIVE | Full |

### CI Workflows

| Workflow | Jobs | Gate Delegation |
|----------|------|-----------------|
| `.github/workflows/ci.yml` | core, packaging, ui-smoke | Delegates to prepush_gate.sh, packaging_lane_check.sh |
| `.github/workflows/pypi.yml` | build, publish | Delegates to verify_pypi_release.sh |

### Governance Integration

| Component | Integration |
|-----------|-------------|
| `governance/store.rs` | Policy persistence |
| `governance/eval.rs` | Policy evaluation |
| `preflight/graph.rs` | Deterministic preflight |
| `serve_ui.rs` | UI/API gates |

### Toolchain

| Lane | Version | Status |
|------|---------|--------|
| Core Rust | 1.82.0 | ✅ FROZEN |
| Packaging Rust | 1.88.0 | ✅ FROZEN |
| Protobuf | 4.25.8 | ✅ FROZEN |

---

## 2. ArqonBus

**Path**: `/home/irbsurfer/Projects/arqon/ArqonBus`
**Type**: Message bus infrastructure

### Gate Scripts

| Gate | Script | Status | Notes |
|------|--------|--------|-------|
| Toolchain Policy | (inherited) | ⚠️ NOT PRESENT | Should use ArqonPilot scripts |
| Pre-push Gate | (inherited) | ⚠️ NOT PRESENT | Should use ArqonPilot scripts |
| Push Safe | (inherited) | ⚠️ NOT PRESENT | Should use ArqonPilot scripts |
| Lock Repair | (inherited) | ⚠️ NOT PRESENT | Should use ArqonPilot scripts |

### CI Workflows

| Workflow | Jobs | Gate Delegation |
|----------|------|-----------------|
| `.github/workflows/ci.yml` | (not found) | ⚠️ MISSING |

### Gap Analysis

| Gap | Severity | Description |
|-----|----------|-------------|
| Missing CI workflow | HIGH | No CI defined |
| No toolchain policy | HIGH | No frozen version enforcement |
| No gate scripts | HIGH | No local gates |

### Recommendations

1. Create `.github/workflows/ci.yml` with toolchain policy checks
2. Add gate scripts or delegate to ArqonPilot
3. Freeze Rust version in `rust-toolchain.toml`

---

## 3. ArqonLattice

**Path**: `/home/irbsurfer/Projects/arqon/ArqonLattice`
**Type**: Lattice/Graph infrastructure

### Gate Scripts

| Gate | Script | Status | Notes |
|------|--------|--------|-------|
| Toolchain Policy | (inherited) | ⚠️ NOT PRESENT | Should use ArqonPilot scripts |
| Pre-push Gate | (inherited) | ⚠️ NOT PRESENT | Should use ArqonPilot scripts |
| Push Safe | (inherited) | ⚠️ NOT PRESENT | Should use ArqonPilot scripts |

### CI Workflows

| Workflow | Jobs | Gate Delegation |
|----------|------|-----------------|
| `.github/workflows/ci.yml` | (not found) | ⚠️ MISSING |

### Gap Analysis

| Gap | Severity | Description |
|-----|----------|-------------|
| Missing CI workflow | HIGH | No CI defined |
| No toolchain policy | HIGH | No frozen version enforcement |

### Recommendations

1. Create `.github/workflows/ci.yml` with toolchain policy checks
2. Delegate gates to ArqonPilot scripts

---

## 4. ArqonStudio

**Path**: `/home/irbsurfer/Projects/arqon/ArqonStudio`
**Type**: Studio/IDE integration

### Gate Scripts

| Gate | Script | Status | Notes |
|------|--------|--------|-------|
| Toolchain Policy | (inherited) | ⚠️ NOT PRESENT | Should use ArqonPilot scripts |
| Pre-push Gate | (inherited) | ⚠️ NOT PRESENT | Should use ArqonPilot scripts |

### CI Workflows

| Workflow | Jobs | Gate Delegation |
|----------|------|-----------------|
| `.github/workflows/ci.yml` | (not found) | ⚠️ MISSING |

### Gap Analysis

| Gap | Severity | Description |
|-----|----------|-------------|
| Missing CI workflow | HIGH | No CI defined |
| No toolchain policy | HIGH | No frozen version enforcement |

### Recommendations

1. Create `.github/workflows/ci.yml` with toolchain policy checks
2. Delegate gates to ArqonPilot scripts

---

## 5. ArqonHPO

**Path**: `/home/irbsurfer/Projects/arqon/ArqonHPO`
**Type**: Hyperparameter optimization

### Gate Scripts

| Gate | Script | Status | Notes |
|------|--------|--------|-------|
| Toolchain Policy | (inherited) | ⚠️ NOT PRESENT | Should use ArqonPilot scripts |
| Pre-push Gate | (inherited) | ⚠️ NOT PRESENT | Should use ArqonPilot scripts |

### CI Workflows

| Workflow | Jobs | Gate Delegation |
|----------|------|-----------------|
| `.github/workflows/ci.yml` | (not found) | ⚠️ MISSING |

### Gap Analysis

| Gap | Severity | Description |
|-----|----------|-------------|
| Missing CI workflow | HIGH | No CI defined |
| No toolchain policy | HIGH | No frozen version enforcement |

### Recommendations

1. Create `.github/workflows/ci.yml` with toolchain policy checks
2. Delegate gates to ArqonPilot scripts

---

## Summary Matrix

| Repo | CI Workflow | Toolchain Policy | Gate Scripts | Gap Severity |
|------|-------------|------------------|--------------|--------------|
| ArqonPilot | ✅ Present | ✅ Active | ✅ All | NONE |
| ArqonBus | ❌ Missing | ❌ None | ❌ None | CRITICAL |
| ArqonLattice | ❌ Missing | ❌ None | ❌ None | CRITICAL |
| ArqonStudio | ❌ Missing | ❌ None | ❌ None | CRITICAL |
| ArqonHPO | ❌ Missing | ❌ None | ❌ None | CRITICAL |

---

## FC-2 Requirements

For FC-2 (Local/CI/Release Parity Lock), the following must be addressed:

1. **Create CI workflows** for ArqonBus, ArqonLattice, ArqonStudio, ArqonHPO
2. **Add toolchain policy** enforcement to each repo
3. **Define gate entry points** or delegate to ArqonPilot scripts
4. **Create parity matrix** showing local vs CI vs release behavior

---

**Repo Gate Entry Inventory**: ✅ COMPLETE
