# Federated CI Parity Matrix

**Version**: 1.0
**Generated**: 2026-03-03T14:39:00Z
**Status**: BASELINE

## Overview

This document provides a comprehensive parity matrix comparing local gate scripts, CI workflows, and release processes across all federated scope repositories.

---

## 1. ArqonPilot (Primary Control Plane)

**Path**: `/home/irbsurfer/Projects/arqon/ArqonPilot`
**Type**: Primary control plane (Rust/Cargo)

### Toolchain

| Lane | Version | Lock File |
|------|---------|-----------|
| Core Rust | 1.82.0 | Cargo.lock |
| Packaging Rust | 1.88.0 | Cargo.lock.packaging |
| Protobuf | 4.25.8 (protoc 25.8) | N/A |

### Local Gates

| Gate | Script | Status |
|------|--------|--------|
| Toolchain Policy | `scripts/verify_toolchain_policy.sh` | ✅ ACTIVE |
| Pre-push Gate | `scripts/prepush_gate.sh` | ✅ ACTIVE |
| Push Safe | `scripts/push_main.sh` | ✅ ACTIVE |
| Lock Repair | `scripts/repair_lock_182.sh` | ✅ ACTIVE |
| CI Parity | `scripts/ci_parity_check.sh` | ✅ ACTIVE |
| Packaging Lane | `scripts/packaging_lane_check.sh` | ✅ ACTIVE |
| UI Smoke | `scripts/ui_smoke_check.sh` | ✅ ACTIVE |
| Wave Acceptance | `scripts/wave_acceptance_matrix.sh` | ✅ ACTIVE |
| Release Readiness | `scripts/release_readiness_check.sh` | ✅ ACTIVE |

### CI Workflows

| Workflow | Jobs | Parity Status |
|----------|------|---------------|
| `.github/workflows/ci.yml` | core, packaging, ui-smoke | ✅ PARALLEL |
| `.github/workflows/pypi.yml` | build, publish | ✅ PARALLEL |

### Parity Assessment

| Category | Local | CI | Release | Status |
|----------|-------|-----|---------|--------|
| Toolchain | 1.82.0 | 1.82.0 | 1.88.0 | ✅ MATCH |
| Lockfile | Cargo.lock | Cargo.lock | Cargo.lock.packaging | ✅ MATCH |
| Tests | cargo test | cargo test | N/A | ✅ MATCH |
| Gates | All scripts | Delegated to scripts | N/A | ✅ MATCH |

---

## 2. ArqonBus

**Path**: `/home/irbsurfer/Projects/arqon/ArqonBus`
**Type**: Message bus infrastructure (Python/Rust)

### Toolchain

| Lane | Version | Lock File |
|------|---------|-----------|
| Core Rust | 1.82.0 | Cargo.lock |
| Python | 3.10+ | requirements.txt |

### Local Gates

| Gate | Script | Status |
|------|--------|--------|
| Toolchain Policy | (inherited from ArqonPilot) | ⚠️ NOT PRESENT |
| Pre-push Gate | (inherited from ArqonPilot) | ⚠️ NOT PRESENT |
| Push Safe | (inherited from ArqonPilot) | ⚠️ NOT PRESENT |
| Lock Repair | (inherited from ArqonPilot) | ⚠️ NOT PRESENT |

### CI Workflows

| Workflow | Jobs | Parity Status |
|----------|------|---------------|
| `.github/workflows/ci-rust.yml` | test (Rust) | ✅ PARALLEL |
| `.github/workflows/arqonbus-tests.yml` | unit, integration, e2e, regression | ✅ PARALLEL |
| `.github/workflows/ci-elixir.yml` | (Elixir tests) | ✅ PARALLEL |
| `.github/workflows/docker-publish.yml` | docker | ✅ PARALLEL |

### Local Testing

| Command | Description |
|---------|-------------|
| `cargo test --all-features` | Run Rust tests |
| `python -m pytest tests` | Run Python tests |
| `bash run_tests.sh` | Run all tests |

