# OpenClaw E2E Integration Pack

Generated: 2026-02-28T22:25:51.768Z
Environment: production
Network: Base (8453)

## Endpoints
- API: https://neuraminds-api-base-prod-v1.onrender.com
- Web: https://neuraminds-web-base-prod-v1.onrender.com
- MCP JSON-RPC: https://neuraminds-api-base-prod-v1.onrender.com/v1/web4/mcp
- MCP manifest: https://neuraminds-api-base-prod-v1.onrender.com/v1/web4/mcp

## Canonical Addresses
- MarketCore: 0x4d66ab11e147f6d8c395b36d6b0da867eb1f7a13
- OrderBook: 0x391050b6a7e3be5b699772880fe6d3d249d49b7b
- CollateralVault: 0x139290d00aebe1df9f5d40b796c0d30dea1e93a0
- AgentRuntime: 0xbf5d884648bd799092f5b1663c950a85dee1e825
- Collateral token (USDC): 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913

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

