# NeuraMinds Open-Permissionless Web4 Launch Closure Status

Last updated: 2026-02-28

## Implemented in this pass

### Workstream 1: Matching Runtime
- Added Base matcher worker script: `scripts/base-matcher-worker.sh`.
- Added matcher API controls:
  - `GET /v1/evm/matcher/health`
  - `GET /v1/evm/matcher/stats`
  - `POST /v1/evm/matcher/pause`
  - `POST /v1/evm/matcher/resume`
  - `POST /v1/evm/matcher/report` (internal worker telemetry)
- Added readiness gate for matcher worker presence.

### Workstream 2: Collateral/Wallet Accounting
- Switched wallet balance endpoint to vault-first onchain sourcing for Base:
  - `available` from `CollateralVault.availableBalance`
  - `locked` from `CollateralVault.lockedBalance`
  - `claimable` from `OrderBook.claimable` across user markets
  - `sourceBlock` from current Base block number
- Deposit verification destination updated to `COLLATERAL_VAULT_ADDRESS`.
- Migrated wallet writes to intent-based phases:
  - `prepare` returns canonical vault transactions (`approve` + `deposit`, or `withdraw`)
  - `confirm` verifies onchain tx target, selector, event topic, sender, and vault balance delta
- Frontend wallet UX now executes Base wallet transactions directly and confirms intents against backend.

### Workstream 3: Global Payout Worker + Reconciliation
- Added schema:
  - `payout_jobs`
  - `chain_sync_cursors`
  - `compliance_decisions`
- Added payout API:
  - `GET /v1/evm/payouts/health`
  - `GET /v1/evm/payouts/backlog`
  - `GET /v1/evm/payouts/jobs`
  - `POST /v1/evm/payouts/report` (internal worker job status updates)
- Updated auto-claimer to:
  - seed payout jobs each cycle
  - report processing/retry/paid transitions to API

### Workstream 4: Indexer Durability
- Added persistent cursor restore/save flow (`chain_sync_cursors`) for EVM indexer.
- Indexer now restores cursor on boot and persists per-cycle metadata.
- Added indexer API:
  - `GET /v1/evm/indexer/health`
  - `GET /v1/evm/indexer/lag`
  - `POST /v1/evm/indexer/backfill`

### Workstream 5: Web4 Runtime Health
- Added aggregate runtime endpoint:
  - `GET /v1/web4/runtime/health`

### Workstream 6: Compliance Controls
- Geo policy now blocks write methods only (reads remain accessible).
- Denied write attempts are persisted into immutable compliance audit records.
- Added compliance API:
  - `GET /v1/compliance/policy`
  - `POST /v1/compliance/decision` (admin/internal)
- Added sanctions write-path enforcement:
  - configured by `SANCTIONS_BLOCKED_ADDRESSES`
  - write attempts by blocked wallets are denied with `SANCTIONS_BLOCKED`
  - decisions are persisted in compliance audit table.

### Workstream 7: CI/CD + Gate Integrity
- Added canonical address manifest:
  - `config/deployments/base-addresses.json`
- Added drift validator:
  - `scripts/validate-address-manifest.mjs`
  - checks env/workflow/report drift
- Added launch scripts:
  - `launch:config:dev-strict`
  - `launch:config:prod-strict`
  - `launch:addresses`
- Production strict validation now forbids `allow-missing-secrets`.
- Updated launch-readiness workflow to strict production config validation.

## Validation run
- `cargo check --manifest-path app/Cargo.toml`
- `cargo test --manifest-path app/Cargo.toml --quiet`
- `npm run production:gates:strict`
- `npm run launch:config:prod-strict`
- `npm run launch:summary`

All passed in local run.

## Remaining closure items
- Matcher is a production worker with admin controls, but not a fully permissionless decentralized matcher network.
- Full index replay persistence is cursor-durable, but not yet a complete event-store backfill architecture.
