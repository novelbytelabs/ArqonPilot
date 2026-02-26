# ArqonPilot

ArqonPilot is a standalone CLI for single-repo and cross-repo engineering operations: oracle indexing/query, healing, release navigation, branch orchestration, security scans, planning, scaffolding, and knowledge capture.

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