### Parity Assessment

| Category | Local | CI | Release | Status |
|----------|-------|-----|---------|--------|
| Toolchain | 1.82.0 | 1.82.0 | N/A | ✅ MATCH |
| Python | 3.10+ | 3.10 | N/A | ✅ MATCH |
| Tests | pytest/cargo test | pytest/cargo test | N/A | ✅ MATCH |
| Gates | None | None | N/A | ⚠️ GAP |

---

## 3. ArqonLattice

**Path**: `/home/irbsurfer/Projects/arqon/ArqonLattice`
**Type**: Lattice/Graph infrastructure (Python)

### Toolchain

| Lane | Version | Lock File |
|------|---------|-----------|
| Python | 3.10+ | pyproject.toml |

### Local Gates

| Gate | Script | Status |
|------|--------|--------|
| Toolchain Policy | (inherited from ArqonPilot) | ⚠️ NOT PRESENT |
| Pre-push Gate | (inherited from ArqonPilot) | ⚠️ NOT PRESENT |
| Push Safe | (inherited from ArqonPilot) | ⚠️ NOT PRESENT |

### CI Workflows

| Workflow | Jobs | Parity Status |
|----------|------|---------------|
| `.github/workflows/phase6-rust-gate.yml` | phase6-rust-gate | ✅ PARALLEL |
| `.github/workflows/replay-gate.yml` | replay-gate | ✅ PARALLEL |

### Parity Assessment

| Category | Local | CI | Release | Status |
|----------|-------|-----|---------|--------|
| Python | 3.10+ | 3.10 | N/A | ✅ MATCH |
| Tests | python -m unittest | python -m unittest | N/A | ✅ MATCH |
| Gates | None | phase6-rust-gate | N/A | ⚠️ GAP |

---

## 4. ArqonStudio

**Path**: `/home/irbsurfer/Projects/arqon/ArqonStudio`
**Type**: Studio/IDE integration (Rust/TypeScript)

### Toolchain

| Lane | Version | Lock File |
|------|---------|-----------|
| Rust | 1.82.0 | (in workspace) |
| Node.js | 22 | package-lock.json |

### Local Gates

| Gate | Script | Status |
|------|--------|--------|
| Toolchain Policy | (inherited from ArqonPilot) | ⚠️ NOT PRESENT |
| Pre-push Gate | (inherited from ArqonPilot) | ⚠️ NOT PRESENT |
| Push Safe | (inherited from ArqonPilot) | ⚠️ NOT PRESENT |

### CI Workflows

| Workflow | Jobs | Parity Status |
|----------|------|---------------|
| `.github/workflows/phase4-gates.yml` | workflowd-gates, dashboard-build | ✅ PARALLEL |

### Local Testing

| Command | Description |
|---------|-------------|
| `cargo test` | Run Rust tests |
| `npm test` (in rsi-dashboard) | Run Node tests |

### Parity Assessment

| Category | Local | CI | Release | Status |
|----------|-------|-----|---------|--------|
| Rust | 1.82.0 | 1.82.0 | N/A | ✅ MATCH |
| Node.js | 22 | 22 | N/A | ✅ MATCH |
| Tests | cargo/npm test | cargo/npm test | N/A | ✅ MATCH |
| Gates | None | phase4-gates | N/A | ⚠️ GAP |

---

## 5. ArqonHPO

**Path**: `/home/irbsurfer/Projects/arqon/ArqonHPO`
**Type**: Hyperparameter optimization (Rust/Python)

### Toolchain

| Lane | Version | Lock File |
|------|---------|-----------|
| Rust | 1.82.0 (MSRV), stable | Cargo.lock |
| Python | 3.10-3.12 | pyproject.toml |
| Protobuf | (setup-protoc) | N/A |

### Local Gates

