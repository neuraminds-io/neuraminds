# Limitless + Polymarket Integration Runbook

## Scope
- Unified market discovery across:
  - Internal Base markets
  - Limitless markets
  - Polymarket markets
- External execution for users and agents through BYOK credentials.
- Binary `YES/NO` execution scope for launch.

## Feature Flags
- `EXTERNAL_MARKETS_ENABLED`
- `EXTERNAL_TRADING_ENABLED`
- `EXTERNAL_AGENTS_ENABLED`
- `LIMITLESS_ENABLED`
- `POLYMARKET_ENABLED`

Recommended rollout order:
1. `EXTERNAL_MARKETS_ENABLED=true`, trading/agents disabled.
2. Enable trading for staging wallets only.
3. Enable external agents with low cadence and tight limits.
4. Promote provider-by-provider.

## Required Environment
- `EXTERNAL_CREDENTIALS_MASTER_KEY`
- `EXTERNAL_CREDENTIALS_KEY_ID`
- `LIMITLESS_API_BASE`
- `POLYMARKET_GAMMA_API_BASE`
- `POLYMARKET_CLOB_API_BASE`
- `POLYGON_RPC_URL`

## Data Model
Migration: `migrations/008_external_integrations.sql`

Tables:
- `external_credentials`
- `external_order_intents`
- `external_orders`
- `external_agents`
- `external_agent_runs`
- `external_market_cache`

## API Surface
### Unified market reads
- `GET /v1/evm/markets?source=all|internal|limitless|polymarket&tradable=all|user|agent`
- `GET /v1/evm/markets/{market_id}`
- `GET /v1/evm/markets/{market_id}/orderbook`
- `GET /v1/evm/markets/{market_id}/trades`

`market_id` accepts:
- Internal numeric IDs (example: `12`)
- Namespaced external IDs (example: `limitless:<slug>`, `polymarket:<id>`)

### External credentials + orders + agents
- `GET /v1/external/credentials`
- `POST /v1/external/credentials`
- `DELETE /v1/external/credentials/{credential_id}`
- `POST /v1/external/orders/intent`
- `POST /v1/external/orders/submit`
- `POST /v1/external/orders/cancel`
- `GET /v1/external/orders`
- `GET /v1/external/agents`
- `POST /v1/external/agents`
- `PATCH /v1/external/agents/{agent_id}`
- `POST /v1/external/agents/{agent_id}/execute`

## MCP/Web4
Updated tools:
- `getMarkets` now supports `source` and `tradable`.
- `getOrderBook` and `getTrades` accept string `market_id`.
- Added:
  - `prepareExternalOrder`
  - `submitExternalOrder`
  - `cancelExternalOrder`
  - `listExternalAgents`
  - `executeExternalAgent`

## Monitoring Gates
Synthetic monitor must pass:
- `source=limitless` markets load.
- `source=polymarket` markets load.
- External sample orderbook read.
- Authenticated external intent creation (when auth token is supplied).

Script:
- `node scripts/synthetic-monitor.mjs --env <env> --api-url <url> --chain-mode base --min-external-markets 1`

Optional authenticated check:
- add `--auth-bearer <jwt>`

## Degraded Mode Operations
- If a provider degrades:
  - Set provider flag false (`LIMITLESS_ENABLED` or `POLYMARKET_ENABLED`)
  - Keep `EXTERNAL_MARKETS_ENABLED=true` if at least one provider remains.
  - Keep internal Base markets active.
- If external execution degrades:
  - Set `EXTERNAL_TRADING_ENABLED=false`
  - Set `EXTERNAL_AGENTS_ENABLED=false`
  - Keep read-only external aggregation enabled.

## Launch Checklist
- Migration `008_external_integrations.sql` applied.
- External env values configured and validated.
- Backend compile check passes.
- Frontend build passes.
- `GET /v1/evm/markets?source=all` returns internal + external entries.
- External order intent and submit validated with real staging credentials.
- External agent executes one successful cycle on each enabled provider.
