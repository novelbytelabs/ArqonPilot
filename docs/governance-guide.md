# Governance & Policy Guide

Arqon Pilot features a **Governance Control Plane** that allows you to define, simulate, and strictly enforce operational rules across your fleet of repositories (AGOs). The primary interface for this is the **Settings Tab** in the Arqon Pilot UI.

## Overview

The Governance Engine is built on a few core principles:
- **Persistent Data Layer:** All policies and exceptions are stored centrally in a PostgreSQL database (`agorg_policies`, `policy_exceptions`), offering a single source of truth.
- **Hierarchical Inheritance:** Governance rules can be defined at the **AGOrg (Organization)** level or overridden at the **AGO (Repository)** level.
- **Deterministic Precedence:** When enforcing a rule (like Branch Naming bounds), the engine evaluates in a strict precedence order:
  1. Explicit Deny Exceptions
  2. Explicit Allow Exceptions
  3. AGO (Project) Override Policy
  4. AGOrg (Fleet) Active Policy
  5. Hardcoded Fallback Defaults

## The Settings Tab

The Settings Tab provides a visual interface for managing the complete policy lifecycle:

### Policy Lifecycle
Policies are not blindly overwritten; they move through a deliberate workflow:
1. **Draft:** Edit the raw JSON policy schema in the editor.
2. **Simulate:** Run the drafted policy against your current active fleet. The simulation provides concrete metrics (e.g., `3 Violations, 2 Warnings`) and shows exactly which repositories would be blocked if the policy were activated today.
3. **Activate:** Once the simulation is successful and acceptable, the policy is committed as the new `active` version.

### Managing Exceptions
A strict policy is useless if you have to turn it off entirely to handle an edge case. The Governance Engine includes a dedicated **Exceptions System**.

- **Surgical Overrides:** You can create targeted exceptions for specific rules (e.g., allow `hotfix/issue-123` on a single repository) without compromising the rest of the fleet.
- **Time-To-Live (TTL):** Every exception requires an **expiration date** (`expires_at`), an **owner**, and a **ticket reference**. When the TTL expires, the exception automatically invalidates itself, ensuring temporary bypasses don't become permanent tech debt.
- **Revocation:** Active exceptions can be immediately revoked directly from the UI list.

## CLI Parity

For automated environments and terminal users, the governance rules can be inspected via the CLI. To view the active branch policy for your current AGOrg context, use:

```bash
pilot settings branch --show
```

This ensures identical deterministic enforcement whether you are interacting via the UI, the CLI, or the underlying API endpoints.

## Branch Policy Schema (Example)

Here is a sample Branch Policy JSON schema that enforces standard prefixes, blocks direct commits to main, and configures lifecycle rules.

```json
{
  "kind": "branch",
  "version": 1,
  "naming": {
    "level": "block",
    "required_prefix": ["feat", "fix", "docs", "test", "refactor", "chore", "perf"],
    "separator": "/",
    "body_format": "kebab-case",
    "max_length": 80
  },
  "protected_branches": {
    "level": "block",
    "patterns": ["main", "master", "dev", "release/*"]
  },
  "lifecycle": {
    "auto_prune_merged": {"level": "info", "enabled": true},
    "prune_requires_confirmation": true,
    "confirmation_phrase": "PRUNE",
    "max_stale_days": {"level": "warn", "days": 30}
  },
  "sync": {
    "strategy": "ff-only",
    "auto_fetch_before_sync": true
  },
  "create": {
    "require_preview": true,
    "base_branch_default": "main"
  }
}
```

## Troubleshooting
- **No Active Scope:** If the Settings Tab fails to load policies, ensure you have an **AGOrg Scope** selected in the top navigation bar.
- **Save Failed:** Make sure your drafted policy is valid JSON. Missing commas or unescaped quotes will block draft creation.
- **Cannot Activate:** You must run a successful Simulation on your draft before the Activate button is permitted.
