# Production Loop Gates

`neuralminds` now includes a deterministic production gate report.

## Commands

```bash
npm run production:gates
npm run production:gates:strict
npm run launch:readiness
npm run launch:readiness:strict
```

## Fast Mode Gates

- `legacy_brand_refs_zero` (required)
- `legacy_palette_refs_zero` (required)
- `auth_route_hardening_enabled` (required)
- `backend_auth_rate_limit_present` (required)

Fast mode skips expensive build/compile checks.

## Strict Mode Gates

Strict mode includes all fast gates plus:

- `web_build` (required)
- `backend_cargo_check` (required)

You can override strict command timeout per step (default `480000` ms):

```bash
PRODUCTION_GATE_TIMEOUT_MS=60000 npm run production:gates:strict
```

## Report Output

Reports are written to:

- `docs/reports/production-loop-report.json`
- `docs/reports/production-loop-report-fast.json`
- `docs/reports/production-loop-report-strict.json`

`ready: true` means all required gates passed.
