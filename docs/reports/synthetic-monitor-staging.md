# Synthetic Monitor

Generated: 2026-02-28T01:49:17.596Z
Environment: staging
Chain mode: base

Ready: YES

## Checks

| Check | Status | Latency | Details | URL |
| --- | --- | --- | --- | --- |
| api_health | PASS | 374ms | status=healthy | https://neuraminds-api-base-staging-v1.onrender.com/health |
| api_health_detailed | PASS | 402ms | http=200 db=healthy redis=healthy base=healthy solana=healthy | https://neuraminds-api-base-staging-v1.onrender.com/health/detailed |
| api_evm_markets_public | PASS | 259ms | marketCount=0 | https://neuraminds-api-base-staging-v1.onrender.com/v1/evm/markets?limit=1 |
| web_health | PASS | 296ms | http=200 | https://neuraminds-web-base-staging-v4.onrender.com |

