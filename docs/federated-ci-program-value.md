# Federated CI Program Value to Arqon Pilot

This page summarizes what the federated CI/CD program (`docs/federated-ci-program-plan.md`) adds to Arqon Pilot.

## Executive Summary

The federated CI program gives Arqon Pilot a cross-repo operating model for CI/CD that is:

1. deterministic (`same contract locally, in CI, and in release lanes`)
2. safe (`preview-first mutation discipline`)
3. scoped (`AGOrg-aware, not single-repo assumptions`)
4. auditable (`evidence and replay instead of ad-hoc logs`)

Without it, Pilot remains strong as a local control plane but weaker as a federation-wide release and policy orchestrator.

## What It Brings

## 1) Contract Unification Across Lanes

The program defines one canonical preflight/gate contract and applies it to:

1. local developer workflows
2. GitHub Actions workflows
3. release verification flows

Value to Arqon Pilot:

1. eliminates local-pass/CI-fail drift
2. reduces duplicated gate logic in scripts/jobs
3. improves failure consistency and operator trust

## 2) Federated Multi-Repo Orchestration

It formalizes grouped execution (for example `core`, `ui`, `infra`) with dependency-aware ordering.

Value to Arqon Pilot:

1. extends Pilot from repo tool to AGOrg federation orchestrator
2. supports partial failures without evidence corruption across unaffected repos
3. enables predictable cross-repo release motion

## 3) Failure-Class Hardening via Gotchas

The plan explicitly anchors execution to known failure classes (`G-001`, `G-002`, `G-003`, `G-007`, `G-010`, `G-014`, `G-015`, `G-017`).

Value to Arqon Pilot:

1. faster triage and remediation
2. fewer repeated mistakes from context loss
3. stronger runbook-driven operations

## 4) Replay + Provenance Discipline

The plan includes replay bundles and provenance metadata for both success and failure runs.

Value to Arqon Pilot:

1. deterministic incident reconstruction
2. better debugging handoffs across operators/AIs
3. stronger release governance evidence

## 5) Security and Governance Guardrails

The program adds strict allowlisting, protected-branch enforcement, mutation scope controls, and secrets-safe evidence behavior.

Value to Arqon Pilot:

1. safer mutating operations at scale
2. tighter governance integration with policy systems
3. reduced blast radius in automation mistakes

## Strategic Fit With PRODUCTIONIZE

The federated CI program is the execution plane for open `PRODUCTIONIZE` gaps:

1. `P2` deterministic preflight graph
2. `P5` cross-tab command graph orchestration
3. `P6` tamper-evident evidence chain
4. `P9` release train hardening

In short:

1. `PRODUCTIONIZE.md` defines what must be true
2. Federated CI program defines how to operationalize it across repos

## Risks and Caveats

1. Legacy copy exists in `Arqon/docs/polity/` for historical reference; canonical plan is `docs/federated-ci-program-plan.md`.
2. Contract duplication risk exists if implementations diverge between Pilot scripts and CI job steps.
3. Federation scope may outpace current runtime reliability if P7 regression monitoring is not maintained.

## Recommendation

Use the federated CI program as a first-class companion to `PRODUCTIONIZE.md`, with each FC wave explicitly mapped to one or more `P*` waves and hard-close evidence tracked in ArqonPilot release artifacts.
