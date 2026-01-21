# PolySecure API Layer Specification

> Backend Team Documentation - January 2026

## Overview

The API layer provides REST endpoints and WebSocket connections for the PolySecure trading platform. It handles authentication, order management, market data, and real-time updates.

## Technology Options

### Option A: Rust (Actix-web) - Recommended
- Maximum performance
- Type safety with Solana SDK
- Native async runtime
- ~50k req/s capacity

### Option B: Node.js (Express/Fastify)
- Faster development
- Larger ecosystem
- Easier hiring
- ~10k req/s capacity

**Recommendation**: Start with Rust for core order service, Node.js for auxiliary services.

---

## REST API Endpoints

### Base URL
```
Production: https://api.polysecure.io/v1
Staging:    https://api-staging.polysecure.io/v1
```

### Authentication

All authenticated endpoints require:
```
Authorization: Bearer <JWT_TOKEN>
```

Or for wallet-based auth:
```
X-Wallet-Address: <SOLANA_PUBKEY>
X-Signature: <SIGNED_MESSAGE>
X-Timestamp: <UNIX_TIMESTAMP>
```

---

## Markets API

### List Markets

```http
GET /markets
```

Query Parameters:
| Param | Type | Description |
|-------|------|-------------|
| status | string | Filter by status: `active`, `closed`, `resolved` |
| category | string | Filter by category |
| sort | string | Sort by: `volume`, `created`, `deadline` |
| order | string | `asc` or `desc` |
| limit | int | Results per page (default: 20, max: 100) |
| offset | int | Pagination offset |

Response:
```json
{
  "markets": [
    {
      "id": "btc-100k-jan-2026",
      "address": "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin",
      "question": "Will BTC reach $100k by Jan 31, 2026?",
      "description": "Resolves YES if Bitcoin price...",
      "category": "crypto",
      "status": "active",
      "yes_price": 0.72,
      "no_price": 0.28,
      "volume_24h": 125000.00,
      "total_volume": 2500000.00,
      "liquidity": 450000.00,
      "resolution_deadline": "2026-01-31T23:59:59Z",
      "trading_end": "2026-01-31T20:00:00Z",
      "created_at": "2025-12-01T10:00:00Z"
    }
  ],
  "total": 156,
  "limit": 20,
  "offset": 0
}
```

### Get Market Details

```http
GET /markets/{market_id}
```

Response:
```json
{
  "id": "btc-100k-jan-2026",
  "address": "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin",
  "question": "Will BTC reach $100k by Jan 31, 2026?",
  "description": "Resolves YES if Bitcoin...",
  "category": "crypto",
  "status": "active",
  "oracle": "7Np41oeYqPefeNQEHSv1UDhYrehxin3NStELsSKCT4K2",
  "collateral_mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
  "yes_mint": "YESm1nt...",
  "no_mint": "NOm1nt...",
  "yes_price": 0.72,
  "no_price": 0.28,
  "yes_supply": 1500000,
  "no_supply": 1500000,
  "volume_24h": 125000.00,
  "total_volume": 2500000.00,
  "fee_bps": 100,
  "resolution_deadline": "2026-01-31T23:59:59Z",
  "trading_end": "2026-01-31T20:00:00Z",
  "resolved_outcome": null,
  "created_at": "2025-12-01T10:00:00Z"
}
```

### Get Market Order Book

```http
GET /markets/{market_id}/orderbook
```

Query Parameters:
| Param | Type | Description |
|-------|------|-------------|
| outcome | string | `yes` or `no` |
| depth | int | Levels of depth (default: 20) |

Response:
```json
{
  "market_id": "btc-100k-jan-2026",
  "outcome": "yes",
  "timestamp": "2026-01-19T14:30:00Z",
  "bids": [
    { "price": 0.71, "quantity": 5000, "orders": 3 },
    { "price": 0.70, "quantity": 12000, "orders": 7 },
    { "price": 0.69, "quantity": 8000, "orders": 4 }
  ],
  "asks": [
    { "price": 0.73, "quantity": 4500, "orders": 2 },
    { "price": 0.74, "quantity": 10000, "orders": 5 },
    { "price": 0.75, "quantity": 15000, "orders": 8 }
  ],
  "spread": 0.02,
  "mid_price": 0.72
}
```

