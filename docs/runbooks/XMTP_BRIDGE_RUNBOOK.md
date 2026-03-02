# XMTP Bridge Runbook

## Purpose
Run the XMTP HTTP bridge used by backend swarm transport when:
- `XMTP_SWARM_ENABLED=true`
- `XMTP_SWARM_TRANSPORT=xmtp_http`

## Required Environment
- `XMTP_ENV` (`production` for mainnet)
- `XMTP_WALLET_KEY` (agent wallet private key, `0x...`)
- `XMTP_DB_ENCRYPTION_KEY` (32-byte hex key)
- `XMTP_DB_DIRECTORY` (persistent directory path)
- `XMTP_BRIDGE_HOST` (default `127.0.0.1`)
- `XMTP_BRIDGE_PORT` (default `8090`)

Backend must also set:
- `XMTP_SWARM_BRIDGE_URL` (for example `http://127.0.0.1:8090`)

## Start
```bash
npm run xmtp:bridge
```

## Health Check
```bash
curl -fsS http://127.0.0.1:8090/health
```

Expected:
- `ok: true`
- `address` present

## Failure Modes
- `ok: false` on `/health`: invalid XMTP env vars or wallet key.
- Backend `XMTP bridge rejected ...`: bridge reachable but request validation failed.
- Backend `Failed to ... XMTP bridge`: bridge down/unreachable.

## Recovery
1. Verify bridge env values and key material.
2. Restart bridge process.
3. Confirm `/health` returns `ok: true`.
4. Re-run backend readiness checks:
```bash
npm run launch:config
npm run launch:readiness:strict
```
