# Launch Ops Focus Runbook

Last updated: February 28, 2026
Owner: launch ops team

## Objective

Close launch operations for Base with deterministic evidence and no new feature scope.
Target readiness date: March 1, 2026.

## Scope Lock

- Network focus: Base only.
- Staging network: Base Sepolia (`84532`).
- Production network: Base mainnet (`8453`).
- Infra: Render-managed API/web/workers.
- Compliance posture: US-restricted writes.
- Native token launch: out of scope.

## Phase Commands

Phase 1 (stabilize + freeze staging):

```bash
npm run launch:ops:phase1 -- \
  --staging-api-url https://neuraminds-api-base-staging-v1.onrender.com \
  --staging-web-url https://neuraminds-web-base-staging-v4.onrender.com \
  --strict
```

Phase 2 (worker reliability evidence):

```bash
npm run launch:ops:phase2 -- \
  --staging-api-url https://neuraminds-api-base-staging-v1.onrender.com \
  --staging-web-url https://neuraminds-web-base-staging-v4.onrender.com \
  --monitor-samples 8 \
  --monitor-interval-sec 15
```

Phase 3 (soak + chaos + compliance probes):

```bash
npm run launch:ops:phase3 -- \
  --staging-api-url https://neuraminds-api-base-staging-v1.onrender.com \
  --staging-web-url https://neuraminds-web-base-staging-v4.onrender.com \
  --soak-samples 96 \
  --soak-interval-sec 900
```

Optional matcher pause/resume chaos drill requires `ADMIN_CONTROL_KEY`.

Phase 4 (mainnet preflight):

```bash
npm run launch:ops:phase4 -- \
  --production-api-url https://api.neuraminds.ai \
  --production-web-url https://app.neuraminds.ai
```

Run all phases in order:

```bash
npm run launch:ops:all -- \
  --staging-api-url https://neuraminds-api-base-staging-v1.onrender.com \
  --staging-web-url https://neuraminds-web-base-staging-v4.onrender.com \
  --production-api-url https://api.neuraminds.ai \
  --production-web-url https://app.neuraminds.ai
```

## Evidence Artifacts

Each run writes timestamped outputs under `docs/reports/`:

- `synthetic-monitor-staging-live-<timestamp>.json|md`
- `launch-freeze-staging-<timestamp>.json|md`
- `launch-ops-phase2-workers-<timestamp>.log`
- `launch-ops-phase2-monitor-<timestamp>.json|md`
- `launch-ops-phase3-soak-<timestamp>.json|md`
- `launch-ops-phase3-compliance-<timestamp>.json`
- `launch-ops-phase4-preflight-<timestamp>.json|md`
- `launch-ops-phase4-cutover-<timestamp>.json`

## Freeze Guard

After phase 1 freeze is active, enforce low-churn policy:

```bash
npm run launch:freeze:guard -- --freeze-report docs/reports/launch-freeze-staging-<timestamp>.json
```

Allowed churn during freeze:

- `docs/reports/**`
- `docs/runbooks/**`
- `docs/LAUNCH_COMMAND_CENTER.md`
- `scripts/launch-ops-*`
- `scripts/launch-freeze-guard.mjs`

## Launch SLO Thresholds

- No persistent matcher backlog beyond 60 seconds.
- No eligible payout stuck beyond 600 seconds.
- Indexer lag stays below 20 blocks.
- Web4 runtime never enters `unhealthy`.

## Production Cutover Order

1. Verify strict production readiness and synthetic pass.
2. Confirm wallet funding runway with `npm run base:wallets:status`.
3. Start workers in order: indexer, matcher, payout, XMTP bridge, MCP server.
4. Run synthetic monitoring every 15 minutes for first 2 hours.
5. Trigger rollback only when thresholds breach.

## Rollback Trigger Thresholds

- API 5xx exceeds 2% over a 5-minute window.
- Matcher backlog exceeds 60 seconds continuously.
- Eligible payout remains unclaimed over 10 minutes.
- Indexer lag exceeds 20 blocks for sustained window.
