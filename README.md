# ArqonPilot

ArqonPilot is the standalone successor to ArqonShip.

## Wave 0 Status

This repo now contains the extracted baseline crate at `crates/pilot` with hard-cut naming:

- Binary: `pilot`
- Config/state path: `.pilot/`
- Release command: `pilot navigate`
- Oracle commands: `pilot oracle scan`, `pilot oracle query --query "..."`

## Quickstart

```bash
cargo run -p pilot -- --help
cargo run -p pilot -- init
cargo run -p pilot -- oracle scan
cargo run -p pilot -- oracle query --query "Where is X?"
cargo run -p pilot -- navigate --dry-run
cargo run -p pilot -- --report-json init
cargo run -p pilot -- heal --log-file test_output.json --plan-only --max-files 6
cargo run -p pilot -- multi register --path /path/to/repo --group core --tag rust
cargo run -p pilot -- multi list --group core
cargo run -p pilot -- multi status --group core
cargo run -p pilot -- multi query --query "state machine" --group core --per-repo-limit 5
cargo run -p pilot -- multi deps set --repo repo-b --depends-on repo-a
cargo run -p pilot -- multi order --group core
cargo run -p pilot -- multi prs create --group core --head-branch dev --base-branch main
cargo run -p pilot -- branch create release/2026-02 --base-branch main --group core --dry-run
cargo run -p pilot -- navigate --multi --dry-run --group core
cargo run -p pilot -- secure scan --group core
cargo run -p pilot -- secure fix --group core
```
