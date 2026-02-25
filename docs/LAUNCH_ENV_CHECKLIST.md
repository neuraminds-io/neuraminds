# Launch Environment Checklist

## Backend Required (Production)
- `DATABASE_URL`
- `REDIS_URL`
- `JWT_SECRET` (>= 32 chars)
- `CORS_ORIGINS` (no wildcard, https origins only)
- `METRICS_TOKEN`
- `BLINDFOLD_WEBHOOK_SECRET`
- `PROGRAM_VAULT_ADDRESS`
- `SOLANA_RPC_URL` (https)
- `SOLANA_WS_URL`

## Frontend/Edge Required (Production)
- `NEXT_PUBLIC_API_URL` (https)
- `NEXT_PUBLIC_RPC_URL` (https)
- `AUTH_ALLOWED_ORIGINS` (no wildcard, https origins only)

## Verification
Run:

```bash
npm run launch:config
```

Soft rehearsal (no production secrets in local dev):

```bash
npm run launch:config:soft
```
