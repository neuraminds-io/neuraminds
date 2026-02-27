# NeuraMinds Agent Interface

## Project Identity
- Name: `neuraminds`
- Mode: Base-native Web4 agent market network
- Core contracts:
  - `MarketCore`
  - `OrderBook`
  - `CollateralVault`
  - `AgentRuntime`

## Chain + Contracts
- Primary chain: Base mainnet (`8453`)
- Test chain: Base Sepolia (`84532`)
- Runtime addresses are environment-driven:
  - `MARKET_CORE_ADDRESS`
  - `ORDER_BOOK_ADDRESS`
  - `COLLATERAL_VAULT_ADDRESS`
  - `AGENT_RUNTIME_ADDRESS`
- Frontend mirrors:
  - `NEXT_PUBLIC_MARKET_CORE_ADDRESS`
  - `NEXT_PUBLIC_ORDER_BOOK_ADDRESS`
  - `NEXT_PUBLIC_COLLATERAL_VAULT_ADDRESS`
  - `NEXT_PUBLIC_COLLATERAL_TOKEN_ADDRESS`

## Read API (Base)
Base URL: `${NEXT_PUBLIC_API_URL}/v1`

- `GET /evm/markets?limit=&offset=`
- `GET /evm/markets/{market_id}/orderbook?outcome=yes|no&depth=`
- `GET /evm/markets/{market_id}/trades?outcome=yes|no&limit=&offset=`
- `GET /evm/agents?limit=&offset=&owner=&market_id=&active=`
- `GET /evm/agents/{agent_id}`
- `GET /evm/token/state`
- `GET /web4/capabilities`
- `GET /web4/mcp`
- `GET /web4/agent-card`

## Prepared Write API (Base)
These endpoints return `{ chain_id, to, data, value, method }` for wallet signing/broadcast.

- `POST /evm/write/markets/create`
- `POST /evm/write/orders/place`
- `POST /evm/write/orders/cancel`
- `POST /evm/write/orders/match`
- `POST /evm/write/positions/claim`
- `POST /evm/write/agents/create`
- `POST /evm/write/agents/execute`
- `POST /evm/write/relay` (raw signed tx relay)

## Agent Runtime Model
- `createAgent(marketId, isYes, priceBps, size, cadence, expiryWindow, strategy)`
- `executeAgent(agentId)` can be called by any executor.
- Agent directory status model:
  - `ready`: active and executable now
  - `cooldown`: active, waiting for cadence window
  - `inactive`: deactivated

## Auth
- User auth: SIWE + JWT
- API auth routes:
  - `GET /auth/siwe/nonce`
  - `POST /auth/siwe/login`
  - `POST /auth/refresh`
  - `POST /auth/logout`

## Frontend Agent Surface
- Route: `/agents`
- Features:
  - Agent directory (filter by market/active)
  - Launch onchain agent
  - Execute ready agent

## Operational Notes
- Strict launch readiness requires DX snapshot capture by default.
- MCP process runtime:
  - `npm run mcp:server`
- Mainnet smoke command for full create/trade/match/resolve/claim loop:
  - `npm run base:smoke:mainnet`
  - `npm run base:smoke:mainnet:dry`
- Autonomous agent execution runtime:
  - `npm run agents:executor`
  - `npm run agents:executor:sepolia`
  - `npm run agents:executor:dry`
- XMTP bridge runtime:
  - `npm run xmtp:bridge`
