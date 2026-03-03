# FC-6 Hard Close: Provenance and Replay

**Date**: 2026-03-03
**Status**: COMPLETE ✅
**Execution**: Arqon Pilot Federated CI Program

---

## 1. Files Changed (Paths)

### New Files Created

| File Path | Description |
|-----------|-------------|
| `ArqonPilot/schemas/provenance_record.json` | JSON Schema for provenance records |
| `ArqonPilot/scripts/capture_provenance.sh` | Provenance capture script |
| `ArqonPilot/scripts/generate_replay_bundle.sh` | Replay bundle generator |
| `ArqonPilot/scripts/replay_execution.sh` | One-command replay entry |
| `ArqonPilot/docs/fc1_artifacts/fc6_hardclose.md` | This document |

---

## 2. Tests/Validations Run

### Schema Validation

| Test | Result |
|------|--------|
| Provenance schema exists | PASS |
| Schema defines InputPayload | PASS |
| Schema defines EnvironmentSummary | PASS |
| Schema defines GitContext | PASS |
| Schema defines ResolvedOperation | PASS |
| Schema defines OutputRecord | PASS |
| Schema defines Artifact | PASS |
| Schema defines ReplayBundle | PASS |

### Script Tests

| Test | Result |
|------|--------|
| capture_provenance.sh executable | PASS |
| generate_replay_bundle.sh executable | PASS |
| replay_execution.sh --list | PASS (shows executions) |
| Schema validation (JSON) | PASS |

---

## 3. Artifact Paths Produced

| Artifact | Path |
|----------|------|
| Provenance Schema | `ArqonPilot/schemas/provenance_record.json` |
| Provenance Capture | `ArqonPilot/scripts/capture_provenance.sh` |
| Replay Bundle Generator | `ArqonPilot/scripts/generate_replay_bundle.sh` |
| One-Command Replay | `ArqonPilot/scripts/replay_execution.sh` |
| Execution Records | `ArqonPilot/.contract_state/` |
| Replay Archives | `ArqonPilot/.provenance_archive/` |

---

## 4. What Remains for FC-7

### FC-7: Federated Orchestration

**Deliverables**:
1. Grouped execution modes (`core`, `ui`, `infra`, custom AGOrg sets)
2. Dependency-aware ordering with explicit skip/fail semantics
3. Consolidated federation status board in Pilot

**Required Work**:
- [ ] Implement execution group modes
- [ ] Add dependency ordering
- [ ] Create federation status board

### Future Phases (Not in Scope for FC-7)
- FC-8: Security + Policy Hardening
- FC-9: Release Train Hard-Close

---

## 5. Gotchas Added

**None** - No new gotchas added for FC-6.

The existing gotchas from the registry are covered by:
- **G-001/G-002**: Provenance includes environment validation
- **G-005/G-006**: Policy checks included in provenance
- **G-007**: Replay provides full execution transparency
- **G-010**: Schema validates all provenance fields
- **G-013/G-014/G-015**: Payload digests in provenance ensure integrity

---

## 6. FC-6 Completion Checklist

- [x] Provenance record schema (input, resolved ops, env summary, output, artifacts)
- [x] Replay bundle generation (provenance + script + dependencies)
- [x] One-command replay capability (replay_execution.sh)
- [x] Provenance/replay schema tests pass
- [x] Hard-close document created

---

## 7. Implementation Details

### Provenance Schema (provenance_record.json)

The schema defines:
- **InputPayload**: command, scope, gates, policy_overrides, agorg_context, env_vars
- **EnvironmentSummary**: rust_version, cargo_version, python_version, platform, os_kernel, frozen_policy_versions
- **GitContext**: branch, commit, commit_short, dirty, tags, remotes
- **ResolvedOperation**: id, type, target, command, arguments, dependencies, status, result
- **OutputRecord**: status, exit_code, duration_seconds, stdout, stderr, failure_reason, failure_code, remediation_hint, gate_results
- **Artifact**: name, path, type, size_bytes, digest, created_at
- **ReplayBundle**: provenance, signature, replay_script, dependencies, created_at

### One-Command Replay Usage

```bash
# List available executions
./replay_execution.sh --list

# Replay specific execution
./replay_execution.sh <execution-id>

# Replay latest execution
./replay_execution.sh --latest

# Replay from bundle
./replay_execution.sh --bundle <tarball-path>
```

### Execution Record Example

Existing execution records in `.contract_state/` include:
- command, scope, gates, timestamp
- status, exit_code, duration_seconds
- environment (rust_version, cargo_version, python_version, shell, path, user, hostname)
- git (branch, commit, commit_short, dirty)
- payload_digest

---

**FC-6 Status**: HARD-CLOSED ✅

Next: Proceed to FC-7: Federated Orchestration
