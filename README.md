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
```
