# Release Log

This log is the release audit trail. Every release (including alpha) must have a complete evidence set.

## Release Entry Template

Use this structure for every new release:

```md
## vX.Y.Z-alpha.N (YYYY-MM-DD)

- Git tag:
- Commit SHA:
- PyPI version:
- Release type: alpha

### Verification
- prepush gate:
- release readiness:
- Wave I matrix artifact:
- Wave J matrix artifact:
- UI smoke log:
- PyPI visibility check:
- clean venv install + `pilot --help`:

### CI/CD
- CI run ID:
- PyPI run ID:
- Docs run ID:
- GitHub release URL:

### Notes
- Key changes:
- Known limitations:
- Follow-up actions:
```

---

## v0.2.0-alpha.1 (Planned)

- Git tag: `TBD`
- Commit SHA: `TBD`
- PyPI version: `0.2.0a1`
- Release type: alpha

### Verification

- prepush gate: `TBD`
- release readiness: `TBD`
- Wave I matrix artifact: `TBD`
- Wave J matrix artifact: `TBD`
- UI smoke log: `TBD`
- PyPI visibility check: `TBD`
- clean venv install + `pilot --help`: `TBD`

### CI/CD

- CI run ID: `TBD`
- PyPI run ID: `TBD`

### Notes

- Key changes: AGOrg control-plane Wave K hard-close, matrix expansion, reconcile governance hardening.
- Known limitations: ArqonBus compatibility shim may be used in some local environments.
- Follow-up actions: start Wave L tech debt burn-down.
