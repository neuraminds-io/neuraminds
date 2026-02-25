# Base Mainnet Launch Runbook

Last updated: February 25, 2026
Owner: Protocol + Backend + Frontend joint release team

## 1. Objective

Launch NeuralMinds on Base mainnet with a controlled cutover from pre-production.

This runbook covers:
- Sepolia rehearsal (required)
- Mainnet deployment
- Verification and smoke checks
- Cutover sequence
- Rollback protocol

## 2. Preconditions

All must be true before mainnet:

- `docs/BASE_MIGRATION_BOARD.md` has no unresolved `P0` migration blockers.
- `npm run evm:test` passes locally and in CI.
- `cargo check --manifest-path app/Cargo.toml` passes.
- `npm --prefix web run build` passes.
- Launch readiness checks pass:
  - `npm run launch:config`
  - `npm run launch:readiness:strict`
- Staging environment is running with Base chain mode and passing synthetic checks.
- Deployer keys are in Foundry keystore (not raw env vars in shell history).

## 3. Required Environment Variables

### 3.1 Contracts/Deploy

- `BASE_RPC_URL`
- `BASE_SEPOLIA_RPC_URL`
- `BASESCAN_API_KEY`
- `FOUNDRY_ACCOUNT`
- `BASE_ADMIN`
- `BASE_TREASURY`
- `NEURA_CAP_WEI`
- `NEURA_INITIAL_SUPPLY_WEI`
- `COLLATERAL_TOKEN_ADDRESS` (if external collateral token is used)

### 3.2 Backend

- `EVM_ENABLED=true`
- `BASE_RPC_URL`
- `BASE_WS_URL`
- `BASE_CHAIN_ID=8453`
- `SIWE_DOMAIN`
- `NEURA_TOKEN_ADDRESS`
- `MARKET_CORE_ADDRESS`
- `ORDER_BOOK_ADDRESS`
- `DATABASE_URL`
- `REDIS_URL`
- `JWT_SECRET`
- `CORS_ORIGINS`

### 3.3 Frontend

- `NEXT_PUBLIC_CHAIN_MODE=base`
- `NEXT_PUBLIC_API_URL`
- `NEXT_PUBLIC_BASE_RPC_URL`
- `NEXT_PUBLIC_BASE_CHAIN_ID=8453`
- `NEXT_PUBLIC_SIWE_DOMAIN`
- `AUTH_ALLOWED_ORIGINS`

## 4. Sepolia Rehearsal (Mandatory)

Run this full sequence before mainnet deployment.

### 4.1 Deploy contracts to Sepolia

```bash
npm run evm:deploy:base-sepolia
```

Capture deployed addresses and write them to release notes.

### 4.2 Contract verification checks

- Verify contracts are published on BaseScan for Sepolia.
- Verify constructor args match expected config.
- Verify role assignments (`DEFAULT_ADMIN_ROLE`, `MATCHER_ROLE`, `PAUSER_ROLE`) are correct.

### 4.3 Runtime API checks (staging)

With staging backend configured for Sepolia:

```bash
node scripts/synthetic-monitor.mjs \
  --env staging-base-sepolia \
  --api-url https://<staging-api-host> \
  --web-url https://<staging-web-host> \
  --chain-mode base
```

Expected:
- `api_health` PASS
- `api_health_detailed` PASS
- `api_evm_markets_public` PASS
- `api_evm_orderbook_smoke` PASS
- `api_evm_trades_smoke` PASS
- `web_home` PASS

### 4.4 Functional smoke

Run automated frontend smoke against staging:

```bash
npm run base:web:e2e:sepolia -- --api-url https://<staging-api-host> --web-url https://<staging-web-host>
```

Then perform manual staging smoke:
- SIWE login success
- Market list/detail loads
- Orderbook renders
- Trades panel renders
- At least one order lifecycle action in staging test path (if write flow enabled)

## 5. Mainnet Deployment Procedure

## 5.1 Freeze window and comms

- Announce deployment window and rollback owner.
- Freeze non-launch merges.
- Pin target commit SHA for deployment.

### 5.2 Deploy contracts to Base mainnet

```bash
npm run evm:deploy:base
```

Record:
- deploy tx hashes
- deployed addresses
- verification links

### 5.3 Post-deploy onchain verification

Use `cast` against mainnet RPC:

```bash
cast call <NEURA_TOKEN_ADDRESS> "decimals()(uint8)" --rpc-url "$BASE_RPC_URL"
cast call <MARKET_CORE_ADDRESS> "marketCount()(uint256)" --rpc-url "$BASE_RPC_URL"
cast call <ORDER_BOOK_ADDRESS> "orderCount()(uint256)" --rpc-url "$BASE_RPC_URL"
```

Sanity criteria:
- Calls return successfully
- Values are internally consistent with expected launch state

### 5.4 Configure backend and frontend

Update production secrets/config:
- backend Base RPC/WS, chain id, SIWE domain
- contract addresses (`NEURA_TOKEN_ADDRESS`, `MARKET_CORE_ADDRESS`, `ORDER_BOOK_ADDRESS`)
- frontend chain mode and Base chain vars

Redeploy backend and frontend.

### 5.5 Post-config smoke

```bash
node scripts/synthetic-monitor.mjs \
  --env production-base-mainnet \
  --api-url https://<prod-api-host> \
  --web-url https://<prod-web-host> \
  --chain-mode base
```

Run launch summary:

```bash
npm run launch:summary
```

## 6. Production Acceptance Criteria

Mainnet launch is considered successful when:
- All required synthetic monitor checks pass.
- Backend `/health/detailed` reports healthy database/redis/base components.
- SIWE auth path works from production domain.
- Market and orderbook endpoints respond within SLO.
- Error rate and p95 latency remain within production gates for 60 minutes after launch.

## 7. Rollback Plan

Rollback triggers (any one):
- sustained auth failure
- sustained API failure for required Base endpoints
- critical contract misconfiguration discovered post-deploy
- unacceptable error/latency breach without short-term mitigation

### 7.1 Application rollback

1. Repoint frontend to previous stable release.
2. Revert backend deployment to previous image/config revision.
3. If needed, set `EVM_ENABLED=false` and restore prior chain mode behavior.
4. Confirm service recovery with synthetic monitor.

### 7.2 Contract-side emergency response

If contract controls support pausing and emergency roles:
- execute pause actions from authorized role accounts
- suspend risky flows while incident response proceeds

Document all pause tx hashes in incident log.

## 8. Post-Launch Tasks

Within 24 hours:
- publish launch verification report to `docs/reports/`
- update `docs/BASE_MIGRATION_BOARD.md` with completed launch items
- schedule cleanup of deprecated Solana-only paths after stability window

Within 7 days:
- retrospective on launch quality, incidents, and missing safeguards
- create follow-up tasks for any observed debt or operational blind spots
