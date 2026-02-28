# Launch Ops Execution Report

Date: February 28, 2026 (UTC)
Scope: NeuraMinds launch-ops closure execution (Base focus)

## Result Snapshot

- Phase 1: PASS
- Phase 2: PASS
- Phase 3: PASS (chaos restart skipped, missing local `ADMIN_CONTROL_KEY`)
- Phase 4: pending (production URL/secret preflight not run in this cycle)

## Staging Deploy Baseline

- Staging API (`neuraminds-api-base-staging-v1`) live on commit `83084bf`.
- Staging web (`neuraminds-web-base-staging-v4`) live on commit `83084bf`.

## Blocker Closed

Resolved real staging blocker before final pass cycle:

- Endpoint failures:
  - `GET /v1/evm/payouts/health` (500)
  - `GET /v1/evm/payouts/backlog` (500)
  - error: `syntax error at or near "FILTER"`
- Code fix:
  - `app/src/services/database.rs` backlog aggregate changed to valid `MIN(CASE WHEN ...)` expression.
- Additional launch-ops hardening:
  - Added keyed geofence probe override for deterministic staging compliance tests:
    - `app/src/middleware/geo_block.rs`
    - `scripts/launch-ops-execute.sh`
  - Staging env configured with `GEO_TEST_OVERRIDE_KEY`.

## Latest Pass Evidence

Phase 1 (strict staging readiness + freeze):

- `docs/reports/synthetic-monitor-staging-live-20260228T022400Z.json`
- `docs/reports/synthetic-monitor-staging-live-20260228T022400Z.md`
- `docs/reports/dx-terminal-snapshot-staging-20260228T022400Z.json`
- `docs/reports/launch-freeze-staging-20260228T022400Z.json`
- `docs/reports/launch-freeze-staging-20260228T022400Z.md`
- `docs/reports/launch-go-no-go.json` (`GO`)

Phase 2 (worker reliability monitor):

- `docs/reports/launch-ops-phase2-monitor-20260228T022430Z.json`
- `docs/reports/launch-ops-phase2-monitor-20260228T022430Z.md`

Phase 3 (soak + compliance):

- `docs/reports/launch-ops-phase3-soak-20260228T022303Z.json`
- `docs/reports/launch-ops-phase3-soak-20260228T022303Z.md`
- `docs/reports/launch-ops-phase3-compliance-20260228T022303Z.json`

Compliance probe result:

- probe mode: `geo-test-override`
- blocked country probe (`US`): `403` PASS
- allowed country probe (`JP`): `200` PASS

## Remaining Launch-Ops Tasks

1. Run full 24h Phase 3 soak window (`96` samples x `900s`) and retain evidence.
2. Run Phase 3 chaos pause/resume path with valid `ADMIN_CONTROL_KEY`.
3. Execute Phase 4 production strict preflight with final production URLs and secrets.
4. Complete first-2h production synthetic cadence (every 15 minutes) post worker startup.
