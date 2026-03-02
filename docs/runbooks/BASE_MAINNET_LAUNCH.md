# Base Mainnet Launch Runbook

Last updated: February 28, 2026
Owner: Protocol + Backend + Frontend release team

## 1. Objective

Launch NeuralMinds on Base with production programs only:
- `MarketCore`
- `OrderBook`
- `CollateralVault`
- `AgentRuntime`

Native token launch is out of scope for this runbook. If token support is required at launch, use the externally launched token address in config.

For launch-ops closure steps and timestamped evidence workflow, use:
- `docs/runbooks/LAUNCH_OPS_FOCUS_2026-03-01.md`
- `scripts/launch-ops-execute.sh`

## 2. Preconditions

All must be true before mainnet:

- `npm run evm:test` passes locally and in CI.
- `cargo check --manifest-path app/Cargo.toml` passes.
- `npm --prefix web run build` passes.
- Launch readiness checks pass:
  - `npm run launch:config`
  - `npm run launch:readiness:strict`
- Staging is running in Base mode and synthetic checks are green.
- Deployer and admin keys are present in Foundry keystore.
- Admin wallet has enough Base ETH for role-wallet funding and deploy tx fees.
- Collateral wallet has planned launch USDC budget.

## 3. Required Environment Variables

### 3.1 Contracts / Deploy

- `BASE_RPC_URL`
- `BASE_SEPOLIA_RPC_URL`
- `FOUNDRY_ACCOUNT`
- `BASE_KEYSTORE_PASSWORD` (in `.env.secrets.local`)
- `BASE_ADMIN`
- `BASE_MARKET_CREATOR`
- `BASE_PAUSER`
- `BASE_RESOLVER`
- `BASE_OPERATOR`
- `BASE_AGENT_RUNTIME_OPERATOR` (optional)
- `BOOTSTRAP_ADMIN`
- `TIMELOCK_MIN_DELAY` (recommended)
- `TIMELOCK_PROPOSER` (recommended)
- `TIMELOCK_EXECUTOR` (recommended)
- `COLLATERAL_TOKEN_BASE_MAINNET`
- `COLLATERAL_TOKEN_BASE_SEPOLIA`
- `BASESCAN_API_KEY` (recommended for verification)
- `DX_TERMINAL_VAULT_ADDRESS` (or `DX_TERMINAL_OWNER_ADDRESS` + DX API access) for strict readiness DX snapshot gate

### 3.2 Backend

- `CHAIN_MODE=base`
- `EVM_ENABLED=true`
- `EVM_READS_ENABLED=true`
- `EVM_WRITES_ENABLED=true`
- `LEGACY_READS_ENABLED=false`
- `LEGACY_WRITES_ENABLED=false`
- `BASE_RPC_URL`
- `BASE_WS_URL`
- `BASE_CHAIN_ID=8453`
- `SIWE_DOMAIN`
- `MARKET_CORE_ADDRESS`
- `ORDER_BOOK_ADDRESS`
- `COLLATERAL_VAULT_ADDRESS`
- `AGENT_RUNTIME_ADDRESS`
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
- `NEXT_PUBLIC_MARKET_CORE_ADDRESS`
- `NEXT_PUBLIC_ORDER_BOOK_ADDRESS`
- `NEXT_PUBLIC_COLLATERAL_TOKEN_ADDRESS`
- `NEXT_PUBLIC_COLLATERAL_VAULT_ADDRESS`
- `AUTH_ALLOWED_ORIGINS`

## 4. Funding Model (Current Launch Budget)

Recommended starting allocation for a `$500` initial budget:

- `$425` USDC collateral (initial market depth)
- `$75` ETH operational reserve (gas + emergency admin actions)

Minimum technical requirement for deploy is much lower, but this split avoids underfunding ops during launch window.

### 4.1 Check balances

```bash
npm run base:wallets:status
```

### 4.2 Fund role wallets from admin signer

If the admin key is not in Foundry keystore yet:

```bash
cast wallet import base-admin --interactive
```

Sepolia rehearsal funding:

```bash
npm run base:fund:sepolia
```

Mainnet funding:

```bash
npm run base:fund:mainnet
```

If needed, override defaults:

```bash
bash scripts/base-distribute-funds.sh --network mainnet --eth-deployer 0.005 --eth-pauser 0.002 --eth-resolver 0.002 --eth-operator 0.002 --usdc-operator 425
```