### Get Market Trades

```http
GET /markets/{market_id}/trades
```

Query Parameters:
| Param | Type | Description |
|-------|------|-------------|
| outcome | string | `yes` or `no` |
| limit | int | Results (default: 50) |
| before | string | Cursor for pagination |

Response:
```json
{
  "trades": [
    {
      "id": "trade_abc123",
      "market_id": "btc-100k-jan-2026",
      "outcome": "yes",
      "price": 0.72,
      "quantity": 1000,
      "side": "buy",
      "timestamp": "2026-01-19T14:29:45Z",
      "tx_signature": "5K8j..."
    }
  ],
  "cursor": "eyJsYXN0X2lkIjoi..."
}
```

---

## Orders API

### Place Order

```http
POST /orders
```

Request:
```json
{
  "market_id": "btc-100k-jan-2026",
  "side": "buy",
  "outcome": "yes",
  "order_type": "limit",
  "price": 0.72,
  "quantity": 1000,
  "expires_at": "2026-01-20T14:30:00Z",
  "private": false
}
```

Response:
```json
{
  "order_id": "ord_xyz789",
  "market_id": "btc-100k-jan-2026",
  "side": "buy",
  "outcome": "yes",
  "order_type": "limit",
  "price": 0.72,
  "quantity": 1000,
  "filled_quantity": 0,
  "status": "open",
  "created_at": "2026-01-19T14:30:00Z",
  "expires_at": "2026-01-20T14:30:00Z",
  "tx_signature": "3Kx9..."
}
```

### Place Private Order (Arcium)

```http
POST /orders/private
```

Request:
```json
{
  "market_id": "btc-100k-jan-2026",
  "side": "buy",
  "outcome": "yes",
  "encrypted_price": "base64_encoded_ciphertext...",
  "encrypted_quantity": "base64_encoded_ciphertext...",
  "range_proof": "base64_encoded_proof...",
  "elgamal_pubkey": "base64_encoded_pubkey..."
}
```

### Get User Orders

```http
GET /orders
```

Query Parameters:
| Param | Type | Description |
|-------|------|-------------|
| market_id | string | Filter by market |
| status | string | `open`, `filled`, `cancelled`, `all` |
| limit | int | Results per page |

### Cancel Order

```http
DELETE /orders/{order_id}
```

Response:
```json
{
  "order_id": "ord_xyz789",
  "status": "cancelled",
  "cancelled_at": "2026-01-19T14:35:00Z",
  "tx_signature": "4Lm2..."
}
```

---

## Positions API

### Get User Positions

```http
GET /positions
```

Response:
```json
{
  "positions": [
    {
      "market_id": "btc-100k-jan-2026",
      "market_question": "Will BTC reach $100k by Jan 31, 2026?",
      "yes_balance": 500,
      "no_balance": 200,
      "avg_yes_cost": 0.68,
      "avg_no_cost": 0.25,
      "current_yes_price": 0.72,
      "current_no_price": 0.28,
      "unrealized_pnl": 26.00,
      "realized_pnl": 150.00
    }
  ]
}
```

### Get Position for Market

```http
GET /positions/{market_id}
```

### Claim Winnings

```http
POST /positions/{market_id}/claim
```

Response:
```json
{
  "market_id": "btc-100k-jan-2026",
  "claimed_amount": 500.00,
  "winning_outcome": "yes",
  "winning_tokens_burned": 500,
  "tx_signature": "5Nk3..."
}
```

---

## User API

### Get User Profile

```http
GET /user/profile
```

