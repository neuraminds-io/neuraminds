# PolySecure Backend Architecture

> Backend Team Documentation - January 2026

## Overview

PolySecure is a privacy-first prediction market platform on Solana. The backend consists of three main layers:

1. **Solana Programs (On-Chain)** - Smart contracts handling settlement, escrow, and privacy
2. **Off-Chain Services** - Order matching, API layer, real-time updates
3. **Privacy Layer** - Arcium MPC integration for confidential operations

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         CLIENT LAYER                                │
│  (Web Dashboard / Mobile App / API Consumers)                       │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      API GATEWAY (REST + WebSocket)                 │
│  - Authentication / Rate Limiting / Request Routing                 │
└─────────────────────────────────────────────────────────────────────┘
                                │
        ┌───────────────────────┼───────────────────────┐
        ▼                       ▼                       ▼
┌───────────────┐     ┌─────────────────┐     ┌─────────────────┐
│ ORDER SERVICE │     │  MARKET SERVICE │     │  USER SERVICE   │
│               │     │                 │     │                 │
│ - Order Book  │     │ - Market CRUD   │     │ - Auth          │
│ - Matching    │     │ - Resolution    │     │ - Positions     │
│ - Execution   │     │ - Oracle Feed   │     │ - History       │
└───────────────┘     └─────────────────┘     └─────────────────┘
        │                       │                       │
        └───────────────────────┼───────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    SETTLEMENT LAYER (Solana)                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌────────────┐ │
│  │   Market    │  │  Order Book │  │   Privacy   │  │   Token    │ │
│  │   Factory   │  │   Program   │  │   Layer     │  │   Vault    │ │
│  │  (Anchor)   │  │  (Anchor)   │  │  (Arcium)   │  │  (SPL/T22) │ │
│  └─────────────┘  └─────────────┘  └─────────────┘  └────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
```

## Core Design Decisions

### 1. Hybrid Order Book (CLOB)

We use a **Central Limit Order Book** model similar to Polymarket:

- **Off-chain matching**: Orders are matched in our backend for speed (sub-second)
- **On-chain settlement**: Matched trades settle on Solana for finality
- **Keeper network**: Optional future enhancement for decentralized matching

**Why not AMM?**
- Better price discovery for binary markets
- Lower slippage for large orders
- Easier regulatory compliance
- No liquidity pool capital requirements

### 2. Privacy Modes

Users can operate in two modes:

| Mode | Balance | Amounts | Identity | Use Case |
|------|---------|---------|----------|----------|
| **Public** | Visible | Visible | Pseudonymous | Standard trading |
| **Private** | Encrypted | Encrypted | Protected | Confidential positions |

### 3. Technology Stack

| Component | Technology | Version (Jan 2026) |
|-----------|------------|-------------------|
| Solana Programs | Anchor Framework | 0.32.x |
| Program Language | Rust | 1.91.x |
| Backend Runtime | Rust (Actix-web) or Node.js | - |
| Database | PostgreSQL + Redis | 16.x / 7.x |
| Message Queue | NATS or Redpanda | - |
| Real-time | WebSocket | - |
| Token Standard | SPL Token-2022 | - |
| Privacy Layer | Arcium SDK | Mainnet |

## Data Flow

### Order Lifecycle

```
1. User submits order via API
        │
        ▼
2. Order Service validates & adds to order book
        │
        ▼
3. Matching engine finds counterparty
        │
        ▼
4. Backend constructs settlement transaction
        │
        ▼
5. Transaction submitted to Solana
        │
        ▼
6. Program verifies & executes trade
        │
        ▼
7. Tokens transferred, positions updated
        │
        ▼
8. WebSocket broadcasts update to clients
```

### Private Order Flow (with Arcium)

```
1. User submits encrypted order + ZK proof
        │
        ▼
2. Order Service validates proof (amount hidden)
        │
        ▼
3. Matching uses committed values (private matching)
        │
        ▼
4. Settlement via Arcium MXE
        │
        ▼
5. C-SPL confidential transfer executed
        │
        ▼
6. Encrypted confirmation sent to user
```

## Solana Program Structure

```
programs/
├── polysecure-market/           # Market Factory
│   ├── src/
│   │   ├── lib.rs               # Entry point
│   │   ├── instructions/
│   │   │   ├── create_market.rs
│   │   │   ├── resolve_market.rs
│   │   │   └── close_market.rs
│   │   ├── state/
│   │   │   ├── market.rs
│   │   │   └── outcome.rs
│   │   └── errors.rs
│   └── Cargo.toml
│
├── polysecure-orderbook/        # Order Book & Settlement
│   ├── src/
│   │   ├── lib.rs
│   │   ├── instructions/
│   │   │   ├── place_order.rs
│   │   │   ├── cancel_order.rs
│   │   │   ├── settle_trade.rs
│   │   │   └── claim_winnings.rs
│   │   ├── state/
│   │   │   ├── order.rs
│   │   │   └── position.rs
│   │   └── errors.rs
│   └── Cargo.toml
│
└── polysecure-privacy/          # Arcium Integration
    ├── src/
    │   ├── lib.rs
    │   ├── instructions/
    │   │   ├── private_deposit.rs
    │   │   ├── private_withdraw.rs
    │   │   └── private_settle.rs
    │   └── state/
    │       └── private_account.rs
    └── Cargo.toml
```

## Account Structure (PDAs)

```rust
// Market Account
seeds = [b"market", market_id.as_bytes()]

// User Position
seeds = [b"position", market.key(), user.key()]

// Order Account
seeds = [b"order", market.key(), order_id.to_le_bytes()]

// Outcome Token Mint (YES)
seeds = [b"outcome", market.key(), b"yes"]

// Outcome Token Mint (NO)
seeds = [b"outcome", market.key(), b"no"]

// Escrow Vault
seeds = [b"vault", market.key()]
```

## Security Considerations

1. **Access Control**: All instructions use Anchor's account validation
2. **Reentrancy**: No external CPI calls during state transitions
3. **Integer Overflow**: Use checked math operations
4. **Oracle Security**: Multiple oracle sources with median pricing
5. **Front-running**: Private mode prevents order front-running
6. **Audit**: Security audit required before mainnet

## Performance Targets

| Metric | Target |
|--------|--------|
| Order placement latency | < 100ms |
| Trade settlement | < 2s (Solana block time) |
| WebSocket update delay | < 50ms |
| Order book depth | 10,000+ orders per market |
| Concurrent users | 10,000+ |

## Next Steps

See the following documents for detailed specifications:
- `02-SOLANA-PROGRAMS.md` - Smart contract specifications
- `03-API-LAYER.md` - REST/WebSocket API documentation
- `04-ARCIUM-INTEGRATION.md` - Privacy layer integration guide
- `05-DEVELOPMENT-SETUP.md` - Local development environment

---

**Sources & References:**
- [Arcium Documentation](https://www.arcium.com/)
- [Anchor Framework](https://www.anchor-lang.com/docs)
- [Solana Token-2022 Confidential Transfers](https://solana.com/docs/tokens/extensions/confidential-transfer)
- [Drift Order Book Architecture](https://extremelysunnyyk.medium.com/inside-drift-architecting-a-high-performance-orderbook-on-solana-612a98b8ac17)
