<div class="pilot-hero">
  <div class="pilot-hero-title">Arqon Pilot</div>
  <p class="pilot-hero-subtitle">Autonomy isn't a workflow anymore.<br/>It's an operating loop.</p>
  <p class="pilot-hero-copy">Safe multi-repo orchestration, self-healing, and release control<br/>with deterministic governance and auditable execution.</p>

  <div class="pilot-pill-row">
    <span class="pilot-pill">🛡️ Safe by Construction</span>
    <span class="pilot-pill">🎯 Deterministic &amp; Replayable</span>
    <span class="pilot-pill">⚡ Cross-Repo Ready</span>
  </div>

  <div class="pilot-powered">Powered by <span class="pilot-powered-chip">ArqonPilot</span></div>

  <div class="pilot-cta-row">
    <a class="pilot-btn pilot-btn-primary" href="developer-guide/">Get Started</a>
    <a class="pilot-btn pilot-btn-secondary" href="operator-runbook/">See It Live</a>
  </div>
</div>

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
pilot --help
pilot init
pilot multi register --path /path/to/ArqonContinuum --group core --tag apply-pilot
pilot multi status --tag apply-pilot
pilot branch create feat/pilot-rollout --group core --dry-run
pilot navigate --multi --group core --dry-run
```

## Documentation

- [Developer Guide](developer-guide.md)
- [Testing Strategy](testing-strategy.md)
- [Operator Runbook](operator-runbook.md)
- [Branch Management Guide](branch-management-guide.md)
