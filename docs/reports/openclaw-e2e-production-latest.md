# OpenClaw E2E Readiness Report

Generated: 2026-02-28T22:46:58.254Z
API: https://neuraminds-api-base-prod-v1.onrender.com
Mode: both
Require full Web4: true
Min markets: 1
Min agents: 0
Decision: FAIL

| Check | Required | Status | Latency | Target | Details |
|---|---|---|---:|---|---|
| runtime_health | yes | PASS | 349ms | https://neuraminds-api-base-prod-v1.onrender.com/v1/web4/runtime/health | status=healthy mcp=true x402=true xmtp=true fullWeb4Ready=true |
| seeded_markets | yes | FAIL | 221ms | https://neuraminds-api-base-prod-v1.onrender.com/v1/evm/markets?limit=1 | markets=0 required>=1 |
| x402_quote | yes | PASS | 258ms | https://neuraminds-api-base-prod-v1.onrender.com/v1/payments/x402/quote?resource=mcp_tool_call | receiver=0x39e4939dF3763e342DB531a2A58867bC26a22B98 amount=5000 |
| xmtp_health | yes | PASS | 219ms | https://neuraminds-api-base-prod-v1.onrender.com/v1/web4/xmtp/health | enabled=true transport=redis bridgeConfigured=false |
| direct_initialize | yes | PASS | 228ms | https://neuraminds-api-base-prod-v1.onrender.com/v1/web4/mcp | http=200 server=neuraminds-mcp |
| direct_tools_list | yes | PASS | 223ms | https://neuraminds-api-base-prod-v1.onrender.com/v1/web4/mcp | tools=13 hasGetMarkets=true |
| direct_resources_list | yes | PASS | 224ms | https://neuraminds-api-base-prod-v1.onrender.com/v1/web4/mcp | resources=5 hasRuntime=true |
| direct_resource_read_runtime | yes | PASS | 263ms | https://neuraminds-api-base-prod-v1.onrender.com/v1/web4/mcp | contents=1 |
| direct_prompts_list | yes | PASS | 227ms | https://neuraminds-api-base-prod-v1.onrender.com/v1/web4/mcp | prompts=4 |
| direct_prompt_get_market_scan | yes | PASS | 224ms | https://neuraminds-api-base-prod-v1.onrender.com/v1/web4/mcp | messages=1 |
| direct_tool_get_markets | yes | FAIL | 296ms | https://neuraminds-api-base-prod-v1.onrender.com/v1/web4/mcp | markets=0 required>=1 |
| stdio_connect | yes | PASS | 106ms | node scripts/mcp-server.mjs | connected pid=69886 |
| stdio_tools_list | yes | PASS | 4ms | listTools | tools=9 hasGetMarkets=true |
| stdio_resources_list | yes | PASS | 1ms | listResources | resources=4 hasRuntime=true |
| stdio_resource_read_runtime | yes | PASS | 400ms | readResource | contents=1 |
| stdio_prompts_list | yes | PASS | 2ms | listPrompts | prompts=4 |
| stdio_prompt_get_market_scan | yes | PASS | 225ms | getPrompt | messages=1 |
| stdio_tool_get_markets | yes | FAIL | 282ms | callTool:getMarkets | markets=0 required>=1 |

## Failed Required Checks
- seeded_markets
- direct_tool_get_markets
- stdio_tool_get_markets

