# Base Migration Board

## Scope
Track Solana -> Base replacement work at module level.

## Onchain
- [x] Add Foundry workspace (`evm/`)
- [x] Add `NeuraToken` contract (cap + role mint + pause)
- [x] Add `MarketCore` lifecycle skeleton
- [x] Add `OrderBook` lifecycle skeleton (place/cancel/fill)
- [x] Add `CollateralVault` skeleton (deposit/withdraw/lock/unlock/settle)
- [x] Add Foundry tests for token, market core, orderbook, and vault
- [ ] Implement full order matching and settlement engine
- [ ] Implement payout routing across market resolution + claim paths
- [ ] Add upgrade/admin governance model and timelock policy

## Backend (app)
- [x] Add Base config fields and `EVM_ENABLED` flag
- [x] Add auth safety gate: block legacy Solana-signature login when `EVM_ENABLED=true`
- [x] Add SIWE nonce+verification endpoints (`/v1/auth/siwe/nonce`, `/v1/auth/siwe/login`)
- [ ] Add EVM RPC client service (replace Solana submission paths)
- [x] Add Base read endpoints (`/v1/evm/token/state`, `/v1/evm/markets` via `eth_call`)
- [x] Add Base orderbook read endpoint (`/v1/evm/markets/{id}/orderbook`)
- [x] Add Base trades read endpoint (`/v1/evm/markets/{id}/trades` via log scan)
- [ ] Add EVM log indexer for markets, orders, and claims
- [x] Migrate auth address validation from Solana pubkey to EVM checksum address
- [ ] Add dual-write/dual-read toggles for controlled rollout

## Frontend (web)
- [x] Add chain-mode constants (`solana`/`base`)
- [x] Add wagmi + viem provider stack
- [x] Add Base wallet hook scaffold (`useBaseWallet`)
- [x] Add Base wallet connect UX and chain switch handling (header + auth flow branch)
- [x] Add Next API auth proxy support for SIWE login flow
- [x] Add first Base read consumer in UI (Settings token state panel)
- [x] Replace market list/detail reads in hooks with Base EVM endpoint (`/v1/evm/markets`)
- [x] Replace orderbook reads in hooks with Base EVM endpoint (`/v1/evm/markets/{id}/orderbook`)
- [x] Replace trade reads in hooks with Base endpoint (`/v1/evm/markets/{id}/trades`)
- [ ] Replace Solana write flows (create order, cancel, claim)
- [x] Update network/token text to Base-native defaults

## DevOps + Docs
- [x] Add Base env scaffold to root `.env.example`
- [x] Add Base deploy scripts in `package.json`
- [x] Add Base deployment runbook with verification commands
- [x] Add Base monitoring checks to synthetic monitor and readiness scripts
- [x] Add CI job for `forge test`
- [x] Add Base Sepolia frontend smoke harness (`scripts/base-sepolia-web-smoke.mjs`, `web/e2e/base-sepolia.spec.ts`)

## Validation Gates
- [x] EVM tests passing (`forge test`, 23 tests)
- [x] Backend smoke test script for Base Sepolia (`scripts/base-sepolia-smoke.mjs`)
- [ ] Frontend E2E smoke executed against live Base Sepolia staging
- [ ] Rollback playbook validated in staging
