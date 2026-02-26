# Launch Environment Checklist

## Backend Required (Production, Common)
- `DATABASE_URL`
- `REDIS_URL`
- `JWT_SECRET` (>= 32 chars)
- `CORS_ORIGINS` (no wildcard, https origins only)
- `METRICS_TOKEN`
- `BLINDFOLD_WEBHOOK_SECRET`

## Frontend/Edge Required (Production, Common)
- `NEXT_PUBLIC_API_URL` (https)
- `AUTH_ALLOWED_ORIGINS` (no wildcard, https origins only)
- `NEXT_PUBLIC_CHAIN_MODE` (`base` or `solana`)

## Base Mode Required

Backend:
- `CHAIN_MODE=base` (recommended)
- `EVM_ENABLED=true`
- `EVM_READS_ENABLED=true`
- `EVM_WRITES_ENABLED=true`
- `LEGACY_READS_ENABLED=false`
- `LEGACY_WRITES_ENABLED=false`
- `BASE_RPC_URL` (https)
- `BASE_WS_URL` (wss)
- `BASE_CHAIN_ID` (`8453` mainnet)
- `SIWE_DOMAIN`
- `MARKET_CORE_ADDRESS`
- `ORDER_BOOK_ADDRESS`
- `COLLATERAL_VAULT_ADDRESS`

Optional (enable after token launch):
- `NEURA_TOKEN_ADDRESS`

Frontend:
- `NEXT_PUBLIC_CHAIN_MODE=base`
- `NEXT_PUBLIC_BASE_RPC_URL` (https)
- `NEXT_PUBLIC_BASE_CHAIN_ID` (`8453` mainnet)
- `NEXT_PUBLIC_SIWE_DOMAIN`
- `NEXT_PUBLIC_MARKET_CORE_ADDRESS`
- `NEXT_PUBLIC_ORDER_BOOK_ADDRESS`
- `NEXT_PUBLIC_COLLATERAL_TOKEN_ADDRESS`
- `NEXT_PUBLIC_COLLATERAL_VAULT_ADDRESS`

## Solana Mode Required (Legacy)

Backend:
- `SOLANA_ENABLED=true`
- `PROGRAM_VAULT_ADDRESS`
- `SOLANA_RPC_URL` (https)
- `SOLANA_WS_URL`

Frontend:
- `NEXT_PUBLIC_CHAIN_MODE=solana`
- `NEXT_PUBLIC_RPC_URL` (https)

## Verification
Run:

```bash
npm run launch:config
```

Soft rehearsal (no production secrets in local dev):

```bash
npm run launch:config:soft
```
