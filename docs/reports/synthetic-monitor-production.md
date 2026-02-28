# Synthetic Monitor

Generated: 2026-02-28T01:20:34.838Z
Environment: production
Chain mode: base

Ready: YES

## Checks

| Check | Status | Latency | Details | URL |
| --- | --- | --- | --- | --- |
| api_health | PASS | 694ms | status=healthy | https://neuraminds-api-base-staging-v1.onrender.com/health |
| api_health_detailed | PASS | 335ms | http=200 db=healthy redis=healthy base=healthy solana=healthy | https://neuraminds-api-base-staging-v1.onrender.com/health/detailed |
| api_evm_markets_public | PASS | 274ms | marketCount=0 | https://neuraminds-api-base-staging-v1.onrender.com/v1/evm/markets?limit=1 |
| web_health | PASS | 297ms | http=200 | https://neuraminds-web-base-staging-v4.onrender.com |

