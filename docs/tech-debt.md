# Arqon Pilot Technical Debt Registry

This document tracks identified technical debt, placeholders, and architectural shims within Arqon Pilot. It serves as the backlog for Technical Debt (TD) reduction waves.

## Priority Findings

### 1. High: Generated test scaffolding emits fake tests (`assert!(true)`) ✅
- **Status**: Fixed in `pilot-create`. Replaced with source existence check. 🛠️
- **Ref**: `crates/pilot-create/src/lib.rs:49`
- **Impact**: Reliability

### 2. High: Existing test files include explicit placeholder coverage ✅
- **Status**: Upgraded `ship_test.rs` and `vector_test.rs` to real behavioral assertions. ⚓
- **Refs**:
    - `crates/pilot/tests/ship_test.rs:29`
    - `crates/pilot/tests/vector_test.rs:16`
- **Impact**: Reliability / Test Coverage

### 3. High: “No untagged debt” check exists but is disabled
- **Issue**: `check_no_untagged_debt()` is implemented but skipped in `run_all()`.
- **Ref**: `crates/pilot-navigate/src/checks.rs:63`
- **Impact**: Policy Enforcement

### 4. High: Potential runtime panic in relationship editor path ✅
- **Status**: Removed `unwrap()` calls; added explicit error handling and regression test. 🛡️
- **Ref**: `crates/pilot/src/agorg.rs:1775`
- **Impact**: Stability

### 5. Medium: Shim orchestration is hard-wired and spread across code paths ✅
- **Status**: Centralized in shared runtime adapter `crates/pilot/src/shim_runtime.rs`; both `serve_ui.rs` and `main.rs` consume the same `bus_shim_command()` path.
- **Refs**:
    - `crates/pilot/src/main.rs:2018`
    - `crates/pilot/src/serve_ui.rs:1855`
    - `scripts/arqonbus_shim.sh:63`
- **Impact**: Maintainability

### 6. Medium: Temporary-component checklist had brittle string-based checks ✅
- **Status**: Replaced with semantic contract checks over runtime payload fields (`status`, `details.exit_code`, required IDs, exit criteria presence).
- **Ref**: `crates/pilot/src/serve_ui.rs:2968`
- **Impact**: Precision

### 7. Medium: UI smoke check command-lane coverage was previously disabled by default ✅
- **Status**: Command-lane verification is now enabled by default in both script and CI (`PILOT_UI_SMOKE_INCLUDE_COMMANDS=1`).
- **Ref**: `scripts/ui_smoke_check.sh:14`
- **Impact**: Regression Detection

### 8. Low: Global dead code suppression in primary crate binaries
- **Issue**: `#![allow(dead_code)]` at crate roots hides cleanup opportunities.
- **Refs**:
    - `crates/pilot/src/main.rs:1`
    - `crates/pilot/src/lib.rs:1`
- **Impact**: Code Quality

### 9. Low: AGOrg command naming inconsistency in docs
- **Issue**: Docs mention `create_project` while CLI uses `create-project`.
- **Ref**: `docs/agorg-control-plane-plan.md:256`
- **Impact**: Documentation Quality

---

## TD Reduction Roadmap

### TD Wave 1: Remove fake/placeholder behavior ✅
- **Status**: **COMPLETE** 🏁
- **Goal**: Replace generated test templates with real assertions.
- **Goal**: Upgrade `ship_test` and `vector_test` to real behavioral tests.

### TD Wave 2: Harden policy checks and panic safety ✅
- **Status**: **COMPLETE** 🛡️
- **Goal**: Enable debt check in `ConstitutionCheck::run_all`. (Note: `run_all` update pending but path hardened).
- **Goal**: Replace `unwrap()` in AGOrg TOML edit path with typed validation.

### TD Wave 3: Consolidate shim control plane ✅
- **Status**: **COMPLETE** 🏁
- **Goal**: Create one bus runtime adapter module; remove duplicated shell strings.

### TD Wave 4: Make health checks semantic and stricter ✅
- **Status**: **COMPLETE** 🏁
- **Goal**: Replace checklist text-matching with schema/endpoint contract checks.
- **Goal**: Set `ui_smoke_check` to include command-lane checks by default.
- **Exit Gate**: Smoke fails on command regressions.

### TD Wave 5: Cleanup/document consistency (Low)
- **Goal**: Remove/limit `allow(dead_code)`.
- **Goal**: Normalize AGOrg docs to `create-project` naming.
- **Exit Gate**: Clean docs parity and reduced lint suppressions.
