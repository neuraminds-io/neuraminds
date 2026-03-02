# Synthetic Monitor

Generated: 2026-02-28T22:48:36.318Z
Environment: production-live-no-market-gate
Chain mode: base
Require full Web4: true
Min EVM markets: 0
Min EVM agents: 0

Ready: YES

## Checks

| Check | Status | Latency | Details | URL |
| --- | --- | --- | --- | --- |
| api_health | PASS | 301ms | status=healthy | https://neuraminds-api-base-prod-v1.onrender.com/health |
| api_health_detailed | PASS | 339ms | http=200 db=healthy redis=healthy base=healthy solana=healthy | https://neuraminds-api-base-prod-v1.onrender.com/health/detailed |
| api_evm_markets_public | PASS | 231ms | markets=0 required>=0 | https://neuraminds-api-base-prod-v1.onrender.com/v1/evm/markets?limit=1 |
| web4_runtime_health | PASS | 223ms | status=healthy mcp=true x402=true xmtp=true fullWeb4Ready=true | https://neuraminds-api-base-prod-v1.onrender.com/v1/web4/runtime/health |
| web4_mcp_ping | PASS | 227ms | mcp ping ok=true | https://neuraminds-api-base-prod-v1.onrender.com/v1/web4/mcp |
| x402_quote | PASS | 231ms | receiver=0x39e4939dF3763e342DB531a2A58867bC26a22B98 amount=5000 | https://neuraminds-api-base-prod-v1.onrender.com/v1/payments/x402/quote?resource=mcp_tool_call |
| xmtp_health | PASS | 226ms | enabled=true transport=redis bridgeConfigured=false | https://neuraminds-api-base-prod-v1.onrender.com/v1/web4/xmtp/health |
| web_health | PASS | 275ms | http=200 | https://neuraminds-web-base-prod-v1.onrender.com |

