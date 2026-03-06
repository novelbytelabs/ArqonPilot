# Pilot for Pilot Tutorial (Plain-English, Step by Step)

This page is for first-time operators. It assumes you want to use **Arqon Pilot to manage Arqon Pilot**.

## What You Are Doing

You are telling the Control Panel:

1. what AGOrg you are working in,
2. which repos belong to that AGOrg,
3. and then running the safe pre-push policy routine from the UI.

## Before You Start

1. Open a terminal in `ArqonPilot`:
     - `cd ~/Projects/arqon/ArqonPilot`
2. Start the UI with full controls enabled:

```bash
cargo run -p pilot -- serve --ws-url ws://127.0.0.1:9100 --room pilot --channel control --telemetry-channel telemetry --ui-port 7788 --ui-allow-mutations
```

3. Open: `http://127.0.0.1:7788`

## Step 1: Make Sure AGOrg Scope Is Active

1. In the top-right header, click the AGOrg chip (example: `AGORG: ...`).
2. In AGOrg view, verify:
     - the active AGOrg name is correct,
     - root path is the AGOrg root you want,
     - and it is marked active/default scope.
3. If no AGOrg exists yet:
     - create one,
     - set root path,
     - save,
     - switch to it so it becomes active.

Expected result:

- Header shows your AGOrg in scope.

## Step 2: Register ArqonPilot in AGOrg Management (Exact UI Clicks)

This is the part that was missing for most people.

1. Click **AGOrg** tab.
2. Open **AGOrg Management**.
3. Click the **REPO REGISTRY** sub-tab.
4. In **Register Repo**, fill the fields:
     - **Path**: `/home/irbsurfer/Projects/arqon/ArqonPilot`
     - **Name**: `ArqonPilot`
     - **Group**: `core`
     - **Tags**: `pilot` (or your preferred tags)
5. Click **Register**.
6. Confirm the response includes:
     - `"execution_mode": "local_direct"`
7. Registration is idempotent:
     - registering same path again does not create duplicates,
     - changing group/tags updates the existing record.

If you do not see it:

- re-check the path is absolute,
- click Register again,
- then open **Multi** and verify the `Registry` chip count is non-zero.

## Step 3: Verify Policy/Gate Inputs in Settings

1. Click **Settings** tab.
2. Open governance/policy section.
3. In **Select Policy Type**, choose `operator_routine` (or any other family).
4. In **Target AGO**, choose:
     - ArqonPilot path for AGO-level override, or
     - blank for AGOrg-level policy.
5. Click **Refresh Active Policy** (Read).
6. Edit **Policy JSON** and click **Save Draft** (Create/Update).
7. Click **Simulate Draft** and then **Activate Policy** (promote draft to active).
8. In **Policy Versions (Precision CRUD)**:
     - click **Refresh Versions** (List),
     - select a version and click **Load Selected Version** (Read exact version),
     - type `DELETE` and click **Delete Selected Version** (Delete exact version).

Expected result:

- policy lookup returns data, versions list populates, and version-targeted load/delete works.

## Step 4: Run the Safe Routine (UI + Gate)

1. Go to **Multi** first and set selectors:
     - **Group**: `core`
     - **Tags**: `pilot`
2. Click the macro button:
     - `List > Status > Order`
3. Optional planning macro:
     - `DAG > PR Plan`
4. Review results in:
     - Multi action output (default HTML summary),
     - Macro telemetry window (expand/collapse, copy/clear).
5. Go to **Dashboard**.
6. In **System Status**, click in this order:
     - `Policy`
     - `Hook Policy`
     - `Drift`
     - `Gate`
7. Review the output panel after each click.
8. If all pass, run push workflow (`Push Safe` in UI, or terminal push routine if your branch flow requires it).

Expected result:

- status chips become PASS,
- output includes actionable details (not unknown/empty).

## Step 5: If Something Fails

Use this mapping:

- `Policy: FAIL`
    - usually wrong AGOrg scope, missing repo registration, or policy draft not active.
- `Hook: FAIL`
    - local hook not installed or outdated; install/update hooks.
- `Drift: FAIL`
    - repo state differs from expected policy baseline; inspect report and reconcile.
- `Gate: FAIL`
    - upstream check failed; open gate output and follow remediation lines in order.

## Step 6: Run Release Routine (Phase D)

Use this when you want release-grade checks directly in UI.

1. Go to **Dashboard**.
2. Find **Release Routine (Phase D)**.
3. Set:
     - **release label** (example: `0.2.0a1` or `alpha-local`)
     - keep **allow publish push step** unchecked unless you explicitly want push from UI
4. Click **Run Release Routine**.
5. Wait for checklist output:
     - Readiness
     - Compat
     - Migration
     - Publish Gate (and optional Publish Push)
     - Bundle collect
     - Bundle verify
     - Signed evidence export
6. Confirm:
     - every required step is `PASS`
     - readiness score is high/green
     - bundle path is present

Terminal equivalents:

```bash
./scripts/release_readiness_check.sh
./scripts/compat_matrix_smoke.sh
./scripts/migration_smoke_test.sh
./scripts/prepush_gate.sh
./scripts/release_collect_evidence.sh --label <release-label>
<bundle_path>/verify_bundle.sh
```

## One-Command Verification (Terminal)

After UI setup, confirm with:

```bash
./scripts/prepush_gate.sh
```

This should pass before commit/push.

## Fast Checklist

1. UI started on `7788` with mutations enabled.
2. AGOrg active in header chip.
3. ArqonPilot registered in AGOrg Management -> REPO REGISTRY.
4. `operator_routine` policy present/active.
5. Multi selector set (group/tags) and `List > Status > Order` macro runs cleanly.
6. Dashboard: Policy/Hook/Drift/Gate checked.
7. `./scripts/prepush_gate.sh` passes.

When all seven are true, you are in the correct Pilot-for-Pilot operating state.