Response:
```json
{
  "wallet": "8xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin",
  "username": "trader123",
  "created_at": "2025-06-15T10:00:00Z",
  "stats": {
    "total_trades": 156,
    "total_volume": 125000.00,
    "win_rate": 0.62,
    "pnl_30d": 2500.00,
    "pnl_all_time": 15000.00
  },
  "settings": {
    "default_privacy_mode": "public",
    "notifications_enabled": true
  }
}
```

### Get Transaction History

```http
GET /user/transactions
```

---

## WebSocket API

### Connection

```
wss://ws.polysecure.io/v1
```

### Authentication

Send after connection:
```json
{
  "type": "auth",
  "token": "<JWT_TOKEN>"
}
```

### Subscribe to Market

```json
{
  "type": "subscribe",
  "channel": "market",
  "market_id": "btc-100k-jan-2026"
}
```

### Subscribe to Order Book

```json
{
  "type": "subscribe",
  "channel": "orderbook",
  "market_id": "btc-100k-jan-2026",
  "outcome": "yes"
}
```

### Subscribe to User Orders

```json
{
  "type": "subscribe",
  "channel": "orders"
}
```

### Message Types

#### Price Update
```json
{
  "type": "price",
  "market_id": "btc-100k-jan-2026",
  "yes_price": 0.73,
  "no_price": 0.27,
  "timestamp": "2026-01-19T14:30:01Z"
}
```

#### Order Book Update
```json
{
  "type": "orderbook",
  "market_id": "btc-100k-jan-2026",
  "outcome": "yes",
  "side": "bid",
  "price": 0.72,
  "quantity": 5500,
  "action": "update"
}
```

#### Trade Execution
```json
{
  "type": "trade",
  "market_id": "btc-100k-jan-2026",
  "outcome": "yes",
  "price": 0.72,
  "quantity": 1000,
  "timestamp": "2026-01-19T14:30:02Z"
}
```

#### Order Update (User)
```json
{
  "type": "order_update",
  "order_id": "ord_xyz789",
  "status": "partially_filled",
  "filled_quantity": 500,
  "remaining_quantity": 500,
  "timestamp": "2026-01-19T14:30:03Z"
}
```

---

## Error Responses

All errors follow this format:

```json
{
  "error": {
    "code": "INSUFFICIENT_BALANCE",
    "message": "Insufficient USDC balance to place order",
    "details": {
      "required": 720.00,
      "available": 500.00
    }
  }
}
```

### Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `UNAUTHORIZED` | 401 | Invalid or missing authentication |
| `FORBIDDEN` | 403 | Action not permitted |
| `NOT_FOUND` | 404 | Resource not found |
| `INVALID_REQUEST` | 400 | Malformed request |
| `INVALID_PRICE` | 400 | Price out of valid range |
| `INVALID_QUANTITY` | 400 | Quantity invalid |
| `INSUFFICIENT_BALANCE` | 400 | Not enough funds |
| `MARKET_NOT_ACTIVE` | 400 | Market not trading |
| `ORDER_NOT_FOUND` | 404 | Order doesn't exist |
| `RATE_LIMITED` | 429 | Too many requests |
| `INTERNAL_ERROR` | 500 | Server error |

---

## Rate Limits

| Endpoint Type | Limit |
|--------------|-------|
| Public (unauthenticated) | 60 req/min |
| Authenticated (general) | 300 req/min |
| Order placement | 60 req/min |
| WebSocket messages | 100 msg/min |

Rate limit headers:
```
X-RateLimit-Limit: 300
X-RateLimit-Remaining: 295
X-RateLimit-Reset: 1705672200
```

---

## Backend Service Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         LOAD BALANCER                               │
│                    (Nginx / AWS ALB / Cloudflare)                   │
└─────────────────────────────────────────────────────────────────────┘
                                │
        ┌───────────────────────┼───────────────────────┐
        ▼                       ▼                       ▼
