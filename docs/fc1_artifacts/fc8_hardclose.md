# FC-8: Security + Policy Hardening - Hard Close

**Status**: HARD-CLOSED  
**Date**: 2026-03-03

## Deliverables

### 1. Command Allowlist + Mutation Scope Controls

Implemented in `crates/pilot/src/governance/model.rs`:

- `CommandCategory` enum: Read, BranchCreate, BranchModify, BranchDestroy, Policy, Release, Admin
- `CommandAllowlist` struct: enabled_categories, blocked_commands, confirmation_required
- `CommandScope` enum: Local, DryRun, Full
- `MutationControlPolicy` struct: integrates with BranchPolicy

### 2. Protected-Branch Typed Confirmations

Enhanced in `crates/pilot/src/governance/eval.rs`:

- `required_confirmation()` function now considers:
  - Prune operations (lifecycle policy)
  - Protected branch patterns (protected_branches policy)
  - Destructive actions via mutation_control policy
- Default confirmation type: `TypedPhrase` for destructive operations
- Default confirmation phrase: "CONFIRM"

### 3. Secrets-Safe Logging

Implemented in `crates/pilot/src/governance/eval.rs`:

- `redact_secrets()` function: redacts patterns from evidence/logs
- Default redaction patterns:
  - `(?i)(api[_-]?key|secret[_-]?key|password|token|auth)[\s:=]+[\S]+`
  - `(?i)ghp_[a-zA-Z0-9]{36}`
  - `(?i)github_pat_[a-zA-Z0-9_]{22,}`
  - `sk-[a-zA-Z0-9]{48}`
- Controlled by `secrets_safe_logging` flag in MutationControlPolicy

## Test Evidence

### Unit Tests

```bash
cargo test -p pilot governance::eval::tests
# All 19 tests passed
```

### Integration Tests

```bash
cargo test -p pilot policy_parity_integration_test
# test_policy_parity_round_trip passed
```

### Existing E2E/Regression Tests

All existing tests pass:
- `policy_workflow_e2e_test`
- `policy_adversarial_test`

## Files Changed

| File | Changes |
|------|---------|
| `crates/pilot/src/governance/model.rs` | Added CommandCategory, CommandAllowlist, CommandScope, MutationControlPolicy; updated BranchPolicy |
| `crates/pilot/src/governance/eval.rs` | Added check_command_allowlist(), redact_secrets(), AllowlistCheckResult; updated required_confirmation() |
| `crates/pilot/Cargo.toml` | Added regex dependency |

## Commands Run

1. `cargo check -p pilot` - Verify compilation
2. `cargo test -p pilot` - All tests pass

## Hard-Close Evidence

- [x] Security tests for blocked disallowed commands pass
- [x] No secrets leak in standard evidence logs (via redact_secrets)
- [x] Protected-branch typed confirmations enforced
- [x] Command allowlist controls implemented
- [x] All existing tests pass (backward compatibility)
