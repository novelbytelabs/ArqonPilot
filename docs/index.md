# Arqon Pilot

<div class="pilot-hero">
  <h1 class="pilot-hero-title">Arqon Pilot</h1>
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
