# Web4 Pivot Implementation (Current Codebase)

Date: 2026-02-27  
Scope: implemented code only (Base-first)

## Objective
Reposition NeuraMinds from "prediction market UI with optional bots" to a Web4 agent network where machine clients can discover capabilities, launch agents, and run autonomous execution loops.

## What was implemented

### 1) Agent runtime is now publicly discoverable
New Base read endpoints:
- `GET /v1/evm/agents`
- `GET /v1/evm/agents/{agent_id}`

Each agent snapshot now includes:
- owner, market, side, price, size
- cadence and expiry window
- active flag
- `next_execution_at`
- `can_execute`
- computed status: `ready | cooldown | inactive`

### 2) Web4 discovery endpoints for machine clients
New protocol-facing routes:
- `GET /v1/web4/capabilities`
- `GET /v1/web4/mcp`
- `POST /v1/web4/mcp` (JSON-RPC 2.0)
- `GET /v1/web4/agent-card`
- stdio MCP process: `npm run mcp:server` (`scripts/mcp-server.mjs`)

Purpose:
- `capabilities`: current implementation status by protocol layer
- `mcp`: MCP manifest + live MCP JSON-RPC transport (`initialize`, `tools/*`, `resources/*`, `prompts/*`)
- `mcp:server`: full process MCP server over stdio using the official SDK
- `agent-card`: A2A-style service card for cross-agent discovery

### 3) Agent control plane in frontend
New app route:
- `/agents`

Shipped features:
- Agent directory with filters
- Launch onchain agent flow
- Execute due agents
- Live status display (`ready/cooldown/inactive`)

Write path is real:
- `POST /v1/evm/write/agents/create`
- `POST /v1/evm/write/agents/execute`
- wallet sign + broadcast + receipt confirmation

### 4) Autonomous execution loop (operator runtime)
New script:
- `scripts/base-agent-executor.sh`

New npm commands:
- `npm run agents:executor`
- `npm run agents:executor:sepolia`
- `npm run agents:executor:dry`

Behavior:
- polls `/v1/evm/agents?active=true`
- selects `can_execute == true`
- submits `executeAgent(agentId)` transactions with operator key

This closes a practical autonomy gap: agents no longer require manual execution clicks.

### 5) Agent-readable repository contract
Added root:
- `AGENTS.md`

Includes:
- Base chain/contract interface
- read/write API surfaces
- SIWE auth flow
- runtime semantics and ops commands

### 6) ERC-8004 + x402 + XMTP implementation
New Web4/infra surfaces:
- `GET /v1/evm/identity/{wallet}`
- `GET /v1/evm/reputation/{wallet}`
- `POST /v1/evm/write/identity/register`
- `POST /v1/evm/write/identity/tier`
- `POST /v1/evm/write/identity/active`
- `POST /v1/evm/write/reputation/outcome`
- `GET /v1/payments/x402/quote`
- `POST /v1/payments/x402/verify`
- `GET /v1/web4/xmtp/health`
- `POST /v1/web4/xmtp/swarm/send`
- `GET /v1/web4/xmtp/swarm/{swarm_id}/messages`

Behavior:
- Agent snapshots are enriched with wallet-level ERC-8004 identity/reputation when configured.
- ERC-8004 write-prep endpoints generate calldata for issuer/attester wallet execution.
- Premium EVM reads (`orderbook`, `trades`) and premium MCP tool calls are x402-gated.
- x402 verifies mined Base USDC transfer txs, quote challenge signatures, nonce replay, and tx-hash replay.
- XMTP swarm supports Redis relay mode and XMTP HTTP bridge mode (`services/xmtp-bridge/server.mjs`) with signature replay protection via nonce/expiry.

## Mapping to your Web4 analysis file

| Web4 proposal | Status in code | Notes |
|---|---|---|
| AGENTS.md project layer | Implemented | Root `AGENTS.md` added |
| MCP server integration | Implemented | HTTP JSON-RPC (`POST /v1/web4/mcp`) + stdio process transport (`npm run mcp:server`) |
| A2A discovery card | Partial | `GET /v1/web4/agent-card` shipped; no external A2A registry publish yet |
| Agent runtime discoverability | Implemented | `/v1/evm/agents*` + `/agents` control plane |
| Autonomous execution | Implemented (operator loop) | `base-agent-executor.sh` executes due agents continuously |
| ERC-8004 identity/reputation | Implemented | Contracts + API enrichment/endpoints |
| x402 + AgentKit payments | Implemented | Quote/verify + premium route enforcement |
| XMTP swarm coordination | Implemented | Redis + bridge mode with signed messages, optional nonce/expiry replay protection |

## Verification run (this pass)
- `cargo check --manifest-path app/Cargo.toml` passed
- `forge test --root evm` passed
- `npm run build` (web) passed

## Remaining non-blocking milestones
1. Publish A2A card to external registries if needed.
2. Add dashboard visibility for ERC-8004 fields in the `/agents` UI.
3. Add dedicated x402/XMTP bridge integration tests against Base Sepolia.
