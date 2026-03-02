# Synthetic Monitoring

## Scope

Synthetic probes run from outside the stack and validate:
- API liveness (`/health`)
- Component health (`/health/detailed`)
- Public market read path (`/v1/markets?limit=1`)
- Web app shell availability (`/`)

The probes produce machine-readable reports in `docs/reports/` for launch and post-launch audits.

## Probe Runner

Single environment check:

```bash
node scripts/synthetic-monitor.mjs \
  --env production \
  --api-url https://api.neuraminds.ai \
  --web-url https://app.neuraminds.ai
```

Staging check:

```bash
node scripts/synthetic-monitor.mjs \
  --env staging \
  --api-url "$SYNTHETIC_STAGING_API_URL" \
  --web-url "$SYNTHETIC_STAGING_WEB_URL"
```

Outputs:
- `docs/reports/synthetic-monitor-<env>.json`
- `docs/reports/synthetic-monitor-<env>.md`

The command exits non-zero when any required probe fails.

## External Probe Cadence

- Production: every 5 minutes from at least 2 external regions.
- Staging: every 15 minutes from at least 1 external region.
- Keep one region in North America and one in Europe for production.

## Alerting Criteria

Trigger an alert immediately when any required probe fails:
- `api_health`
- `api_health_detailed`
- `api_markets_public`
- `web_home`

Routing and escalation rules are defined in `docs/ALERT_ROUTING_MATRIX.md`.

## CI Variables

Configure these repository variables for automated runs:
- `SYNTHETIC_PROD_API_URL`
- `SYNTHETIC_PROD_WEB_URL`
- `SYNTHETIC_STAGING_API_URL`
- `SYNTHETIC_STAGING_WEB_URL`
