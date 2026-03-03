# Settings Tab & Governance Control Plane Plan (Implementation-Grade)

## 0. Objective

Implement a production-grade Governance Control Plane in Arqon Pilot, with **Branch Policy** as phase-1 policy family.

This plan is implementation-ready: big decisions are pre-made so another AI/engineer can execute with minimal ambiguity.

## 1. Immutable Constraints

1. Core lane Rust/Cargo: `1.82.0` (frozen).
2. Packaging lane Rust: `1.88.0` (frozen).
3. Protobuf/protoc: `4.25.8` / `25.8` (frozen).
4. Existing hardcoded behavior (`is_protected_branch`, `is_valid_branch_name`) must remain fallback-compatible.

## 2. Architecture Decisions (Locked)

### 2.1 Governance Engine Placement

1. Do not embed business logic directly in `serve_ui.rs`.
2. Create a dedicated governance module in Pilot crate:
   - `crates/pilot/src/governance/mod.rs`
   - `crates/pilot/src/governance/model.rs`
   - `crates/pilot/src/governance/store.rs`
   - `crates/pilot/src/governance/eval.rs`
   - `crates/pilot/src/governance/lease.rs`
3. `serve_ui.rs` stays transport/orchestration layer.

### 2.2 Policy Lifecycle (No direct overwrite)

States:

1. `draft`
2. `previewed`
3. `approved`
4. `active`
5. `superseded`

Rules:

1. Active policy is immutable.
2. New policy writes create new version row.
3. Activation always references a specific version and simulation artifact.

### 2.3 Deterministic Precedence Contract

Precedence order is fixed:

1. explicit deny exception
2. explicit allow exception
3. AGO override policy
4. AGOrg active policy
5. hardcoded fallback defaults

Tie-break rule for multiple matching exceptions:

1. most-specific scope wins (`repo+rule+operation` > `repo+rule` > `org+rule+operation` > `org+rule`).
2. if equal specificity, `deny` wins.
3. if still equal, newest `created_at` wins.

### 2.4 Multi-Agent Safety

All mutating operations require:

1. `idempotency_key`.
2. lease acquisition on affected resource keys.
3. two-phase mutation (`prepare` then `commit`).

Resource key format (branch policy v1):

1. `branch::<canonical_repo_path>::<branch_name_or_star>`

### 2.5 Decision Record Requirement

Every policy evaluation must emit a decision record row + timeline event, including `policy_hash` and `decision_id`.

## 3. Data Model (Final)

Implementation note:
1. The runtime currently uses Postgres DDL managed in `crates/pilot/src/agorg.rs` + governance store wiring.
2. Treat the SQL blocks in this section as conceptual schema intent.
3. For migration-authoritative SQL, always reconcile against in-tree executable DDL before coding.

### 3.1 Tables

#### `agorg_policies`

```sql
CREATE TABLE IF NOT EXISTS agorg_policies (
  id TEXT PRIMARY KEY,
  agorg_id TEXT NOT NULL,
  ago_path TEXT NULL,
  policy_kind TEXT NOT NULL,
  version INTEGER NOT NULL,
  lifecycle_state TEXT NOT NULL,
  policy_hash TEXT NOT NULL,
  policy_json TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  updated_by TEXT NOT NULL,
  FOREIGN KEY (agorg_id) REFERENCES agorg_scopes(id)
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_agorg_policies_unique
  ON agorg_policies(agorg_id, IFNULL(ago_path,), policy_kind, version);
CREATE INDEX IF NOT EXISTS idx_agorg_policies_active_lookup
  ON agorg_policies(agorg_id, IFNULL(ago_path,), policy_kind, lifecycle_state, version DESC);
```

#### `policy_exceptions`

```sql
CREATE TABLE IF NOT EXISTS policy_exceptions (
  id TEXT PRIMARY KEY,
  agorg_id TEXT NOT NULL,
  ago_path TEXT NULL,
  policy_kind TEXT NOT NULL,
  rule_path TEXT NOT NULL,
  operation_scope TEXT NOT NULL,
  mode TEXT NOT NULL,
  reason TEXT NOT NULL,
  owner TEXT NOT NULL,
  ticket TEXT NOT NULL,
  expires_at INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  FOREIGN KEY (agorg_id) REFERENCES agorg_scopes(id)
);
CREATE INDEX IF NOT EXISTS idx_policy_exceptions_lookup
  ON policy_exceptions(agorg_id, IFNULL(ago_path,), policy_kind, rule_path, operation_scope, expires_at);
```

#### `policy_decisions`

