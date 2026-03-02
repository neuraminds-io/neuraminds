# OpenClaw E2E Readiness Report

Generated: 2026-02-28T22:26:04.521Z
API: https://neuraminds-api-base-staging-v1.onrender.com
Mode: both
Require full Web4: false
Min markets: 1
Min agents: 0
Decision: FAIL

| Check | Required | Status | Latency | Target | Details |
|---|---|---|---:|---|---|
| runtime_health | yes | PASS | 432ms | https://neuraminds-api-base-staging-v1.onrender.com/v1/web4/runtime/health | status=healthy mcp=true x402=true xmtp=true fullWeb4Ready=false |
| seeded_markets | yes | FAIL | 310ms | https://neuraminds-api-base-staging-v1.onrender.com/v1/evm/markets?limit=1 | markets=0 required>=1 |
| direct_initialize | yes | PASS | 223ms | https://neuraminds-api-base-staging-v1.onrender.com/v1/web4/mcp | http=200 server=neuraminds-mcp |
| direct_tools_list | yes | PASS | 196ms | https://neuraminds-api-base-staging-v1.onrender.com/v1/web4/mcp | tools=13 hasGetMarkets=true |
| direct_resources_list | yes | FAIL | 821ms | https://neuraminds-api-base-staging-v1.onrender.com/v1/web4/mcp | resources=4 hasRuntime=false |
| direct_resource_read_runtime | yes | FAIL | 261ms | https://neuraminds-api-base-staging-v1.onrender.com/v1/web4/mcp | contents=0 |
| direct_prompts_list | yes | PASS | 240ms | https://neuraminds-api-base-staging-v1.onrender.com/v1/web4/mcp | prompts=3 |
| direct_prompt_get_market_scan | yes | FAIL | 228ms | https://neuraminds-api-base-staging-v1.onrender.com/v1/web4/mcp | messages=0 |
| direct_tool_get_markets | yes | FAIL | 500ms | https://neuraminds-api-base-staging-v1.onrender.com/v1/web4/mcp | markets=0 required>=1 |
| stdio_connect | yes | PASS | 137ms | node scripts/mcp-server.mjs | connected pid=64980 |
| stdio_tools_list | yes | PASS | 3ms | listTools | tools=9 hasGetMarkets=true |
| stdio_resources_list | yes | FAIL | 0ms | listResources | resources=3 hasRuntime=false |
| stdio_runtime | yes | FAIL | 141ms | node scripts/mcp-server.mjs | MCP error -32602: MCP error -32602: Resource neuraminds://runtime/health not found |

## Failed Required Checks
- seeded_markets
- direct_resources_list
- direct_resource_read_runtime
- direct_prompt_get_market_scan
- direct_tool_get_markets
- stdio_resources_list
- stdio_runtime

