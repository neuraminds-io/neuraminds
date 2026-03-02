# Singularity Parity Matrix (Solana <-> Base)

Status: implementation matrix
Date: 2026-03-02

## Legend
- Solana instruction names are conceptual and map to current program entrypoints.
- Base function names map to `evm/src/*.sol` implementation.
- API behavior is canonical and must be equal across chains.

| Domain | Solana Core | Base Core | API Contract | Required Tests |
|---|---|---|---|---|
| Create market | `create_market` | `SingularityMarketCore.createMarket` | `POST /api/markets` (`source=core`, `chain`) | unit + conformance + e2e |
| Pause market | `pause_market` | `pauseMarket` | admin pause endpoint + market state reads | unit + conformance |
| Resume market | `resume_market` | `resumeMarket` | admin resume behavior parity | unit + conformance |
| Close market | `close_market` | `closeMarket` | status transitions reflected in feed | unit + conformance |
| Cancel market | `cancel_market` | `cancelMarket` | terminal cancellation semantics | unit + conformance |
| Resolve market | `resolve_market` | `resolveMarket` or committee finalize | `POST /api/markets/admin/:id/resolve` + disputes | integration + conformance |
| Place order | `place_order` | `SingularityOrderbookCore.placeOrder` | `POST /api/orders` | unit + integration |
| Match orders | matcher flow | `matchOrders` | deterministic fills/trades projection | invariant + conformance |
| Cancel order | `cancel_order` | `cancelOrder` | `DELETE /api/orders/:id` | unit + integration |
| Expire order | expiry sweep | `expireOrder` | order status parity (`expired`) | unit + integration |
| Claim settlement | `claim` | `claim` | `POST /api/positions/:marketId/claim` | unit + e2e |
| Oracle vote | dispute vote ix | `SingularityOracleCommittee.castVote` | dispute vote/finalize endpoints | integration + conformance |
| Agent policy enforce | policy account checks | `SingularityAgentPolicy.enforceOrder` | `POST /api/orders/agent` | unit + integration |
| Timelock admin | governance program flow | `SingularityTimelock.schedule/execute` | admin operations gated by timelock policy | governance tests |

## Response Metadata Parity Requirements
- Every core market/orderbook/trade payload includes:
  - `source`
  - `provider`
  - `chain`
  - `providerMarketRef` (or equivalent)
- Namespaced IDs must resolve in both read and write routes.

## Compatibility Rules During Deprecation Window
- `provider=ledger` remains accepted.
- `source=ledger` maps to `source=core&chain=solana`.
- Legacy `mkt-*` IDs remain resolvable via map tables.