```sql
CREATE TABLE IF NOT EXISTS policy_decisions (
  decision_id TEXT PRIMARY KEY,
  agorg_id TEXT NOT NULL,
  repo_path TEXT NULL,
  policy_kind TEXT NOT NULL,
  policy_hash TEXT NOT NULL,
  action TEXT NOT NULL,
  input_json TEXT NOT NULL,
  result_json TEXT NOT NULL,
  blocked INTEGER NOT NULL,
  exception_applied INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  FOREIGN KEY (agorg_id) REFERENCES agorg_scopes(id)
);
CREATE INDEX IF NOT EXISTS idx_policy_decisions_recent
  ON policy_decisions(agorg_id, policy_kind, created_at DESC);
```

#### `policy_leases`

```sql
CREATE TABLE IF NOT EXISTS policy_leases (
  lease_id TEXT PRIMARY KEY,
  agorg_id TEXT NOT NULL,
  resource_key TEXT NOT NULL,
  holder TEXT NOT NULL,
  expires_at INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  FOREIGN KEY (agorg_id) REFERENCES agorg_scopes(id)
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_policy_leases_resource
  ON policy_leases(agorg_id, resource_key);
```

#### `policy_idempotency`

```sql
CREATE TABLE IF NOT EXISTS policy_idempotency (
  idempotency_key TEXT PRIMARY KEY,
  agorg_id TEXT NOT NULL,
  command TEXT NOT NULL,
  request_hash TEXT NOT NULL,
  response_json TEXT NOT NULL,
  status_code INTEGER NOT NULL,
  created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_policy_idempotency_agorg
  ON policy_idempotency(agorg_id, created_at DESC);
```

## 4. Policy Schema (Branch v1)

This schema is canonical and versioned.

```json
{
  "kind": "branch",
  "version": 1,
  "naming": {
    "level": "warn",
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

## 5. Contracts

### 5.1 Shared Evaluation Envelope

```json
{
  "decision_id": "uuid",
  "policy_kind": "branch",
  "policy_hash": "sha256:...",
  "policy_source": "AGO:/abs/path (v4)",
  "blocked": false,
  "violations": [],
  "warnings": [],
  "infos": [],
  "auto_fixes": [],
  "exception_applied": false,
  "fix_suggestions": []
}
```

### 5.2 API Endpoints

#### Get effective policy

1. `GET /api/settings/policy?kind=branch&scope=agorg|ago&ago_path=<path?>`

Response:

```json
{
  "ok": true,
  "kind": "branch",
  "resolved": true,
  "source": "agorg|ago|fallback",
  "version": 4,
  "lifecycle_state": "active",
  "policy_hash": "sha256:...",
  "policy": { }
}
```

#### Save draft policy

1. `POST /api/settings/policy/draft`

Request:

```json
{
  "idempotency_key": "uuid",
  "kind": "branch",
  "scope": "agorg|ago",
  "ago_path": null,
  "updated_by": "operator",
  "policy": { }
}
```

#### Preview policy (validation + simulation summary)

1. `POST /api/settings/policy/preview`

Request:

```json
{
  "idempotency_key": "uuid",
  "kind": "branch",
  "version": 5
}
```

Response:

```json
{
  "ok": true,
  "version": 5,
  "policy_hash": "sha256:...",
  "validation": {"ok": true, "errors": []},
  "simulation": {
    "repos_scanned": 12,
    "would_block": 3,
    "would_warn": 8,
    "artifact_path": "/home/.../policy_simulation_branch_v5_....json"
  }
}
```

#### Approve policy

1. `POST /api/settings/policy/approve`

Request:

```json
{
  "idempotency_key": "uuid",
  "kind": "branch",
  "version": 5,
  "simulation_artifact": "/home/...json"
}
```

#### Activate policy

1. `POST /api/settings/policy/activate`

Request:

```json
{
  "idempotency_key": "uuid",
  "kind": "branch",
  "version": 5,
  "require_simulation_hash": "sha256:...",
  "holder": "ui:settings"
}
```

#### Resolve policy for repo path

1. `POST /api/settings/policy/resolve`

Request:

```json
{
  "kind": "branch",
  "repo_path": "/abs/repo/path"
}
```

#### Compliance scan

1. `POST /api/settings/compliance_scan`

Request:

```json
{
  "kind": "branch",
  "group": "core",
  "tags": ["apply-pilot"],
  "limit_repos": 200
}
```

Response includes `artifact_path` and summarized counts.

#### Exceptions

1. `GET /api/settings/exceptions?kind=branch`
2. `POST /api/settings/exception`
3. `DELETE /api/settings/exception`

Create request:

```json
{
  "idempotency_key": "uuid",
  "kind": "branch",
  "ago_path": null,
  "rule_path": "branch.naming.required_prefix",
  "operation_scope": "create",
  "mode": "allow",
  "owner": "irbsurfer",
  "ticket": "ARQON-1234",
  "expires_at": 1777777777,
  "reason": "Legacy migration branch transition"
}
```

#### Decision query

1. `GET /api/settings/decisions?kind=branch&limit=100`

## 6. CLI Parity (Must Implement)

Add `pilot policy` subcommand group in `crates/pilot/src/main.rs`.

Commands:

1. `pilot policy get --kind branch [--ago-path <path>]`
2. `pilot policy set-draft --kind branch --file <json>`
3. `pilot policy preview --kind branch --version <n>`
4. `pilot policy approve --kind branch --version <n> --simulation-artifact <path>`
5. `pilot policy activate --kind branch --version <n>`
6. `pilot policy resolve --kind branch --repo-path <path>`
7. `pilot policy scan --kind branch [--group <g>] [--tag <t>]`
8. `pilot policy exceptions list --kind branch`
9. `pilot policy exceptions add ...`
10. `pilot policy exceptions delete --id <uuid>`
11. `pilot policy decisions --kind branch --limit 100`

## 7. File-by-File Implementation Map

### 7.1 Backend

1. `crates/pilot/src/governance/model.rs`
   - policy structs, exception structs, decision envelope structs.
2. `crates/pilot/src/governance/store.rs`
   - DB CRUD, migrations, active policy lookup, exception lookup, idempotency storage.
3. `crates/pilot/src/governance/eval.rs`
   - precedence resolver, evaluator, fix suggestion generation.
4. `crates/pilot/src/governance/lease.rs`
   - acquire/release/cleanup lease API.
5. `crates/pilot/src/governance/mod.rs`
   - public facade used by `serve_ui.rs` and `main.rs`.
6. `crates/pilot/src/serve_ui.rs`
   - add settings endpoints and wire to governance facade.
   - integrate evaluator into branch mutation handlers.
7. `crates/pilot/src/main.rs`
   - add `pilot policy ...` CLI subcommands.

### 7.2 Frontend

1. `crates/pilot/src/serve_ui.rs` (HTML/CSS template)
   - add Settings tab button and panel markup.
2. `crates/pilot/src/pilot_ui.js`
   - settings API functions, rendering, workflow actions, session restore.
   - decision explorer rendering + accessibility states.

### 7.3 Docs

1. `docs/operator-runbook.md`
   - governance operational workflow.
2. `docs/troubleshooting.md`
   - policy failure signatures and recovery.
3. `docs/gotcha-registry.md`
   - new governance gotchas.
4. `docs/PRODUCTIONIZE.md`
   - add governance phase status.

## 8. Algorithms (Reference Pseudocode)

### 8.1 Resolve effective policy

```text
resolve_policy(agorg_id, repo_path, kind):
  canonical_path = canonicalize(repo_path)
  ago = load_active_policy(agorg_id, canonical_path, kind)
  if ago exists: return ago
  agorg = load_active_policy(agorg_id, NULL, kind)
  if agorg exists: return agorg
  return default_policy(kind)
