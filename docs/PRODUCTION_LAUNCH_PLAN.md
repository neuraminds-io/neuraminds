# Production Launch Plan

## Objective
Ship a deterministic, auditable production launch process with hard go/no-go gates.

## Scope
- Backend API + Solana programs + web app.
- CI/CD, security controls, launch runbooks, and operational checks.
- Secrets provisioning remains external; this plan enforces validation and fail-fast behavior.

## Phase 1: Gate Baseline
- [x] Add machine-readable launch config validation.
- [x] Add one-command launch readiness runner.
- [x] Ensure production gate reports are persisted in `docs/reports/`.

## Phase 2: Web Production Hardening
- [x] Add strict security headers for web responses.
- [x] Disable framework fingerprint header.
- [x] Align production CDN allowlists with current brand domain.

## Phase 3: CI Launch Enforcement
- [x] Add dedicated launch-readiness workflow.
- [x] Enforce config validation + production gates + frontend build + backend check.
- [x] Persist readiness artifacts.

## Phase 4: Operational Runbook
- [x] Create launch command center with exact cutover sequence.
- [x] Define go/no-go criteria and rollback trigger thresholds.
- [x] Define post-launch monitoring checkpoints.

## Phase 5: End-to-End Execution
- [x] Execute readiness command locally (soft-secret mode).
- [x] Generate and inspect reports.
- [x] Generate machine-readable launch go/no-go summary report.
- [x] Close `backend_cargo_check` timeout blocker (strict now passes cargo check).
- [x] Close remaining blocker for launch (`web_build` strict gate now passes locally).

## Success Criteria
- `launch-config-report.json` and `production-loop-report.json` both generated.
- No required gate failures.
- CI has a dedicated launch-readiness workflow.
- Clear launch/rollback command sequence documented.
