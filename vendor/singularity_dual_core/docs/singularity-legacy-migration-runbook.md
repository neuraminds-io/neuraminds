# Legacy Synthetic Ledger Migration Runbook (to Solana Core)

Status: executable staging runbook
Date: 2026-03-02

## 1. Preconditions
- Dual-core API build deployed to staging.
- Solana core programs verified and operational.
- Migration escrow treasury funded with liabilities + margin.
- `SYNTHETIC_LEDGER_WRITES_ENABLED=0` tested in staging.

## 2. Freeze
1. Enable maintenance mode for write routes.
2. Set `SYNTHETIC_LEDGER_WRITES_ENABLED=0`.
3. Verify no new synthetic orders/markets/disputes can be created.

## 3. Snapshot
- Snapshot tables:
  - `keiro_markets`
  - `keiro_orders`
  - `keiro_positions`
  - `keiro_wallet_accounts`
  - `keiro_disputes`
  - `keiro_dispute_votes`
- Compute immutable snapshot hash and store in `keiro_migration_runs`.

## 4. Market Recreation on Solana
- Recreate each active/pending legacy market on Solana with deterministic metadata links.
- Persist map entries in `keiro_legacy_market_map`.

## 5. Capital and Position Allocation
- Prefund migration escrow.
- Allocate wallet balances and position tokens using idempotent batch keys.
- Record deltas in `keiro_migration_deltas`.

## 6. Open Order and Dispute Recreation
- Replay valid open/partially-filled orders only.
- Reconstruct unresolved disputes and vote state in chain-compatible records.

## 7. Reconciliation Gates
- Per-wallet delta check.
- Per-market total check.
- Global liabilities/assets check.
- Hard stop on threshold breach; log failures in `keiro_migration_failures`.

## 8. Cutover
1. Switch API core writes to Solana adapter only.
2. Keep legacy tables read-only.
3. Keep compatibility mapping enabled for `mkt-*` links.

## 9. Rollback Strategy
- If reconciliation fails pre-cutover: abort and replay failed wallets only.
- If post-cutover incident: disable core writes, keep reads on projected state, execute runbook rollback checkpoint.

## 10. Post-Cutover Monitoring
- Monitor:
  - core projection lag per checkpoint
  - claim failures
  - order placement failure rate
  - dispute finalization latency
- Run shadow reconciliation for 24h and 72h windows.
