# Launch Environment Checklist

## Backend Required (Production, Common)
- `DATABASE_URL`
- `REDIS_URL`
- `JWT_SECRET` (>= 32 chars)
- `CORS_ORIGINS` (no wildcard, https origins only)
- `BLINDFOLD_WEBHOOK_SECRET`
- `CHAIN_MODE` (`base`, `solana`, or `dual`)
- `GEO_BLOCKING_ENABLED=true` (required for compliance write restrictions)
- `GEO_TEST_OVERRIDE_KEY` (staging-only, required for deterministic launch compliance probes)

## Frontend/Edge Required (Production, Common)
- `NEXT_PUBLIC_API_URL` (https)
- `AUTH_ALLOWED_ORIGINS` (no wildcard, https origins only)
- `NEXT_PUBLIC_CHAIN_MODE` (`base`, `solana`, or `dual`)
- `SYNTHETIC_PROD_API_URL` (https, for launch synthetic monitor)
- `SYNTHETIC_PROD_WEB_URL` (https, for launch synthetic monitor)
- `SYNTHETIC_STAGING_API_URL` (https, required for staging synthetic checks)
- `SYNTHETIC_STAGING_WEB_URL` (https, required for staging synthetic checks)

## Base Mode Required (`CHAIN_MODE=base` or `dual`)

Backend:
- `EVM_ENABLED=true`
- `EVM_READS_ENABLED=true`
- `EVM_WRITES_ENABLED=true` (or explicitly false for read-only)
- `BASE_RPC_URL` (https)
- `BASE_WS_URL` (wss)
- `BASE_CHAIN_ID` (`8453` mainnet)
- `SIWE_DOMAIN`
- `MARKET_CORE_ADDRESS`
- `ORDER_BOOK_ADDRESS`
- `COLLATERAL_VAULT_ADDRESS`
- `AGENT_RUNTIME_ADDRESS`
- `ERC8004_IDENTITY_REGISTRY_ADDRESS`
- `ERC8004_REPUTATION_REGISTRY_ADDRESS`
- `BASE_AGENT_RUNTIME_OPERATOR_PRIVATE_KEY` (required when writes enabled)
- `BASE_GLOBAL_CLAIMER_PRIVATE_KEY` (required for global payout worker, can reuse runtime operator key)
- `AUTO_CLAIMER_PRIVATE_KEY` (optional explicit claimer key override)
- `AUTO_CLAIMER_POLL_INTERVAL_SEC` (default `30`)
- `AUTO_CLAIMER_CANDIDATE_SCAN_LIMIT` (default `1000`)
- `AUTO_CLAIMER_AGENT_SCAN_LIMIT` (default `1000`)
- `AUTO_CLAIMER_ORDER_SCAN_WINDOW` (default `2500`)
- `AUTO_CLAIMER_MAX_CLAIMS_PER_CYCLE` (default `200`, `0` = unlimited)
- `AUTO_CLAIMER_AUTO_WITHDRAW` (`true` or `false`)
- `AUTO_CLAIMER_OWNER_KEYS_FILE` or `AUTO_CLAIMER_OWNER_KEYS_JSON` (required only when auto-withdraw is enabled)
- `BASE_MATCHER_PRIVATE_KEY` (required for matcher worker, can reuse operator key)
- `MATCHER_ENABLED=true`
- `MATCHER_MAX_FILL_SIZE` (default `1000000`)
- `MATCHER_RATE_LIMIT_PER_MARKET` (default `1`)
- `MATCHER_MAX_MARKETS_PER_CYCLE` (default `100`)
- `ADMIN_CONTROL_KEY` (required for matcher/payout/indexer admin control endpoints)
- `INDEXER_LOOKBACK_BLOCKS` (default `25000`)
- `INDEXER_CONFIRMATIONS` (default `8`)
- `X402_ENABLED=true`
- `X402_SIGNING_KEY`
- `X402_RECEIVER_ADDRESS`
- `XMTP_SWARM_ENABLED=true`
- `XMTP_SWARM_SIGNING_KEY`
- `XMTP_SWARM_TRANSPORT=xmtp_http`
- `XMTP_SWARM_BRIDGE_URL`

Frontend:
- `NEXT_PUBLIC_BASE_RPC_URL` (https)
- `NEXT_PUBLIC_BASE_CHAIN_ID` (`8453` mainnet)
- `NEXT_PUBLIC_SIWE_DOMAIN`
- `NEXT_PUBLIC_MARKET_CORE_ADDRESS`
- `NEXT_PUBLIC_ORDER_BOOK_ADDRESS`
- `NEXT_PUBLIC_AGENT_RUNTIME_ADDRESS`
- `NEXT_PUBLIC_COLLATERAL_TOKEN_ADDRESS`
- `NEXT_PUBLIC_COLLATERAL_VAULT_ADDRESS`

## External Venue Integration (Limitless + Polymarket)

Backend:
- `EXTERNAL_MARKETS_ENABLED`
- `EXTERNAL_TRADING_ENABLED`
- `EXTERNAL_AGENTS_ENABLED`
- `LIMITLESS_ENABLED`
- `POLYMARKET_ENABLED`
- `EXTERNAL_CREDENTIALS_MASTER_KEY` (required when trading or external agents enabled)
- `EXTERNAL_CREDENTIALS_KEY_ID` (recommended non-empty for key rotation)
- `LIMITLESS_API_BASE` (https)
- `POLYMARKET_GAMMA_API_BASE` (https)
- `POLYMARKET_CLOB_API_BASE` (https)
- `POLYGON_RPC_URL` (https)

Operational checks:
- `GET /v1/evm/markets?source=limitless`
- `GET /v1/evm/markets?source=polymarket`
- `GET /v1/evm/markets/{namespaced_id}/orderbook`
- Authenticated `POST /v1/external/orders/intent` dry-run with staging credential

## Solana Mode Required (`CHAIN_MODE=solana` or `dual`)

Backend:
- `SOLANA_ENABLED=true`
- `SOLANA_READS_ENABLED=true`
- `SOLANA_RPC_URL` (https)
- `SOLANA_WS_URL` (wss)
- `SOLANA_MARKET_PROGRAM_ID` (base58)
- `SOLANA_ORDERBOOK_PROGRAM_ID` (base58)
- `SOLANA_WRITES_ENABLED=true` for writable mode
- `SOLANA_PRIVACY_PROGRAM_ID` (base58, required when writes enabled)
- Verify `/v1/solana/write/relay` with a signed base64 transaction from your Solana signer flow

Frontend:
- `NEXT_PUBLIC_SOLANA_RPC_URL` (https)
- `NEXT_PUBLIC_SOLANA_MARKET_PROGRAM_ID` (base58)
- `NEXT_PUBLIC_SOLANA_ORDERBOOK_PROGRAM_ID` (base58)

## Verification

Strict config validation:

```bash
npm run launch:config
```

Dev/staging strict validation (allows placeholders):

```bash
npm run launch:config:dev-strict
```

Canonical address drift validation:

```bash
npm run launch:addresses
```

Solana program smoke:

```bash
npm run solana:smoke
```

Global Base payout worker (separate process):

```bash
npm run payouts:worker
```

Global Base payout worker with managed auto-withdraw:

```bash
npm run payouts:worker:auto-withdraw
```

Global Base matcher worker:

```bash
npm run matcher:worker
```
