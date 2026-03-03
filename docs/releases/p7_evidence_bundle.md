# P7 Runtime Reliability Supervision Evidence Bundle

Date: 2026-03-03
Scope: Hard-close evidence for P7 in `docs/PRODUCTIONIZE.md`.

## Files Changed (P7 lane)

1. `scripts/arqonbus_shim.sh`
2. `crates/pilot/src/db_runtime.rs`
3. `crates/pilot/src/serve_ui.rs`
4. `crates/pilot/src/pilot_ui.js`
5. `docs/PRODUCTIONIZE.md`

## Root Cause Summary

1. Bus status probe could report false STOPPED in shells with limited PATH if `ss` was not resolvable.
2. DB stopped/error conditions were not consistently surfaced with operator-usable context.
3. UI status surface previously relied too heavily on binary running booleans without explicit failure-state differentiation.

## Implemented Fixes

1. Bus shim hardened:
   - deterministic `ss` resolution (`command -v ss`, `/usr/sbin/ss`, `/bin/ss`)
   - explicit error output and non-zero exit when probe dependency is unavailable.
2. DB status hardened:
   - `DbStatus.error_note` propagated for clearer stopped/failure diagnostics.
3. UI/API health semantics hardened:
   - `/api/health` now emits explicit Bus/DB state values:
     - `RUNNING`
     - `STOPPED`
     - `PROBE_FAILED`
     - `UNAVAILABLE`
   - surfaced `note` fields for remediation context.

## Verification Commands

The following command was executed during remediation review:

```bash
cargo check -p pilot --locked
```

Result:

1. Passes successfully for `pilot` crate after P7 updates.

Additional expected operator verification loop (manual runtime lane):

1. Start serve/UI.
2. Observe `/api/health` baseline.
3. Interrupt Bus/DB process intentionally.
4. Observe degraded state transitions in `/api/health`.
5. Trigger service restart.
6. Confirm return to stable `RUNNING` states without flapping.

## Acceptance Mapping to P7

1. Deterministic startup order and restart semantics:
   - implemented via supervised service startup/restart pathways.
2. Accurate degraded-state reporting:
   - explicit health state contract plus notes now emitted.
3. No silent disconnected status:
   - failures return typed states and operator-visible notes.

## Notes

1. This evidence bundle records the implementation and compile verification trace in-repo for auditability.
2. Runtime interruption/recovery evidence should continue to be captured in `~/.pilot/reports/` during operator acceptance runs.
