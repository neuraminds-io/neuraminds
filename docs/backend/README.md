# Polyguard Backend Documentation

> Privacy-First Prediction Market on Solana | Backend Team Docs

**Last Updated:** January 19, 2026

## Quick Links

| Document | Description |
|----------|-------------|
| [PROGRESS.md](./PROGRESS.md) | **Development progress tracker** |
| [01-ARCHITECTURE.md](./01-ARCHITECTURE.md) | System architecture, design decisions, data flow |
| [02-SOLANA-PROGRAMS.md](./02-SOLANA-PROGRAMS.md) | Smart contract specifications (Anchor/Rust) |
| [03-API-LAYER.md](./03-API-LAYER.md) | REST API, WebSocket, database schema |
| [04-ARCIUM-INTEGRATION.md](./04-ARCIUM-INTEGRATION.md) | Privacy layer with Arcium MPC |
| [05-DEVELOPMENT-SETUP.md](./05-DEVELOPMENT-SETUP.md) | Local dev environment setup |

## Tech Stack Summary

```
On-Chain (Solana)
├── Anchor Framework 0.32.x
├── Rust 1.91.x
├── SPL Token-2022 (Confidential Transfers)
└── Arcium SDK (MPC/Privacy)

Off-Chain (Backend)
├── Rust (Actix-web) - Core services
├── PostgreSQL 16 - Primary database
├── Redis 7 - Caching & pub/sub
└── NATS/Redpanda - Event streaming
```

## Architecture Overview

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   Frontend   │────▶│  API Layer   │────▶│   Solana     │
│  Dashboard   │◀────│  (REST/WS)   │◀────│  Programs    │
└──────────────┘     └──────────────┘     └──────────────┘
                            │
                     ┌──────┴──────┐
                     │   Arcium    │
                     │  (Privacy)  │
                     └─────────────┘
```

## Key Design Decisions

1. **Hybrid CLOB**: Off-chain matching, on-chain settlement
2. **Privacy Modes**: Public (standard) or Private (Arcium MPC)
3. **Token-2022**: Using confidential transfer extensions
4. **Microservices**: Separate order, market, settlement services

## Getting Started

```bash
# 1. Read architecture overview
open 01-ARCHITECTURE.md

# 2. Set up development environment
open 05-DEVELOPMENT-SETUP.md

# 3. Review program specifications
open 02-SOLANA-PROGRAMS.md
```

## Status - Sprint 1 Complete

| Component | Status |
|-----------|--------|
| Architecture Design | ✅ Done |
| Program Specs | ✅ Done |
| API Design | ✅ Done |
| Arcium Integration Plan | ✅ Done |
| **Solana Programs** | ✅ **Implemented** |
| **Backend API** | ✅ **Implemented** |
| **Order Matching Engine** | ✅ **Implemented** |
| Tests | 🔄 Scaffolded |
| Devnet Deployment | ⏳ Next |

See [PROGRESS.md](./PROGRESS.md) for detailed implementation status.

---

*Polyguard Backend Team - 2026*
