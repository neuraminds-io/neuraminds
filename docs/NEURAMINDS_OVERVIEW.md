# neuraminds Overview (Current Implementation)

Last updated: 2026-02-26

This document is codebase-first. It describes what is implemented now.

## 1. What neuraminds is in the current code

neuraminds is a Base-focused prediction market stack with:

- EVM contracts for market lifecycle, collateral accounting, matching, payouts, and onchain agent execution.
- A Rust backend for auth, indexed reads, and EVM write API preparation/relay.
- A Next.js frontend for Base wallet UX and market interaction.
- An agent SDK that uses backend EVM write APIs for transaction preparation.

The repository is Base-only on active runtime paths.

## 2. Implemented system components

### 2.1 Onchain contracts (`evm/src/`)

- `NeuraToken.sol`
  - ERC-20 token with cap, mint role, pause role, and permit.
- `MarketCore.sol`
  - `createMarket(questionHash, closeTime, resolver)`
  - `createMarketRich(question, description, category, resolutionSource, closeTime, resolver)`
  - `setMarketMetadata` / `getMarketMetadata`
  - `resolveMarket`, pause/unpause, role control.
- `CollateralVault.sol`
  - deposit/withdraw/lock/unlock/settle and internal `transferAvailable` ledger moves.
- `OrderBook.sol`
  - `placeOrder`, `cancelOrder`, `matchOrders`, `claim`, `claimable`, `placeOrderFor`.
  - `matchOrders` is permissionless (no matcher gate).
  - Uses `CollateralVault` for collateral locking/settlement/payout transfer.
- `AgentRuntime.sol`
  - onchain autonomous agent scheduler/executor:
  - `createAgent`, `updateAgent`, `deactivateAgent`, `executeAgent`.
  - Executes by calling `OrderBook.placeOrderFor` under `AGENT_RUNTIME_ROLE`.

Role model is explicit (`DEFAULT_ADMIN_ROLE`, `MARKET_CREATOR_ROLE`, `RESOLVER_ROLE`, `PAUSER_ROLE`, `OPERATOR_ROLE`, `AGENT_RUNTIME_ROLE`).

### 2.2 Backend API (`app/`)

Server entrypoint: `app/src/main.rs`.

Implemented Base read routes:

- `GET /v1/evm/token/state`
- `GET /v1/evm/markets`
- `GET /v1/evm/markets/{market_id}/orderbook`
- `GET /v1/evm/markets/{market_id}/trades`

Implemented Base write preparation/relay routes:

- `POST /v1/evm/write/markets/create`
- `POST /v1/evm/write/orders/place`
- `POST /v1/evm/write/orders/cancel`
- `POST /v1/evm/write/orders/match`
- `POST /v1/evm/write/positions/claim`
- `POST /v1/evm/write/agents/create`
- `POST /v1/evm/write/agents/execute`
- `POST /v1/evm/write/relay`

Implemented services include Postgres, Redis, EVM RPC, EVM log indexer, websocket hub, and metrics.

Runtime toggles:

- `EVM_ENABLED`
- `EVM_READS_ENABLED`
- `EVM_WRITES_ENABLED`

### 2.3 Frontend (`web/`)

Implemented frontend behavior:

- Base wallet connect/switch via wagmi/viem.
- SIWE auth through Next API auth proxy.
- Base reads through backend `/v1/evm/*` endpoints.
- Base writes now use backend EVM write-preparation endpoints, then wallet signs/sends prepared tx data.
  - market creation (`createMarketRich` path)
  - place/cancel order
  - claim payouts

### 2.4 Agent SDK (`sdk/agent/`)

Implemented SDK behavior:

- `TradingAgent` strategy + risk loop.
- Write operations call backend EVM write API (`/v1/evm/write/*`) to prepare tx data.
- SDK wallet signs/sends prepared transactions.

Agents can remain offchain processes, but execution logic now has an onchain runtime counterpart (`AgentRuntime`).

## 3. Current runtime behavior

### Market creation and metadata

Markets can be created with full onchain text metadata (`question`, `description`, `category`, `resolutionSource`) using `createMarketRich`.

### Order lifecycle and matching

Orders are posted with `placeOrder`. Matching is permissionless through `matchOrders`.

### Collateral and payout flow

Collateral accounting is unified in `CollateralVault`:

- users deposit to vault,
- `OrderBook` locks/settles via vault operator role,
- claims transfer available vault balances from protocol escrow to winners.

### Agent execution

`AgentRuntime.executeAgent` can be called by any executor; it enforces cadence and submits orders via `placeOrderFor`.

### Data exposure

Backend read APIs pull state via `eth_call`, read fills from logs (`OrderFilled`), and return normalized snapshots including onchain market metadata when available.

## 4. Remaining gaps

- No trustless in-contract strategy VM; agent “intelligence” is still configured parameters, while strategy generation remains offchain.
- No slashing/economic incentive layer for third-party executors beyond transaction-level permissionlessness.

## 5. Operational implementation present in repo

- Foundry tests for contracts.
- Rust tests/lint/security checks in CI.
- Docker build checks in CI.
- Launch-readiness and synthetic monitoring scripts in `scripts/`.

## 6. LLM mental model for collaboration

- Source of truth for protocol state: Base contracts.
- Source of truth for read APIs, tx preparation, and relay hooks: Rust backend.
- Source of truth for tx signatures: user wallet / SDK wallet.
- Source of truth for autonomous execution scheduling: `AgentRuntime` onchain with optional offchain executors.
