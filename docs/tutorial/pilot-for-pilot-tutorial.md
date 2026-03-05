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

## Step 2: Register ArqonPilot in Multi Tab (Exact UI Clicks)

This is the part that was missing for most people.

1. Click **Multi** tab.
2. In the **Register Repo** section, fill the fields:
     - **Path**: `/home/irbsurfer/Projects/arqon/ArqonPilot`
     - **Name**: `ArqonPilot`
     - **Group**: `core`
     - **Tags**: `apply-pilot,operator`
3. Click **Register**.
4. Confirm the response includes:
     - `"execution_mode": "local_direct"`
5. Click **Status** or **List** in the same tab.
6. Confirm `ArqonPilot` appears in output.

If you do not see it:

- re-check the path is absolute,
- click Register again,
- then re-run List/Status.

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

1. Go to **Dashboard**.
2. In **System Status**, click in this order:
     - `Policy`
     - `Hook Policy`
     - `Drift`
     - `Gate`
3. Review the output panel after each click.
4. If all pass, run push workflow (`Push Safe` in UI, or terminal push routine if your branch flow requires it).

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

## One-Command Verification (Terminal)

After UI setup, confirm with:

```bash
./scripts/prepush_gate.sh
```

This should pass before commit/push.

## Fast Checklist

1. UI started on `7788` with mutations enabled.
2. AGOrg active in header chip.
3. ArqonPilot registered in Multi tab.
4. `operator_routine` policy present/active.
5. Dashboard: Policy/Hook/Drift/Gate checked.
6. `./scripts/prepush_gate.sh` passes.

When all six are true, you are in the correct Pilot-for-Pilot operating state.
