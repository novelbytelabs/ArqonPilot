# Arqon Pilot Incident Response Runbook

This document provides the standard procedure for triaging and recovering from Arqon Pilot service incidents.

## 1. Triage Protocol

1. **STOP**: Do not add more code or deploy changes while a P0/P1 incident is active.
2. **Identify Severity**: Is it P0 (data loss/dead UI), P1 (core feature broken), or P2 (degraded)?
3. **Snapshot**: Create a safety backup of `~/.arqon/pilot/db/` and `~/.pilot/reports/` before any recovery action.
4. **Recovery**: Follow the exact recovery flow in `docs/gotcha-registry.md` if the signature matches.
5. **Document**: If the signature is new, add a new G-XXX entry to `docs/gotcha-registry.md` immediately after recovery.

---

## 2. Common Diagnostic Commands

### Bus Connectivity (G-007)
Signs: ArqonBus chip is DISCONNECTED; telemetry stream is empty.
```bash
# Check if shim is running
./scripts/arqonbus_shim.sh status

# Check if port 9100 is listening
ss -ltnp | grep ':9100'
```

### Database Issues (G-012)
Signs: AGOrg operations return "No such file or directory (os error 2)".
```bash
# Check embedded DB status
./scripts/pilot_local.sh db status

# Check for stale sockets in /tmp
ls -la /tmp/.arqon-pilot/
```

### Total UI Death (G-015)
Signs: Tabs don't switch; buttons are unresponsive; page load is static.
```bash
# Check for JavaScript syntax or duplicate const errors
node -c crates/pilot/src/pilot_ui.js
```

### Toolchain Drift (G-001/G-002)
Signs: `cargo check` fails with "feature edition2024 is required".
```bash
./scripts/verify_toolchain_policy.sh
# If failed, run repair
./scripts/repair_lock_182.sh --no-gate
```

---

## 3. Post-Incident Requirements

Every P0 or P1 incident must produce a post-mortem entry in `docs/gotcha-registry.md`.

**Required details per entry**:
- **Signature**: Exact error message or observable symptom.
- **Cause**: Root cause (e.g., stale binary, drifted dependency).
- **Recovery**: One-command or step-by-step recovery flow.
- **Prevention**: Identification of a new gate or script modification to prevent recurrence.
