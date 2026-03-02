# Singularity Dual-Native Domain Spec

Status: frozen baseline for dual-native implementation (Solana + Base)
Date: 2026-03-02

## 1. Core Principles
- Internal core markets are chain-canonical only.
- Solana and Base operate as separate matching/liquidity domains.
- API parity is behavior parity, not bytecode parity.
- Legacy `provider=ledger` is an alias only and maps to `source=core&chain=solana` during deprecation.

## 2. Market Lifecycle
### 2.1 Create
- Required: `question`, `category`, `tradingEnd`, `resolutionDeadline`, `resolutionMode`.
- Market starts in `active` unless explicitly paused by operator policy.
- Deterministic IDs:
  - Solana canonical ID emitted by program account.
  - Base canonical ID emitted by `MarketCreated` event (`marketId` + address reference).

### 2.2 Pause / Resume / Close / Cancel
- `pause`: operator or pauser role.
- `resume`: operator role only.
- `close`: trading close before resolution.
- `cancel`: terminal state that disables claims except cancellation/refunds logic.

### 2.3 Resolve
- Resolution outcome is binary `yes|no`.
- Resolver authority:
  - Solana committee authority.
  - Base oracle committee/timelock-governed resolver flow.
- Resolution must emit immutable evidence metadata (`oracleSource`, `evidenceHash`, resolver identity).

## 3. Order Lifecycle
### 3.1 Place
- Supported outcomes: `yes` or `no`.
- Supported sides: `buy` or `sell`.
- Supported types: `limit` and launch-compatible market semantics.
- Validation:
  - Market tradable and not terminal.
  - Price bounds (0.01 - 0.99 equivalent bps bounds onchain).
  - Quantity > 0.
  - Optional expiry > now.

### 3.2 Match
- Matching is chain-local.
- Match event must include maker/taker refs, market ref, outcome, fill qty, execution price.
- Fees are applied at settlement/claim boundaries and routed to treasury.

### 3.3 Cancel / Expire
- Owner or privileged operator can cancel when open.
- Expiry transitions open orders to terminal expired state.
- Any unfilled collateral is released according to chain-specific collateral rules.

## 4. Settlement and Claims
- Binary payout: winning side token claimable at 1:1 collateral unit.
- Settlement fee: 50 bps.
- Fee destination is constrained to configured treasury authority.
- Claim requires proof of winning inventory on the canonical chain state.

## 5. Disputes and Oracle Committee
- Dispute scope: unresolved or contested resolution events.
- Voting model:
  - Committee members submit outcome vote (`yes|no|cancel`).
  - Quorum threshold required.
  - Finalized outcome triggers resolve/cancel transition.
- Full vote history must be indexable by chain and dispute ref.

## 6. Agent Policy Constraints
- Per-agent policy fields:
  - `maxOrderQuantity`
  - `maxOrderNotional`
  - `maxOpenOrders`
  - category allowlist (API/runtime enforcement)
- Policy violation is hard-reject (no best-effort fallback).

## 7. Access Control and Governance
- Operator role: matcher/executor actions.
- Committee role: resolution voting/finalization.
- Timelock-controlled admin updates for privileged configuration.
- Pause controls must be separate from normal operator trade flow.

## 8. API Canonical Semantics
- Core IDs:
  - Solana: `sol:<market_ref>`
  - Base: `base:<market_ref>`
- Legacy IDs:
  - `mkt-*` accepted as compatibility alias to Solana core during migration.
- Core query model:
  - `source=core|all|...`
  - `chain=solana|base|all`

## 9. Projection and Storage Model
- DB is projection/cache/audit only.
- Projection tables are materialized from chain/indexer streams.
- Legacy ledger tables remain read-only archive post-cutover.

## 10. Migration Rules (Legacy -> Solana Core)
- Economically active state migrates fully to Solana core.
- Historical fills remain archive-projected.
- Migration uses treasury-prefunded escrow.
- Reconciliation required per wallet and per market before cutover.
