# FC-5 Hard Close: Pilot CI Contract Layer

**Date**: 2026-03-03
**Status**: COMPLETE ✅
**Execution**: Arqon Pilot Federated CI Program

---

## 1. Files Changed (Paths)

### New Files Created

| File Path | Description |
|-----------|-------------|
| `ArqonPilot/schemas/ci_contract_commands.json` | JSON Schema for typed CI contract commands |
| `ArqonPilot/scripts/ci_contract.sh` | Main contract command dispatcher (run/replay/repair/readiness/report) |
| `ArqonPilot/scripts/ci_run.sh` | Wrapper for run command |
| `ArqonPilot/scripts/ci_replay.sh` | Wrapper for replay command |
| `ArqonPilot/scripts/ci_repair.sh` | Wrapper for repair command |
| `ArqonPilot/scripts/ci_readiness.sh` | Wrapper for readiness command |
| `ArqonPilot/scripts/ci_report.sh` | Wrapper for report command |
| `ArqonPilot/scripts/ci_contract_parity_test.sh` | CLI/API/UI parity test suite |
| `ArqonPilot/docs/fc1_artifacts/fc5_hardclose.md` | This document |

### Modified Files

| File Path | Changes |
|-----------|---------|
| None | - |

---

## 2. Tests/Validations Run

### Command Tests

| Test | Command | Result |
|------|---------|--------|
| Schema validation | `ci_contract_commands.json` schema check | PASS |
| Readiness check | `./ci_readiness.sh` | PASS |
| Report generation | `./ci_report.sh --type gate_status` | PASS |
| Run with preview | `./ci_run.sh --scope ArqonPilot --gates toolchain_policy --preview --dry-run` | PASS |
| Run execution | `./ci_run.sh --scope ArqonPilot --gates toolchain_policy` | PASS |
| Repair preview | `./ci_repair.sh --type lock_repair --preview --dry-run` | PASS |
| Invalid scope rejection | `./ci_run.sh --scope InvalidRepo --gates toolchain_policy` | PASS (rejected) |

### Parity Test Suite Results

```
FC-5 CLI/API/UI Parity Tests

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
| CI Contract Schema | `ArqonPilot/schemas/ci_contract_commands.json` |
| Contract Command Dispatcher | `ArqonPilot/scripts/ci_contract.sh` |
| Run Command Wrapper | `ArqonPilot/scripts/ci_run.sh` |
| Replay Command Wrapper | `ArqonPilot/scripts/ci_replay.sh` |
| Repair Command Wrapper | `ArqonPilot/scripts/ci_repair.sh` |
| Readiness Command Wrapper | `ArqonPilot/scripts/ci_readiness.sh` |
| Report Command Wrapper | `ArqonPilot/scripts/ci_report.sh` |
| Parity Test Suite | `ArqonPilot/scripts/ci_contract_parity_test.sh` |
| Execution Records | `ArqonPilot/.contract_state/` |

### Schema Capabilities

The `ci_contract_commands.json` schema defines:
- **5 Typed Commands**: run, replay, repair, readiness, report
- **Scope Validation**: ArqonPilot, ArqonBus, ArqonLattice, ArqonStudio, ArqonHPO
- **Policy Checks**: toolchain_policy with frozen versions (Core Rust 1.82.0, Packaging Rust 1.88.0, Protobuf 4.25.8)
- **Contract Preview**: resolved_commands, payload_digest, preview_timestamp, confirmation_type

---

## 4. What Remains for FC-6

### FC-6: Provenance and Replay

**Deliverables**:
1. Full execution provenance recording (command, context, environment, results)
2. Deterministic replay with same inputs produces same outputs
3. Evidence collection and archival
4. Audit trail for compliance

**Required Work**:
- [ ] Enhance execution record format with full provenance data
- [ ] Implement deterministic replay verification
- [ ] Create evidence collection scripts
- [ ] Build audit trail viewer/report generator

### Future Phases (Not in Scope for FC-6)
- FC-7: Federated Orchestration (multi-repo coordination)
- FC-8: Security + Policy Hardening
- FC-9: Release Train Hard-Close

---

## 5. Gotchas Added

**None** - No new gotchas added for FC-5.

The existing gotchas from the registry are covered by:
- **G-001/G-002/G-003**: Scope validation in `check_scope()` function
- **G-005/G-006**: Policy checks in `check_policy()` function  
- **G-007**: Preview capability provides contract transparency
- **G-010**: Contract schema validates all command inputs
- **G-013/G-014/G-015**: Payload digest ensures integrity
- **G-017**: Exit codes defined for all failure modes

---

## 6. FC-5 Completion Checklist

- [x] Typed commands schema (run/replay/repair/readiness/report)
- [x] Scope validation before dispatch (ArqonPilot, ArqonBus, ArqonLattice, ArqonStudio, ArqonHPO)
- [x] Policy checks before dispatch (toolchain_policy, frozen versions)
- [x] Contract preview always available before execution
- [x] CLI/API/UI parity tests pass (14/14)
- [x] Evidence includes resolved command list and payload digest
- [x] Hard-close document created

---

## 7. Execution Evidence

### Payload Digest Example

When running with `--preview`, each execution produces a SHA256 payload digest:

```json
{
  "command": "run",
  "resolved_commands": [
    "scope_check:ArqonPilot",
    "policy_check",
    "toolchain_policy,prepush_gate"
  ],
  "payload_digest": "85db129470428f07126272039cc263c3c6b3ac793ee89aaf26996fdaa99e2204",
  "preview_timestamp": "2026-03-03T15:59:19Z",
  "requires_confirmation": true,
  "confirmation_type": "standard"
}
```

### Gate Execution Evidence

```
[INFO] Executing gates: toolchain_policy (execution_id: 0fe7a404-ea25-45af-ae3d-557ed7343edb)
[INFO] Running toolchain_policy gate...
[policy] rust-toolchain pin
[policy] CI lane pin
[policy] packaging lane pin
[policy] protobuf/protoc freeze pin
[policy] packaging lockfile policy
[policy] lockfiles exist
[policy] lockfile compatibility for Rust 1.82 core lane
[policy] python and cargo versions aligned
Toolchain policy checks passed.
[OK] toolchain_policy: PASS
[OK] Execution completed successfully
```

---

**FC-5 Status**: HARD-CLOSED ✅

Next: Proceed to FC-6: Provenance and Replay
