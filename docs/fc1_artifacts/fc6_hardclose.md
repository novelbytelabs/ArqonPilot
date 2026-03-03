# FC-6 Hard Close: Provenance and Replay

**Date**: 2026-03-03
**Status**: COMPLETE ✅
**Execution**: Arqon Pilot Federated CI Program

---

## 1. Files Changed (Paths)

### New Files Created

| File Path | Description |
|-----------|-------------|
| (None) | - |

### Modified Files

| File Path | Changes |
|-----------|---------|
| `scripts/ci_contract.sh` | Enhanced with full provenance recording, deterministic replay verification, and audit trail reports |
| `crates/pilot/src/main.rs` | Fixed stability probe early exit for reduced latency |

---

## 2. Tests/Validations Run

### Provenance Tests

| Test | Command | Result |
|------|---------|--------|
| Execution record with provenance | Run gate and check record | PASS |
| Environment snapshot | Check rust/cargo/python versions captured | PASS |
| Git info captured | Check branch/commit/dirty status | PASS |
| Payload digest | Verify SHA256 digest generation | PASS |
| Duration tracking | Verify execution time captured | PASS |

### Replay Tests

| Test | Command | Result |
|------|---------|--------|
| Replay with verification | `./ci_contract.sh replay --execution-id <id>` | PASS |
| Deterministic check | Verify payload digest matches | PASS |
| Environment comparison | Current env vs original env | PASS |

### Audit Trail Tests

| Test | Command | Result |
|------|---------|--------|
| Audit trail report | `./ci_report.sh --type audit_trail` | PASS |
| Provenance detail | `./ci_report.sh --type provenance_detail --execution-id <id>` | PASS |
| Full JSON output | Verify structured output | PASS |

### Parity Test Suite Results

```
FC-6 CLI/API/UI Parity Tests (Enhanced)

Test Results:
- Contract schema exists: PASS
- Schema contains all 5 commands: PASS
- Schema has all scope repositories: PASS
- Schema has PolicyCheck: PASS
- Schema has ContractPreview: PASS
- All command scripts exist and are executable: PASS
- CLI readiness check works: PASS
- CLI report command works: PASS
- CLI run preview shows contract preview: PASS
- CLI run preview includes payload digest: PASS
- CLI repair preview shows repair preview: PASS
- JSON output works: PASS
- Gate execution via contract layer: PASS
- Error handling for invalid scope: PASS

Passed:  14
Failed:  0
Skipped: 0

All CLI/API/UI parity tests PASSED
```

---

## 3. Artifact Paths Produced

| Artifact | Path |
|----------|------|
| Execution Records | `.contract_state/*.json` |
| Contract Script | `scripts/ci_contract.sh` |
| Report Command | `scripts/ci_report.sh` |
| Replay Command | `scripts/ci_replay.sh` |

### Provenance Data Structure

Each execution record now contains:

```json
{
  "execution_id": "uuid",
  "command": "run",
  "scope": "ArqonPilot",
  "gates": "toolchain_policy",
  "timestamp": "2026-03-03T17:49:07Z",
  "duration_seconds": 0,
  "status": "SUCCESS",
  "provenance": {
    "environment": {
      "rust_version": "rustc 1.82.0",
      "cargo_version": "cargo 1.82.0",
      "python_version": "Python 3.10.12",
      "shell": "/bin/bash",
      "user": "irbsurfer",
      "hostname": "workstation"
    },
    "git": {
      "branch": "main",
      "commit": "873ef4e...",
      "commit_short": "873ef4e",
      "dirty": "3"
    },
    "payload_digest": "sha256hash"
  },
  "resolved_commands": [
    "scope_check:ArqonPilot",
    "policy_check",
    "toolchain_policy"
  ]
}
```

---

## 4. What Remains for FC-7

### FC-7: Federated Orchestration (Multi-Repo Coordination)

**Deliverables**:
1. Cross-repo dependency resolution
2. Coordinated multi-repo gate execution
3. Federated timeline/events
4. Cross-repo undo/rollback

**Required Work**:
- [ ] Implement dependency graph across repos
- [ ] Create coordinated execution engine
- [ ] Build federated event bus
- [ ] Add cross-repo undo capability

### Future Phases (Not in Scope for FC-7)
- FC-8: Security + Policy Hardening
- FC-9: Release Train Hard-Close

---

## 5. Gotchas Added

**None** - No new gotchas added for FC-6.

The existing gotchas from the registry are covered by:
- **G-001/G-002/G-003**: Scope validation in `check_scope()` function
- **G-005/G-006**: Policy checks in `check_policy()` function  
- **G-007**: Preview capability provides contract transparency
- **G-010**: Contract schema validates all command inputs
- **G-013/G-014/G-015**: Payload digest ensures integrity
- **G-017**: Exit codes defined for all failure modes
- **G-018** (new): Deterministic replay verifies environment consistency

---

## 6. FC-6 Completion Checklist

- [x] Full execution provenance recording (command, context, environment, results)
- [x] Deterministic replay with same inputs produces same outputs (payload digest verification)
- [x] Evidence collection and archival (`.contract_state/` directory)
- [x] Audit trail for compliance (`--type audit_trail` report)
- [x] Provenance detail report (`--type provenance_detail`)
- [x] CLI/API/UI parity tests pass (14/14)
- [x] Hard-close document created

---

## 7. Execution Evidence

### Sample Execution Record

```
Execution ID: 1c132d57-6b73-4360-a6d6-2f44262bb589
Scope: ArqonPilot
Gates: toolchain_policy
Status: SUCCESS
Duration: 0s
Environment: rustc 1.82.0, cargo 1.82.0, Python 3.10.12
Git: main @ 873ef4e (dirty: 3)
Payload Digest: 271a6d1994502762f1ab6206931ecb15513ee87e3d455542eba361170a770f52
```

### Audit Trail Report Output

```
=== Audit Trail ===
Full execution history with provenance

Total audit records: 5

--- Execution: 1c132d57-6b73-4360-a6d6-2f44262bb589 ---
Command: run
Scope: ArqonPilot
Gates: toolchain_policy
Status: SUCCESS
Timestamp: 2026-03-03T17:49:07Z
Duration: 0s
Environment: rustc 1.82.0 (f6e511eec 2024-10-15), cargo 1.82.0 (8f40fc59f 2024-08-21)
Git: main @ 873ef4e
Payload Digest: 271a6d1994502762f1ab6206931ecb15513ee87e3d455542eba361170a770f52
```

---

**FC-6 Status**: HARD-CLOSED ✅

Next: Proceed to FC-7: Federated Orchestration (Multi-Repo Coordination)
