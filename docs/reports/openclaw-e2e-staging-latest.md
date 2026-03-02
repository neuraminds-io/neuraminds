# OpenClaw E2E Readiness Report

Generated: 2026-02-28T22:42:20.542Z
API: https://neuraminds-api-base-staging-v1.onrender.com
Mode: both
Require full Web4: false
Min markets: 1
Min agents: 0
Decision: FAIL

| Check | Required | Status | Latency | Target | Details |
|---|---|---|---:|---|---|
| runtime_health | yes | PASS | 341ms | https://neuraminds-api-base-staging-v1.onrender.com/v1/web4/runtime/health | status=degraded mcp=true x402=false xmtp=false fullWeb4Ready=false |
| seeded_markets | yes | FAIL | 233ms | https://neuraminds-api-base-staging-v1.onrender.com/v1/evm/markets?limit=1 | markets=0 required>=1 |
| direct_initialize | yes | PASS | 234ms | https://neuraminds-api-base-staging-v1.onrender.com/v1/web4/mcp | http=200 server=neuraminds-mcp |
| direct_tools_list | yes | PASS | 606ms | https://neuraminds-api-base-staging-v1.onrender.com/v1/web4/mcp | tools=13 hasGetMarkets=true |
| direct_resources_list | yes | PASS | 220ms | https://neuraminds-api-base-staging-v1.onrender.com/v1/web4/mcp | resources=5 hasRuntime=true |
| direct_resource_read_runtime | yes | PASS | 295ms | https://neuraminds-api-base-staging-v1.onrender.com/v1/web4/mcp | contents=1 |
| direct_prompts_list | yes | PASS | 224ms | https://neuraminds-api-base-staging-v1.onrender.com/v1/web4/mcp | prompts=4 |
| direct_prompt_get_market_scan | yes | PASS | 232ms | https://neuraminds-api-base-staging-v1.onrender.com/v1/web4/mcp | messages=1 |
| direct_tool_get_markets | yes | FAIL | 360ms | https://neuraminds-api-base-staging-v1.onrender.com/v1/web4/mcp | markets=0 required>=1 |
| stdio_connect | yes | PASS | 108ms | node scripts/mcp-server.mjs | connected pid=68905 |
| stdio_tools_list | yes | PASS | 3ms | listTools | tools=9 hasGetMarkets=true |
| stdio_resources_list | yes | PASS | 0ms | listResources | resources=4 hasRuntime=true |
| stdio_resource_read_runtime | yes | PASS | 601ms | readResource | contents=1 |
| stdio_prompts_list | yes | PASS | 2ms | listPrompts | prompts=4 |
| stdio_prompt_get_market_scan | yes | PASS | 229ms | getPrompt | messages=1 |
| stdio_tool_get_markets | yes | FAIL | 293ms | callTool:getMarkets | markets=0 required>=1 |

## Failed Required Checks
- seeded_markets
- direct_tool_get_markets
- stdio_tool_get_markets

