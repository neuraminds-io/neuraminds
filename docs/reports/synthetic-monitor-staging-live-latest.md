# Synthetic Monitor

Generated: 2026-02-28T22:47:49.943Z
Environment: staging-live
Chain mode: base
Require full Web4: false
Min EVM markets: 1
Min EVM agents: 0

Ready: NO

## Checks

| Check | Status | Latency | Details | URL |
| --- | --- | --- | --- | --- |
| api_health | PASS | 306ms | status=healthy | https://neuraminds-api-base-staging-v1.onrender.com/health |
| api_health_detailed | PASS | 693ms | http=200 db=healthy redis=healthy base=healthy solana=healthy | https://neuraminds-api-base-staging-v1.onrender.com/health/detailed |
| api_evm_markets_public | FAIL | 233ms | markets=0 required>=1 | https://neuraminds-api-base-staging-v1.onrender.com/v1/evm/markets?limit=1 |
| web4_runtime_health | PASS | 221ms | status=degraded mcp=true x402=false xmtp=false fullWeb4Ready=false | https://neuraminds-api-base-staging-v1.onrender.com/v1/web4/runtime/health |
| web4_mcp_ping | PASS | 315ms | mcp ping ok=true | https://neuraminds-api-base-staging-v1.onrender.com/v1/web4/mcp |
| web_health | PASS | 286ms | http=200 | https://neuraminds-web-base-staging-v4.onrender.com |

