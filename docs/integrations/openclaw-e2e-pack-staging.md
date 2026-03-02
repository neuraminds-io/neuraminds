# OpenClaw E2E Integration Pack

Generated: 2026-02-28T22:25:51.624Z
Environment: staging
Network: Base (84532)

## Endpoints
- API: https://neuraminds-api-base-staging-v1.onrender.com
- Web: https://neuraminds-web-base-staging-v4.onrender.com
- MCP JSON-RPC: https://neuraminds-api-base-staging-v1.onrender.com/v1/web4/mcp
- MCP manifest: https://neuraminds-api-base-staging-v1.onrender.com/v1/web4/mcp

## Canonical Addresses
- MarketCore: 0x823d54726ddc48a784ee6eb53235f5d68c94f1c0
- OrderBook: 0xc6ec840da50fc708bf155b5b15a585c5d0004ebf
- CollateralVault: 0x588e19e5831ddc8aed5c8e0d687f86884ff98ee2
- AgentRuntime: unset
- Collateral token (USDC): 0x036CbD53842c5426634e7929541eC2318f3dCF7e

## Transport Profiles
- Direct HTTP: config/openclaw/neuraminds-mcp.direct-http.json
- Stdio bridge: config/openclaw/neuraminds-mcp.stdio.json

## Required Headers
| Header | Required | Purpose |
|---|---|---|
| content-type: application/json | yes | JSON-RPC and API POST requests |
| x-payment | no | x402 payment proof for premium reads/tool calls |
| x-client-id | no | stable client identity for MCP policy/rate controls |

## x402 Flow
1. Request quote from `GET /v1/payments/x402/quote?resource=<resource>`
2. Submit onchain Base USDC transfer to quote.receiver
3. Pass proof in `x-payment` header or MCP `arguments.payment` object
4. Retry tool call with proof before quote expiry

## Acceptance
- Run `npm run openclaw:e2e -- --mode both --api-url <api> --require-full-web4`
- Require pass for direct HTTP MCP and stdio MCP transport checks

