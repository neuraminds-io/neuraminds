# Load Baseline

## Goal

Define a repeatable capacity baseline with explicit latency and throughput targets for launch and post-launch validation.

## Baseline Targets

| Metric | Target |
| --- | --- |
| Sustained throughput | `>= 80` requests/sec |
| `http_req_duration` p95 | `<= 500ms` |
| `http_req_duration` p99 | `<= 900ms` |
| HTTP error rate | `<= 1%` |

## Execute Baseline Test

Run the public baseline scenario:

```bash
API_URL=https://api.neuraminds.ai \
TARGET_QPS=80 \
DURATION=10m \
k6 run tests/load/public-baseline.js
```

Optional staging run:

```bash
API_URL="$SYNTHETIC_STAGING_API_URL" \
TARGET_QPS=60 \
DURATION=10m \
k6 run tests/load/public-baseline.js
```

This writes `tests/load/public-baseline-summary.json`.

## Generate Launch Report

```bash
npm run load:baseline:report -- \
  --input tests/load/public-baseline-summary.json \
  --env production \
  --target-qps 80 \
  --p95-ms 500 \
  --p99-ms 900 \
  --max-error-rate 0.01
```

Outputs:
- `docs/reports/load-baseline-production.json`
- `docs/reports/load-baseline-production.md`

The report command exits non-zero if any target is missed.

## Launch Gate Usage

Use this baseline as a launch gate during rehearsal and on release commit:
1. Run k6 baseline against target environment.
2. Generate report with `load:baseline:report`.
3. Require `Decision: PASS` before launch approval.