```

### 8.2 Evaluate + exceptions

```text
evaluate(action, input, policy, exceptions):
  applicable = filter_not_expired(exceptions)
  rules = collect_rules(policy, action)
  for rule in rules:
    exc = find_best_exception(applicable, rule, action, input.scope)
    if exc.mode == deny: block
    else if exc.mode == allow: continue
    else evaluate rule by level
  emit deterministic envelope + decision record
```

### 8.3 Idempotency guard

```text
handle_mutation(cmd, idempotency_key, req_hash):
  prev = load_idempotency(idempotency_key)
  if prev exists and prev.req_hash == req_hash: return prev.response
  if prev exists and prev.req_hash != req_hash: reject conflict
  execute
  store response under idempotency key
```

## 9. Tab Interop Rules (Locked)

1. Branch tab remains authoritative for branch execution.
2. Settings tab defines policy and compliance state only.
3. Dependencies tab consumes governance health summary and gate impact.
4. Dashboard shows governance chips and route links, no duplicate policy editors.

## 10. Accessibility + UX Requirements

1. Settings controls all have explicit labels and helper text.
2. Empty states include next action.
3. `role="status" aria-live="polite"` for result regions.
4. `role="alert" aria-live="assertive"` for blocking/critical errors.
5. Keyboard-only path covers full policy lifecycle.

## 11. Gotchas (Execution Guardrails)

1. Default parity must be test-locked vs existing hardcoded checks.
2. Canonicalize paths before policy and exception lookups.
3. Cache policy/exception lookups and invalidate on writes.
4. Enforce simulation before activation.
5. Reject mutation without idempotency key.
6. Reject mutation when lease acquisition fails.
7. Deterministic JSON serialization required for hashes.
8. Never bypass AGOrg scope requirement.

## 12. Testing Matrix

### Unit

1. `test_default_policy_matches_hardcoded`.
2. `test_resolution_order_ago_agorg_fallback`.
3. `test_exception_precedence_allow_deny`.
4. `test_exception_expiry`.
5. `test_policy_hash_stability`.

### Integration

1. policy lifecycle transitions.
2. idempotent replay behavior.
3. lease conflict behavior.
4. branch endpoint enforcement integration.

### E2E

1. settings draft->preview->approve->activate.
2. branch operation warn and block outcomes.
3. compliance scan artifact generation and UI rendering.

### Adversarial

1. concurrent activation race.
2. stale prepare token commit.
3. malformed policy levels/schema.
4. conflicting idempotency key payload.

## 13. Acceptance Criteria (Hard-Close)

1. Settings Governance tab operational with row-based stacked cards.
2. Branch policy lifecycle enforced with immutable active versions.
3. Precedence contract deterministic and test-covered.
4. fallback behavior unchanged when no policy exists.
5. branch enforcement returns structured decision envelope.
6. exceptions require owner/ticket/reason/expiry and respect TTL.
7. compliance scan emits report artifact and summary chips.
8. idempotency + lease safety active on all mutating governance endpoints.
9. CLI/API/UI parity for phase-1 commands complete.
10. `cargo check -p pilot --locked` and `node -c crates/pilot/src/pilot_ui.js` pass.
11. runbook/troubleshooting/gotchas updated in same PR.

## 14. Phased Execution Sequence

### Phase G1 (Core Engine + DB)

1. governance module skeleton.
2. migrations for policy tables.
3. model + store + evaluator baseline.
4. parity unit tests.

### Phase G2 (Lifecycle + Simulation)

1. draft/preview/approve/activate endpoints.
2. policy hash + simulation artifacts.
3. decision records.

### Phase G3 (Branch Enforcement + Exceptions)

1. wire evaluator into branch run.
2. exception CRUD + precedence.
3. structured rejections and fixes.

### Phase G4 (Settings UI + CLI)

1. settings tab UI.
2. `pilot policy` CLI commands.
3. compliance scan rendering and decision explorer.

### Phase G5 (Hard-Close)

1. full acceptance test matrix.
2. adversarial concurrency tests.
3. docs/gotcha/runbook hard-close.

## 15. Operator Narrative (What this enables)

1. define governance safely in one place.
2. preview impact before activation.
3. activate with deterministic confidence.
4. see exactly why operations are warned/blocked.
5. run compliance continuously and export evidence.
6. handle temporary exceptions with accountability and expiry.

This is the required base for the comprehensive AGOrg control plane.

## 16. Implementation Reality Check (2026-03-03) — Hard-Close

This section is the source of truth for current governance completion state.

### 16.1 Completed (Verified In-Tree)

1. CLI parity (`pilot policy ...`) is implemented in `crates/pilot/src/main.rs`:
   - `get`
   - `set-draft`
   - `preview`
   - `approve`
   - `activate`
   - `resolve`
   - `scan`
   - `exceptions list/add/delete`
   - `decisions`
2. Settings API parity is implemented in `crates/pilot/src/serve_ui.rs`:
   - `POST /api/settings/policy/resolve`
   - `POST /api/settings/compliance_scan`
   - `GET /api/settings/decisions`
3. Settings UI parity is implemented in `crates/pilot/src/pilot_ui.js`:
   - resolve action flow
   - compliance scan execution/status flow
   - decisions explorer flow
   - no blocking `alert()` calls in governance flow
4. Governance store parity is implemented in `crates/pilot/src/governance/store.rs`:
   - versioned policy lookup
   - policy status transition update
   - decision query API
   - exception upsert logic compatible with current unique-index strategy.
5. Lifecycle contract decision is explicit:
   - UI/API lifecycle is `draft -> simulate -> activate`.
   - CLI also exposes `approve` for explicit governance workflows and audit discipline.

### 16.2 Verification Evidence

1. `cargo check -p pilot --locked` passed.
2. `cargo test -p pilot --locked` passed.
3. `node -c crates/pilot/src/pilot_ui.js` passed.

### 16.3 Guardrails (Still Mandatory)

1. No placeholders, stubs, or silent fallback paths in governance APIs/UI.
2. No happy-path-only handlers; all endpoints return structured errors.
3. Every “done” claim must include command/test evidence.
4. Frozen constraints remain unchanged:
   - Rust core lane `1.82.0`
   - packaging lane `1.88.0`
   - protobuf `4.25.8` / protoc `25.8`