┌───────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  API Server   │     │   API Server    │     │   API Server    │
│   (Rust)      │     │    (Rust)       │     │    (Rust)       │
│   Port 8080   │     │   Port 8080     │     │   Port 8080     │
└───────────────┘     └─────────────────┘     └─────────────────┘
        │                       │                       │
        └───────────────────────┼───────────────────────┘
                                │
        ┌───────────────────────┼───────────────────────┐
        ▼                       ▼                       ▼
┌───────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   PostgreSQL  │     │     Redis       │     │      NATS       │
│   (Primary)   │     │   (Cache/Pub)   │     │  (Event Bus)    │
└───────────────┘     └─────────────────┘     └─────────────────┘
```

### Service Components

1. **API Gateway**: Routes requests, rate limiting, auth validation
2. **Order Service**: Order book management, matching engine
3. **Market Service**: Market data, price feeds, resolution
4. **User Service**: Authentication, positions, history
5. **Settlement Service**: Solana transaction submission
6. **WebSocket Service**: Real-time updates distribution

### Database Schema (PostgreSQL)

```sql
-- Markets
CREATE TABLE markets (
    id VARCHAR(64) PRIMARY KEY,
    address VARCHAR(44) NOT NULL,
    question TEXT NOT NULL,
    description TEXT,
    category VARCHAR(32),
    status VARCHAR(16) NOT NULL,
    yes_price DECIMAL(10, 6),
    no_price DECIMAL(10, 6),
    volume_24h DECIMAL(18, 2),
    total_volume DECIMAL(18, 2),
    resolution_deadline TIMESTAMP,
    trading_end TIMESTAMP,
    resolved_outcome VARCHAR(8),
    created_at TIMESTAMP DEFAULT NOW()
);

-- Orders
CREATE TABLE orders (
    id VARCHAR(64) PRIMARY KEY,
    market_id VARCHAR(64) REFERENCES markets(id),
    owner VARCHAR(44) NOT NULL,
    side VARCHAR(4) NOT NULL,
    outcome VARCHAR(3) NOT NULL,
    order_type VARCHAR(16) NOT NULL,
    price DECIMAL(10, 6) NOT NULL,
    quantity DECIMAL(18, 6) NOT NULL,
    filled_quantity DECIMAL(18, 6) DEFAULT 0,
    status VARCHAR(16) NOT NULL,
    is_private BOOLEAN DEFAULT FALSE,
    tx_signature VARCHAR(88),
    created_at TIMESTAMP DEFAULT NOW(),
    expires_at TIMESTAMP
);

-- Trades
CREATE TABLE trades (
    id VARCHAR(64) PRIMARY KEY,
    market_id VARCHAR(64) REFERENCES markets(id),
    buy_order_id VARCHAR(64) REFERENCES orders(id),
    sell_order_id VARCHAR(64) REFERENCES orders(id),
    outcome VARCHAR(3) NOT NULL,
    price DECIMAL(10, 6) NOT NULL,
    quantity DECIMAL(18, 6) NOT NULL,
    tx_signature VARCHAR(88),
    created_at TIMESTAMP DEFAULT NOW()
);

-- Positions
CREATE TABLE positions (
    id SERIAL PRIMARY KEY,
    market_id VARCHAR(64) REFERENCES markets(id),
    owner VARCHAR(44) NOT NULL,
    yes_balance DECIMAL(18, 6) DEFAULT 0,
    no_balance DECIMAL(18, 6) DEFAULT 0,
    avg_yes_cost DECIMAL(10, 6),
    avg_no_cost DECIMAL(10, 6),
    UNIQUE(market_id, owner)
);

-- Indexes
CREATE INDEX idx_orders_market ON orders(market_id, status);
CREATE INDEX idx_orders_owner ON orders(owner, status);
CREATE INDEX idx_trades_market ON trades(market_id, created_at DESC);
CREATE INDEX idx_positions_owner ON positions(owner);
```

---

## Deployment Notes

- Use Kubernetes for orchestration
- Redis for order book caching and pub/sub
- NATS or Redpanda for event streaming
- Helius or Triton for Solana RPC
- CloudFlare for DDoS protection
