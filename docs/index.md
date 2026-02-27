<div class="pilot-hero">
  <div class="pilot-hero-title">Arqon Pilot</div>
  <p class="pilot-hero-subtitle">Monorepo with<br/>Multi-repo power.</p>
  <p class="pilot-hero-copy">The local control plane for Arqon's ecosystem:<br/>orchestrate branches, enforce policy, self-heal failures, and push safely across the fleet.</p>

  <div class="pilot-pill-row">
    <span class="pilot-pill">🛡️ Safe by Construction</span>
    <span class="pilot-pill">🎯 Deterministic &amp; Replayable</span>
    <span class="pilot-pill">⚡ Cross-Repo Ready</span>
  </div>

  <div class="pilot-powered">Powered by <span class="pilot-powered-chip">Arqon Pilot</span></div>

  <div class="pilot-cta-row">
    <a class="pilot-btn pilot-btn-primary" href="developer-guide/">Get Started</a>
    <a class="pilot-btn pilot-btn-secondary" href="operator-runbook/">See It Live</a>
  </div>
</div>

## Why Arqon Pilot Exists in Arqon

Arqon is a multi-repo system: Rust core, ArqonBus, UI, Python bindings, and docs.
Git manages repositories, but not cross-repo intent, dependency order, or release flow.
Arqon Pilot closes that gap and turns the fleet into one governed operating system.

### Arqon-Specific Value

- **Fleet-Wide Orchestration**
    - `multi`, `branch`, and `navigate` coordinate branches, dependency order, and release flow across repos.
- **Shift-Left CI/CD Governance**
    - Policy, hook, and pre-push gates run locally first so broken changes do not reach CI.
- **Self-Healing and Repair**
    - Heal builds context from failures and runs controlled repair loops with audit evidence.
- **Push Safe as Default**
    - `Push Safe` verifies policy + gate state before push, replacing push-and-pray with deterministic preflight.

## Capabilities

<div class="pilot-feature-grid">
  <div class="pilot-feature-card">
    <h3>🔮 Oracle</h3>
    <p>Tree-sitter parsing, hybrid search, and codebase intelligence for precise context.</p>
  </div>
  <div class="pilot-feature-card">
    <h3>🩹 Heal</h3>
    <p>Failure parsing, repair planning, and verification-gated self-healing workflows.</p>
  </div>
  <div class="pilot-feature-card">
    <h3>🚢 Navigate</h3>
    <p>Release preflight, versioning, changelog flow, and controlled rollout orchestration.</p>
  </div>
  <div class="pilot-feature-card">
    <h3>🌿 Branch</h3>
    <p>Create, sync, status, and prune branch operations across repo cohorts.</p>
  </div>
  <div class="pilot-feature-card">
    <h3>🧭 Multi</h3>
    <p>Cross-repo registry, dependency ordering, linked PR planning, and scoped execution.</p>
  </div>
  <div class="pilot-feature-card">
    <h3>🔐 Secure</h3>
    <p>Security scans and dependency maintenance with dry-run-first and auditable output.</p>
  </div>
  <div class="pilot-feature-card">
    <h3>🗺️ Plan</h3>
    <p>Issue ingestion, scoring, and roadmap generation for execution-focused prioritization.</p>
  </div>
  <div class="pilot-feature-card">
    <h3>🏗️ Create</h3>
    <p>Feature and test scaffolding to accelerate repeatable engineering workflows.</p>
  </div>
  <div class="pilot-feature-card">
    <h3>🧠 Know</h3>
    <p>Decision capture and queryable operational memory for long-term continuity.</p>
  </div>
</div>

## Quick Start

```bash
pip install arqon-pilot
# Linux/Conda: if shared libs fail, configure conda hooks (see Developer Guide)
pilot --help
pilot init
pilot multi register --path /path/to/ArqonContinuum --group core --tag apply-pilot
pilot multi status --tag apply-pilot
pilot branch create feat/pilot-rollout --group core --dry-run
pilot navigate --multi --group core --dry-run
```

## First 5 Minutes (Recommended)

1. Start the control panel:

```bash
pilot serve --ws-url ws://127.0.0.1:9100 --room pilot --channel control --telemetry-channel telemetry --ui-port 7788
```

2. Open `http://127.0.0.1:7788`.
3. In `Dashboard -> System Status`, run:
   - `Policy`
   - `Hook Policy`
   - `Gate`
4. If ArqonBus is down, use:
   - `Start Bus`
   - `Bus Status`
5. Use `Dependencies` tab for drift diagnosis before push.

## Documentation

- [Roadmap & Execution Plan](roadmap-and-execution-plan.md)
- [AGOrg Control Plane Plan](agorg-control-plane-plan.md)
- [Developer Guide](developer-guide.md)
- [Troubleshooting](troubleshooting.md)
- [Testing Strategy](testing-strategy.md)
- [Operator Runbook](operator-runbook.md)
- [Branch Management Guide](branch-management-guide.md)
