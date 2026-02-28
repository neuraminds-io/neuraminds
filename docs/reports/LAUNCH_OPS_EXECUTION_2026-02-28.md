# Launch Ops Execution Report

Date: February 28, 2026 (UTC)
Scope: NeuraMinds launch-ops closure execution (Base focus)

## Result Snapshot

- Phase 1: PASS
- Phase 2: FAIL (real blocker found)
- Phase 3: not started (blocked by Phase 2)
- Phase 4: not started (blocked by Phase 2)

## Phase 1 Evidence (PASS)

Command executed:

```bash
npm run launch:ops:phase1 -- \
  --staging-api-url https://neuraminds-api-base-staging-v1.onrender.com \
  --staging-web-url https://neuraminds-web-base-staging-v4.onrender.com \
  --strict
```

Artifacts generated:

- `docs/reports/synthetic-monitor-staging-live-20260228T014850Z.json`
- `docs/reports/synthetic-monitor-staging-live-20260228T014850Z.md`
- `docs/reports/dx-terminal-snapshot-staging-20260228T014850Z.json`
- `docs/reports/launch-freeze-staging-20260228T014850Z.json`
- `docs/reports/launch-freeze-staging-20260228T014850Z.md`
- `docs/reports/launch-go-no-go.json`
- `docs/reports/launch-go-no-go.md`

Gate outcome:

- strict staging readiness: GO
- staging synthetic monitor: pass
- address manifest check: pass (staging overrides loaded from canonical manifest)

## Phase 2 Evidence (FAIL)

Command executed:

```bash
npm run launch:ops:phase2 -- \
  --staging-api-url https://neuraminds-api-base-staging-v1.onrender.com \
  --staging-web-url https://neuraminds-web-base-staging-v4.onrender.com \
  --monitor-samples 6 \
  --monitor-interval-sec 15
```

Artifacts generated:

- `docs/reports/launch-ops-phase2-workers-20260228T014926Z.log`
- `docs/reports/launch-ops-phase2-monitor-20260228T014926Z.json`
- `docs/reports/launch-ops-phase2-monitor-20260228T014926Z.md`

Failure reason:

- `GET /v1/evm/payouts/health` returned 500 in 6/6 samples.
- `GET /v1/evm/payouts/backlog` returned 500 in 6/6 samples.
- API error payload: `syntax error at or near "FILTER"`.

Impact:

- Launch-critical payout observability endpoints are not healthy in staging.
- Phase 3 soak and Phase 4 cutover are blocked until this is fixed and redeployed.

## Code Fix Implemented (Local)

Fixed payout backlog SQL in backend:

- File: `app/src/services/database.rs`
- Change: removed invalid aggregate `FILTER` placement by switching to a `MIN(CASE WHEN ... THEN updated_at END)` expression.

Validation:

```bash
cargo check --manifest-path app/Cargo.toml
```

Status: pass.

## Immediate Next Actions

1. Deploy current fix to staging API (main branch deploy).
2. Re-run Phase 2 command and require all launch-critical endpoints to pass.
3. Start Phase 3 24h soak only after Phase 2 is green.
4. Run Phase 4 production preflight after successful soak.
