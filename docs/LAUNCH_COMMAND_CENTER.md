# Launch Command Center

## Canonical Readiness Commands

```bash
npm run launch:config
npm run production:gates
npm run launch:readiness
npm run launch:summary
```

Strict mode (includes frontend build + backend cargo check through production gates strict):

```bash
npm run launch:readiness:strict
```

Optional local timeout override for strict checks:

```bash
PRODUCTION_GATE_TIMEOUT_MS=60000 npm run launch:readiness:strict
```

## Local End-to-End Launch Rehearsal

```bash
npm run launch:e2e
```

Generated artifacts:
- `docs/reports/launch-config-report.json`
- `docs/reports/production-loop-report.json`
- `docs/reports/production-loop-report-fast.json`
- `docs/reports/production-loop-report-strict.json`
- `docs/reports/launch-go-no-go.json`
- `docs/reports/launch-go-no-go.md`

## Go / No-Go Rules

Go only if all conditions are true:
1. `launch-config-report.json` has `"ready": true` (or only expected missing secrets during dry rehearsal).
2. `production-loop-report.json` has `"ready": true`.
3. `launch-go-no-go.json` has `"go": true`.
4. CI workflow `Launch Readiness` is green on latest commit.
5. Deployment target environment secrets are present and rotated (see `docs/LAUNCH_ENV_CHECKLIST.md`).

No-Go if any of:
1. Required gate fails.
2. Auth/CORS origin validation fails.
3. Backend check/build fails.
4. Health checks fail post-deploy.

## Cutover Sequence
1. Freeze deploy window and announce change window.
2. Run `npm run launch:readiness:strict` on release commit.
3. Trigger deployment workflow (staging first, then production).
4. Verify `/health` and `/health/detailed`.
5. Verify market listing + auth refresh path + order placement sanity check.
6. Monitor error rate, p95 latency, and Redis/DB health for 30 minutes.

## Rollback Trigger
Rollback immediately on:
1. Sustained 5xx spike (>2% for 5 minutes),
2. auth refresh failure rate spike,
3. failed health checks after retries,
4. critical settlement/order mismatch alerts.
