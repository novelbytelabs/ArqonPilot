# Arqon Pilot SLO & Error Budget Policy

This document defines the quantitative targets for Arqon Pilot's performance and reliability.

## 1. Service Level Objectives (SLOs)

As a local control plane, SLOs focus on responsiveness and consistency during an active operator session.

| Category | Operation | Metric | Target (Alpha) | Target (Stable) |
|----------|-----------|--------|----------------|-----------------|
| **UI** | Tool/Tab Switch | Latency (p95) | < 300ms | < 100ms |
| **UI** | Page Load (First) | Latency (p95) | < 3.0s | < 1.0s |
| **API** | `/api/health` | Latency (p99) | < 100ms | < 50ms |
| **API** | `/api/agorg/*` | Latency (p95) | < 500ms | < 200ms |
| **DB** | Managed Start | Time to Ready | < 10.0s | < 5.0s |
| **Bus** | SSE Reconnect | Reconnect time | < 2.0s | < 0.5s |

---

## 2. Error Budget Policy

The error budget is the allowed threshold of unreliability before a release is blocked.

- **Budget Cycle**: Per release cycle (not calendar month).
- **Consumption**: Any P0 or P1 bug discovered in production-equivalent environments (Dogfood) consumes the budget.
- **Budget Depleted If**:
  - Any **P0** bug remains open.
  - More than **3** unique **P1** bugs remain open.
- **Consequence**: Channel promotion (e.g., Alpha → Beta) is strictly blocked until the budget is restored (bugs fixed and verified).

---

## 3. Severity Ladder

| Severity | Impact | Typical Response |
|----------|--------|------------------|
| **P0** | **Critical**: Data loss, scope bypass, silent failure, total UI death (internal JS syntax error). | Block release; fix within the same session. |
| **P1** | **Major**: Core feature (e.g., Reconcile) broken for all users with no workaround. | Block promotion; fix in next iteration. |
| **P2** | **Normal**: Feature degraded but workaround exists and is documented. | Fix in the next wave; document in release notes. |
| **P3** | **Minor**: Cosmetic, UX enhancement, or edge-case bug. | Backlog; fix as time permits. |

---

## 4. Measurement

SLOs are measured during:
1. `ui_smoke_check.sh` execution.
2. Manual dogfooding cycles (operator perception).
3. `wave_acceptance_matrix.sh` automation logs.
