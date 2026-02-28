# Synthetic Monitor

Generated: 2026-02-28T01:23:06.035Z
Environment: staging
Chain mode: base

Ready: YES

## Checks

| Check | Status | Latency | Details | URL |
| --- | --- | --- | --- | --- |
| api_health | PASS | 344ms | status=healthy | https://neuraminds-api-base-staging-v1.onrender.com/health |
| api_health_detailed | PASS | 385ms | http=200 db=healthy redis=healthy base=healthy solana=healthy | https://neuraminds-api-base-staging-v1.onrender.com/health/detailed |
| api_evm_markets_public | PASS | 702ms | marketCount=0 | https://neuraminds-api-base-staging-v1.onrender.com/v1/evm/markets?limit=1 |
| web_health | PASS | 312ms | http=200 | https://neuraminds-web-base-staging-v4.onrender.com |