## 5. Sepolia Rehearsal (Mandatory)

### 5.1 Dry-run deployment

```bash
bash scripts/base-deploy-programs.sh --network sepolia --dry-run --no-verify
```

### 5.2 Broadcast deployment

```bash
bash scripts/base-deploy-programs.sh --network sepolia
```

Artifacts generated:
- `docs/reports/base-programs-deploy-sepolia.json`
- `docs/reports/base-programs-deploy-sepolia.env`
- `docs/reports/base-programs-roles-sepolia.json`

### 5.3 Runtime API checks (staging)

```bash
node scripts/synthetic-monitor.mjs \
  --env staging-base-sepolia \
  --api-url https://<staging-api-host> \
  --web-url https://<staging-web-host> \
  --chain-mode base
```

### 5.4 Frontend smoke

```bash
npm run base:web:e2e:sepolia -- --api-url https://<staging-api-host> --web-url https://<staging-web-host>
```

## 6. Mainnet Deployment Procedure

### 6.1 Freeze window

- Announce deployment window and rollback owner.
- Freeze non-launch merges.
- Pin target commit SHA.

### 6.2 Broadcast programs to Base mainnet

```bash
bash scripts/base-deploy-programs.sh --network mainnet
```

Artifacts generated:
- `docs/reports/base-programs-deploy-mainnet.json`
- `docs/reports/base-programs-deploy-mainnet.env`
- `docs/reports/base-programs-roles-mainnet.json`

### 6.3 Timelock governance handoff (recommended before opening traffic)

Deploy timelock:

```bash
npm run evm:deploy:timelock:base
```

Set `TIMELOCK_ADDRESS` to deployed timelock, then hand off admin roles:

```bash
npm run evm:handoff:timelock:base
```

### 6.4 Post-deploy onchain checks

```bash
cast call <MARKET_CORE_ADDRESS> "marketCount()(uint256)" --rpc-url "$BASE_RPC_URL"
cast call <ORDER_BOOK_ADDRESS> "orderCount()(uint256)" --rpc-url "$BASE_RPC_URL"
cast call <COLLATERAL_VAULT_ADDRESS> "collateral()(address)" --rpc-url "$BASE_RPC_URL"
cast call <ORDER_BOOK_ADDRESS> "claimable(uint256,address)(uint256)" 1 <WALLET_ADDRESS> --rpc-url "$BASE_RPC_URL"
```

### 6.5 Configure backend/frontend

- Apply generated contract addresses to production env.
- Set collateral token and SIWE domain values.
- Redeploy backend + frontend.

### 6.6 Post-config smoke

```bash
node scripts/synthetic-monitor.mjs \
  --env production-base-mainnet \
  --api-url https://<prod-api-host> \
  --web-url https://<prod-web-host> \
  --chain-mode base
npm run launch:summary
```

### 6.7 End-to-end onchain smoke (mainnet)

Run a full create/trade/match/resolve/claim loop with small size:

```bash
npm run base:smoke:mainnet
```

Dry-run planning mode (no tx broadcast):

```bash
npm run base:smoke:mainnet:dry
```

Required smoke keys:
- `BASE_SMOKE_ADMIN_PRIVATE_KEY`
- `BASE_SMOKE_YES_TRADER_PRIVATE_KEY`
- `BASE_SMOKE_NO_TRADER_PRIVATE_KEY` (optional; defaults to yes trader key)

Output:
- `docs/reports/base-mainnet-smoke-report.json`

## 7. Production Acceptance Criteria

Launch is successful only if all are true:

- Synthetic monitor checks pass.
- Role report for mainnet shows all required roles assigned.
- Backend `/health/detailed` is green.
- SIWE login works from production domain.
- Market and orderbook endpoints stay within SLO for 60 minutes.

## 8. Rollback Plan

Rollback immediately on:

- sustained auth failure,
- sustained API failures for required Base endpoints,
- contract misconfiguration,
- unacceptable latency/error spikes.

Immediate actions:

1. Repoint frontend to prior stable release.
2. Roll backend to prior image/config revision.
3. Pause contracts with authorized pauser if required.
4. Confirm recovery using synthetic monitor.

## 9. Post-Launch Tasks

Within 24 hours:

- publish launch verification report in `docs/reports/`
- record deployed addresses and tx hashes in release notes
- open follow-up issues for any observed operational gaps
