# Troubleshooting

This page captures high-impact operational issues and exact fixes.

## 1) Linux/Conda: `libssl-*.so.10` or `libcrypto-*.so.10` not found

Symptom:

```bash
pilot: error while loading shared libraries: libssl-....so.10: cannot open shared object file
```

Cause: the `arqon-pilot` wheel bundles runtime libs in `site-packages/arqon_pilot.libs`, but the env runtime loader path does not include that directory.

Fix (recommended): add conda activation/deactivation hooks for the current environment only.

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

Then reactivate:

```bash
conda deactivate
conda activate helios-gpu-118
pilot --help
```

Important:
- Do not put this in global `.bashrc`.
- Keep it env-scoped.

## 2) `pilot serve` command missing

Symptom:
- `pilot --help` does not list `serve`.

Cause:
- You are running an older installed package (for example `arqon-pilot==0.1.1`) that predates this command.

Fix options:
1. Run local source directly:

```bash
cargo run -p pilot -- --help
cargo run -p pilot -- serve --help
```

2. Install an updated package version that includes `serve`.

## 3) ArqonBus reachable but no responses in UI

Checks:
1. Verify Bus URL:

```bash
echo "$ARQONBUS_WS_URL"
```

2. Confirm Pilot bridge and UI are running:

```bash
pilot serve --ws-url ws://127.0.0.1:9100 --room pilot --channel control --telemetry-channel telemetry --ui-port 7788
```

3. Open the UI:
- `http://127.0.0.1:7788`

4. If auth is enabled, ensure JWT exists:

```bash
echo "$ARQONBUS_AUTH_JWT" | wc -c
```

## 4) Command rejected by Bus bridge

The bridge currently only accepts namespaced `pilot.*` commands and strict contract payloads with:

```json
{ "schema_version": 1, ... }
```

Unknown fields and wrong schema versions are rejected by design.

## 5) Pre-push fails with `edition2024` dependency errors

Symptom (examples):

```text
feature `edition2024` is required
The package requires the Cargo feature called `edition2024`
```

or

```text
error: failed to parse manifest ... comfy-table-7.2.2/Cargo.toml
```

Cause:
- `Cargo.lock` drifted to dependencies requiring Rust 1.85+ while core lane is pinned to Rust/Cargo `1.82.0`.

Canonical recovery flow:

```bash
./scripts/repair_lock_182.sh --no-gate
./scripts/prepush_gate.sh
git push origin main
```

Why this works:
1. `repair_lock_182.sh` restores or force-pins Rust-1.82-compatible lock state.
2. `prepush_gate.sh` validates policy + locked compile + targeted locked tests.
3. Push proceeds only after the gate passes.

Gotchas:
1. Ambiguous package names during pinning:
- Use exact package IDs: `name@from_version` (for example `getrandom@0.4.1`).

2. Transitive chain issues:
- A crate can reintroduce Rust-2024 dependencies indirectly (`uuid -> getrandom -> wasip3 -> wit-bindgen`).
- Another known drift source is `constant_time_eq 0.4.x` (also requires `edition2024`).
- `constant_time_eq` cannot always be pinned directly; in some lock states it is constrained by `blake3` (`^0.4.2`), so `blake3` must be downgraded first.

3. Logging location:
- Gate log is written to `~/.pilot/reports/`.
- If that directory is not writable, gate falls back to `/tmp/pilot-reports/`.

4. Packaging lane is separate:
- PyPI packaging lane can use newer toolchain and `Cargo.lock.packaging`.
- Core lane still must satisfy Rust `1.82.0` with `Cargo.lock`.
