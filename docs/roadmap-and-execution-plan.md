# ArqonPilot Roadmap and Execution Plan

This is the active execution plan for ArqonPilot. It is the canonical status reference for what is complete, what is in progress, and what remains before production-complete operations.

## Why This Document Exists

We are operating with a long-running, multi-wave delivery. Context can be lost across sessions, so this plan is designed to be:

1. operationally precise
2. current-state explicit
3. recovery-oriented
4. easy to resume from at any point

## Current State Snapshot

As of now:

1. Core modules are implemented (`oracle`, `heal`, `navigate`, `branch`, `multi`, `secure`, `plan`, `create`, `know`).
2. Bus bridge and Control Panel are active with Oracle/Heal/Branch/Multi/Telemetry surfaces.
3. Guardrail scripts and pre-push enforcement are active.
4. Main risk area is dependency/toolchain drift in the Rust `1.82.0` core lane.

## Detailed Continuity Snapshot (2026-02-26)

This section is the full continuity handoff context for future sessions.

## Canonical Invariants

1. Core lane is pinned to Rust/Cargo `1.82.0`.
2. Packaging lane is pinned to Rust `1.88.0` and uses `Cargo.lock.packaging`.
3. Protobuf is pinned to `4.25.8` (`protoc` `25.8`) in packaging workflow.
4. Pushes must pass `./scripts/prepush_gate.sh` via `.githooks/pre-push`.
5. Primary push path is `./scripts/push_main.sh` (defaults to current branch).

## Current Branching State

1. `main` and `dev` both exist on remote and are currently synchronized.
2. Recommended operating flow:
- feature -> `dev` -> `main`
- run `./scripts/push_main.sh` on current branch after commit.

## Guardrail Artifacts (Authoritative)

1. `scripts/prepush_gate.sh`
2. `scripts/verify_toolchain_policy.sh`
3. `scripts/verify_git_hook_policy.sh`
4. `scripts/repair_lock_182.sh`
5. `scripts/push_main.sh`
6. `scripts/frozen_versions.sh`
7. `docs/gotcha-registry.md`

## Known Failure Classes

1. Lockfile drift to Rust-incompatible crates (`edition2024` and post-1.82 minimums).
2. Transient DNS/index failures against crates.io.
3. CI false negatives from policy checks that were too strict to script format changes.
4. Toolchain mismatch risk when local shell uses non-rustup `cargo/rustc` binaries.

## Confirmed Recent Fixes

1. Push diagnostics are now explicit via `push_main.sh` (trace + log file path).
2. Pre-push gate has retry logic and DNS diagnostics for transient network failures.
3. Git-hook policy check was updated to match retry-wrapped locked compile command.

## Open Critical Hardening (Next)

1. Extend lock compatibility and repair map for ICU drift chain (`icu_* 2.1.x`) and related transitive dependencies.
2. Keep docs synchronized immediately whenever guardrail behavior changes.

## One-Page Recovery Flow

1. `./scripts/verify_toolchain_policy.sh`
2. If fail: `./scripts/repair_lock_182.sh --no-gate`
3. `./scripts/prepush_gate.sh`
4. `./scripts/push_main.sh`
5. If CI fails: inspect failing step first, then update guardrail script + docs in same change.

## Wave Ledger

## Wave 0 to Wave 8 (Completed Foundations)

Completed and archived in `archive/wave-history/`:

1. Wave 0 / 0.5: extraction baseline, validation, and dependency strategy
2. Wave 1: modularization
3. Wave 2: multi-repo foundation
4. Wave 3: branch + navigate orchestration
5. Wave 4: secure + heal expansion
6. Wave 5: plan/create/know
7. Wave 6 / 6.5: acceptance + dogfooding
8. Wave 7: controlled rollout
9. Wave 8: release readiness

## Wave 9 (Capability Completion Hardening)

Status: `In Progress`

Goals:

1. complete missing production hardening edges for module surfaces
2. tighten operational observability and failure diagnostics
3. ensure GUI and CLI parity where practical

Current focus:

1. Heal and Oracle interaction surfaces in Control Panel
2. robust error surfacing with actionable remediation

## Wave 10 (Packaging and Distribution Reliability)

Status: `In Progress`

Goals:

1. deterministic, reproducible publish path
2. stable Linux runtime behavior in conda environments
3. reliable PyPI install and immediate CLI usability

Known gotchas integrated into process:

1. split toolchain lanes (`1.82.0` core, `1.88.0` packaging)
2. lockfile drift to `edition2024` dependencies
3. protobuf/protoc freeze (`4.25.8` / `25.8`) must remain pinned in workflow
4. pre-push gate must pass before remote push

## Wave 11 (Production Operations and Documentation Closure)

Status: `In Progress`

Goals:

1. full operator-grade docs and troubleshooting completeness
2. clear runbooks for multi-repo apply workflows
3. sustained dogfooding and self-host usage

## Wave 12 (Guardrails and Drift Immunity) NEW

Status: `In Progress`

Objective:

Make toolchain/dependency drift failures detectable, repairable, and auditable by default, both CLI-first and GUI-assisted.

Scope:

1. pre-push hard gate on policy + locked compile + targeted tests
2. precise incompatibility detection in policy checks
3. lock drift recovery automation with exact-version transitions
4. structured logs and operator-facing remediation hints
5. planned GUI `Guardrails` surface for checks and recovery actions

Delivered in this wave (so far):

1. `scripts/prepush_gate.sh` with timestamped logs and remediation output
2. `scripts/verify_toolchain_policy.sh` compatibility checks
3. `scripts/verify_git_hook_policy.sh`
4. `scripts/install_git_hooks.sh` + `.githooks/pre-push`
5. `scripts/repair_lock_182.sh` initial and iterative hardening
6. documentation updates in Developer Guide, Testing Strategy, Troubleshooting
7. drift chain mitigation for `uuid/getrandom/wasip3/wit-bindgen` and `blake3/constant_time_eq`
8. centralized frozen policy constants in `scripts/frozen_versions.sh`
9. enforced core-lane execution via `rustup run 1.82.0` in gate/readiness scripts
10. enforced protobuf/protoc pin validation in `verify_toolchain_policy.sh`

Open tasks:

1. keep `repair_lock_182.sh` transition map updated as new drift crates appear
2. implement Control Panel `Guardrails` tab (or `Dependencies` tab naming if retained):
- policy check action
- drift report action
- repair action
- pre-push gate action
- last-run log viewer
3. add/maintain structured JSON output mode for guardrail scripts to improve UI integration
4. enforce actual runtime toolchain in guardrail commands (not only file pin checks)

Acceptance criteria:

1. push failures always include explicit root cause and next action
2. lock drift is recoverable with one documented command path
3. guardrail checks are available via both CLI and GUI
4. docs remain synchronized with scripts and real-world incident learnings

## Immediate Next Execution Steps

1. finalize `repair_lock_182.sh` for repeatable one-shot recovery across observed drift variants
2. add `Guardrails` tab and backend API endpoints in `pilot serve` UI
3. emit guardrail outcomes into telemetry and audit trail for historical visibility
4. continue module GUI expansion only after guardrail lane is stable

## Resume Checklist (Low-Context Recovery)

When resuming after interruption, run these in order:

1. `./scripts/verify_toolchain_policy.sh`
2. `./scripts/repair_lock_182.sh --no-gate` (if policy fails)
3. `./scripts/prepush_gate.sh`
4. `cargo check -p pilot --locked`
5. inspect latest `~/.pilot/reports/prepush_gate_*.log`

If all pass, continue planned wave implementation.
