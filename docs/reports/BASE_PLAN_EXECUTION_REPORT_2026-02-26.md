# Base Plan Execution Report

Date: 2026-02-26  
Scope: `docs/BASE_MIGRATION_BOARD.md` end-to-end execution pass.

## Completed

### Onchain
- Implemented deterministic order matching in `evm/src/OrderBook.sol` via `matchOrders`.
- Implemented market-resolution payout claims in `evm/src/OrderBook.sol` via `claim` and `claimable`.
- Added timelock governance path:
  - `evm/script/DeployTimelock.s.sol`
  - `evm/script/HandoffToTimelock.s.sol`
  - `evm/test/TimelockGovernance.t.sol`

### Backend
- Added Base RPC service: `app/src/services/evm_rpc.rs`.
- Added Base log indexer: `app/src/services/evm_indexer.rs`.
- Wired EVM indexer startup in `app/src/main.rs`.
- Refactored Base API reads through shared RPC layer in `app/src/api/evm.rs`.
- Added explicit dual-stack toggles in `app/src/config/mod.rs`:
  - `LEGACY_READS_ENABLED`, `LEGACY_WRITES_ENABLED`
  - `EVM_READS_ENABLED`, `EVM_WRITES_ENABLED`
- Applied legacy read/write gates in:
  - `app/src/api/markets.rs`
  - `app/src/api/orders.rs`
  - `app/src/api/positions.rs`
  - `app/src/api/wallet.rs`

### Frontend
- Enabled Base write flows in:
  - `web/src/components/market/CreateMarketForm.tsx`
  - `web/src/hooks/useOrders.ts`
  - `web/src/hooks/usePositions.ts`
  - `web/src/lib/contracts.ts`
- Updated order quantity semantics in `web/src/components/order/OrderForm.tsx`.
- Added staging API route coverage in Next app for smoke gates:
  - `/health`
  - `/health/detailed`
  - `/v1/auth/siwe/nonce`
  - `/v1/evm/markets`
  - `/v1/evm/markets/{id}/orderbook`
  - `/v1/evm/markets/{id}/trades`

### Agent SDK
- Migrated SDK from Solana client path to Base viem path in `sdk/agent`.

## Validation Evidence

- `forge test --root evm`: passing.
- `cargo test -p polyguard-backend`: passing.
- `npm --prefix web run build`: passing.
- `npm run build` in `sdk/agent`: passing.
- `npm run base:web:e2e:sepolia -- --web-url http://127.0.0.1:3010 --api-url http://127.0.0.1:3010`: passing (3/3).

## Staging Closeout

- Live staging deploy: `dep-d6g5n0lactks73d3uqs0` on `neuraminds-web-base-staging-v4`.
- Frontend smoke gate (live staging): passing (3/3). Evidence: `docs/reports/base-sepolia-web-smoke-2026-02-26.json`.
- Rollback playbook validation: passing (pause/unpause controls across all three Base programs). Evidence: `docs/reports/staging-rollback-validation-2026-02-26.json`.
