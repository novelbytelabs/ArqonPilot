# Arqon Pilot

ArqonPilot is a Rust-first, local DevSecOps control plane for the Arqon ecosystem.
It gives teams the unified developer experience of a monorepo while preserving the
security, flexibility, and independence of Arqon's multi-repo architecture.

Arqon repositories span a Rust core engine, ArqonBus messaging, UI surfaces, Python
bindings via maturin, and docs/research repos. Standard Git does not understand these
cross-repo relationships. ArqonPilot is the orchestration layer that coordinates them
as one logical workspace.

## Big Picture for Arqon

ArqonPilot removes the "push-and-pray" loop and turns decentralized repo operations
into governed, deterministic workflows that run locally before CI.

### How this maps to ArqonPilot modules

- Fleet-wide orchestration (`multi`, `branch`, `navigate`)
  - Branch creation/sync/status, DAG ordering, and PR planning across repo cohorts.
- CI/CD shift-left (`policy`, `hook`, `gate`, `push safe`)
  - Toolchain/lock policy and release preflight enforced locally before push.
- Autonomous repair (`heal`, `repair`)
  - Detects and surfaces failures, with guided repair paths and auditable outcomes.
- Unified control panel (`serve` dashboard + tabs)
  - Central command surface for Oracle, Heal, Dependencies, Branch, Multi, and telemetry.

## Arqon Elevator Pitch

ArqonPilot lets Arqon teams branch, test, self-heal, and safely push across an entire
fleet of repositories from one intelligent local control panel.

## Core Modules

- `oracle`
- `heal`
- `navigate`
- `branch`
- `multi`
- `secure`
- `plan`
- `create`
- `know`

## Quickstart

```bash
cargo run -p pilot -- --help
cargo run -p pilot -- init
cargo run -p pilot -- multi register --path /path/to/repo --group core --tag apply-pilot
cargo run -p pilot -- multi status --group core
cargo run -p pilot -- branch create feat/pilot-wave --group core --dry-run
cargo run -p pilot -- serve --ws-url ws://127.0.0.1:9100 --room pilot --channel control --telemetry-channel telemetry
cargo run -p pilot -- serve --ws-url ws://127.0.0.1:9100 --room pilot --channel control --telemetry-channel telemetry --ui-port 7788
cargo run -p pilot -- serve --ws-url ws://127.0.0.1:9100 --room pilot --channel control --telemetry-channel telemetry --ui-port 7788 --ui-allow-command pilot.branch.status --ui-allow-command pilot.multi.status
```

## Critical Linux/Conda Runtime Step

If you installed `arqon-pilot` via PyPI inside conda and see:
`libssl-*.so.10` or `libcrypto-*.so.10` not found, configure the env runtime path.

Do this with conda activation hooks (recommended), not global `.bashrc`:

```bash
mkdir -p "$CONDA_PREFIX/etc/conda/activate.d" "$CONDA_PREFIX/etc/conda/deactivate.d"

cat > "$CONDA_PREFIX/etc/conda/activate.d/arqon_pilot_libs.sh" <<'EOF'
export _ARQONPILOT_OLD_LD_LIBRARY_PATH="${LD_LIBRARY_PATH-}"
export LD_LIBRARY_PATH="$CONDA_PREFIX/lib/python3.10/site-packages/arqon_pilot.libs:${LD_LIBRARY_PATH-}"
EOF

cat > "$CONDA_PREFIX/etc/conda/deactivate.d/arqon_pilot_libs.sh" <<'EOF'
export LD_LIBRARY_PATH="${_ARQONPILOT_OLD_LD_LIBRARY_PATH-}"
unset _ARQONPILOT_OLD_LD_LIBRARY_PATH
EOF
```

Then reactivate and verify:

```bash
conda deactivate
conda activate helios-gpu-118
pilot --help
```

## Testing

Run the full matrix:

```bash
./scripts/test_matrix.sh all
```

Run by category:

```bash
./scripts/test_matrix.sh unit
./scripts/test_matrix.sh integration
./scripts/test_matrix.sh e2e
./scripts/test_matrix.sh regression
./scripts/test_matrix.sh adversarial
```

Release gate:

```bash
./scripts/release_readiness_check.sh
```

Mandatory pre-push gate (run before every commit/push):

```bash
./scripts/prepush_gate.sh
```

Automate this with a git hook (recommended, one-time per clone):

```bash
./scripts/install_git_hooks.sh
```

CI also enforces this policy via `./scripts/verify_git_hook_policy.sh`.

If lockfiles drift to Rust-2024-only dependencies and pushes fail on Rust 1.82:

```bash
./scripts/repair_lock_182.sh
```

## Packaging

PyPI packaging uses `maturin`.

```bash
python3 -m pip install maturin
maturin build --release --locked --out dist
./scripts/pypi_smoke_check.sh
```

CI workflows:

- `.github/workflows/ci.yml`
- `.github/workflows/pypi.yml`

## Release Management (Alpha)

For non-half-step release process and evidence requirements:

1. `docs/release-playbook.md` (authoritative release procedure)
2. `docs/release-log.md` (auditable release journal)
3. `docs/releases/0.2.0-alpha.1.md` (version-specific release notes)

Collect release evidence bundle/logs in one command:

```bash
./scripts/release_collect_evidence.sh --label 0.2.0a1
```

## Documentation

Primary docs are in `docs/` and published with MkDocs.

- `docs/developer-guide.md`
- `docs/testing-strategy.md`
- `docs/bus-integration-guide.md`
- `docs/operator-runbook.md`
- `docs/branch-management-guide.md`
- `archive/docs/pilot-deep-dive-plan.md` (archived)

## Publish Docs to GitHub Pages

Docs deploy via `.github/workflows/docs.yml`.

1. In GitHub: `Settings -> Pages -> Build and deployment`.
2. Set `Source` to `GitHub Actions`.
3. Push to `main` (or run the `Docs (MkDocs)` workflow manually).