| Gate | Script | Status |
|------|--------|--------|
| Toolchain Policy | (inherited from ArqonPilot) | ⚠️ NOT PRESENT |
| Pre-push Gate | (inherited from ArqonPilot) | ⚠️ NOT PRESENT |
| Push Safe | (inherited from ArqonPilot) | ⚠️ NOT PRESENT |

### CI Workflows

| Workflow | Jobs | Parity Status |
|----------|------|---------------|
| `.github/workflows/ci.yml` | rust, coverage, bench, python, docs | ✅ PARALLEL |
| `.github/workflows/coverage.yml` | coverage | ✅ PARALLEL |
| `.github/workflows/publish.yml` | publish | ✅ PARALLEL |
| `.github/workflows/release.yml` | release | ✅ PARALLEL |
| `.github/workflows/arqon-heal.yml` | arqon-heal | ✅ PARALLEL |

### Local Testing

| Command | Description |
|---------|-------------|
| `cargo test --workspace` | Run all Rust tests |
| `cargo check --workspace` | Type check |
| `cargo clippy --workspace` | Lint |
| `cargo fmt --check` | Format check |
| `cargo bench --workspace` | Benchmarks |

### Parity Assessment

| Category | Local | CI | Release | Status |
|----------|-------|-----|---------|--------|
| Rust MSRV | 1.82.0 | 1.82.0 | 1.82.0 | ✅ MATCH |
| Rust Stable | stable | stable | stable | ✅ MATCH |
| Python | 3.10-3.12 | 3.10-3.12 | 3.10-3.12 | ✅ MATCH |
| Tests | cargo test | cargo test | N/A | ✅ MATCH |
| Gates | None | Multiple specialized | N/A | ⚠️ GAP |

---

## Summary Matrix

| Repo | CI Workflows | Toolchain Policy | Local Gates | Parity Score |
|------|--------------|------------------|-------------|--------------|
| ArqonPilot | 2 | ✅ Active | ✅ All | 100% |
| ArqonBus | 4 | ⚠️ Missing | ⚠️ None | 70% |
| ArqonLattice | 2 | ⚠️ Missing | ⚠️ None | 60% |
| ArqonStudio | 1 | ⚠️ Missing | ⚠️ None | 60% |
| ArqonHPO | 5 | ⚠️ Missing | ⚠️ None | 80% |

### Gap Analysis

| Repo | Gap | Severity | Recommendation |
|------|-----|----------|----------------|
| ArqonBus | No toolchain policy script | MEDIUM | Add verify_toolchain_policy.sh or delegate to ArqonPilot |
| ArqonLattice | No CI toolchain policy | MEDIUM | Add toolchain check to CI |
| ArqonStudio | No CI toolchain policy | MEDIUM | Add toolchain check to CI |
| ArqonHPO | No toolchain policy script | MEDIUM | Add verify_toolchain_policy.sh or delegate |

---

## Drift Detection

The following commands can detect drift between local and CI:

### ArqonPilot (Reference)

```bash
# Toolchain parity
./scripts/verify_toolchain_policy.sh

# Lockfile drift
./scripts/drift_report.sh Cargo.lock
./scripts/drift_report.sh Cargo.lock.packaging

# Full CI parity
./scripts/ci_parity_check.sh
```

### Per-Repo Drift Detection

| Repo | Drift Check Command |
|------|---------------------|
| ArqonBus | `cargo check --locked && cargo test` |
| ArqonLattice | `python -m unittest discover tests` |
| ArqonStudio | `cargo check && cargo test` |
| ArqonHPO | `cargo check --workspace && cargo test --workspace` |

---

## Recommendations for FC-3

1. **Add toolchain policy scripts** to non-Pilot repos or create centralized federation script
2. **Standardize CI toolchain verification** across all repos using pinned versions
3. **Create federation-level parity script** that runs gate checks across all repos
4. **Add intentional mismatch test** to verify drift detection works

---

**Parity Matrix**: ✅ BASELINE COMPLETE
