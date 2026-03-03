# Arqon Pilot Compatibility Matrix

This document defines the verified toolchain, platform, and runtime combinations for Arqon Pilot.

## 1. Toolchain Compatibility

| Component | Required Version | Tested Versions | Notes |
|-----------|------------------|-----------------|-------|
| **Rust (Core)** | `1.82.0` | `1.82.0` | Frozen per G-001/G-002 |
| **Rust (Packaging)** | `1.88.0` | `1.88.0` | Frozen for PyPI builds |
| **Protoc (protobuf)** | `25.8` | `25.8` | G-014: must be installed before build |
| **Python** | `3.10+` | `3.10`, `3.11`, `3.12` | For bindings and ML components |
| **PostgreSQL** | `15+` | `15`, `16` | Pilot-managed embedded preferred |

---

## 2. Platform Compatibility

| OS | Architecture | Status | Notes |
|----|--------------|--------|-------|
| **Ubuntu 22.04** | x86_64 | ✅ Supported | Primary CI target |
| **Ubuntu 24.04** | x86_64 | ✅ Supported | Verified locally |
| **macOS 13+** | arm64 (M-series) | ⚠️ Best Effort | Bus shim paths may differ |
| **Windows WSL2** | x86_64 | ⚠️ Not Tested | Potential socket issues |

---

## 3. Runtime Compatibility

| Component | Minimum Version | Notes |
|-----------|-----------------|-------|
| **ArqonBus** | matches Pilot | G-007: legacy shim required for older bus |
| **Pilot DB** | `15.x` | Managed lifecycle via `pilot db start` |
| **Browser (UI)** | Chrome 120+ | Requires modern JS support for G-015 prevention |

---

## 4. Known Incompatibilities

- **Rust > 1.82.0** (core lane): Causes `edition2024` drift in transitive dependencies (see G-001).
- **Protoc ≠ 25.8**: Causes potential protobuf serialization mismatch (see G-014).
- **Non-Postgres DB**: Arqon Pilot requires the embedded Postgres runtime for AGOrg state.
