# Polyguard API Reference

REST API for the Polyguard prediction market platform.

Base URL: `https://api.polyguard.cc/v1`

## Authentication

All authenticated endpoints require a JWT token in the Authorization header:

```
Authorization: Bearer <token>
```

### Get Nonce

Request a nonce for wallet signature authentication.

```
GET /v1/auth/nonce?wallet={wallet_address}
```

**Response:**
```json
{
  "nonce": "abc123...",
  "expires_at": "2024-01-01T00:05:00Z"
}
```

### Login

Authenticate with a signed message.

```
POST /v1/auth/login
```

**Request:**
```json
{
  "wallet": "7xKX...",
  "signature": "base58_signature",
  "message": "polyguard:7xKX...:1704067200:abc123"
}
```

**Response:**
```json
{
  "access_token": "eyJ...",
  "refresh_token": "eyJ...",
  "expires_in": 3600
}
```

### Refresh Token

```
POST /v1/auth/refresh
```

**Request:**
```json
{
  "refresh_token": "eyJ..."
}
```

### Logout

```
POST /v1/auth/logout
```

Revokes the current token.

---

## Markets

### List Markets

```
GET /v1/markets
```

**Query Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| status | string | Filter by status: `active`, `paused`, `closed`, `resolved`, `cancelled` |
| category | string | Filter by category |
| limit | integer | Max results (default: 20, max: 100) |
| offset | integer | Pagination offset |

**Response:**
```json
{
  "markets": [
    {
      "id": "btc-100k-2024",
      "address": "7xKX...",
      "question": "Will BTC reach $100k by end of 2024?",
      "description": "Bitcoin price prediction",
      "category": "crypto",
      "status": "active",
      "yes_price": 0.65,
      "no_price": 0.35,
      "volume_24h": 50000.0,
      "total_volume": 1250000.0,
      "fee_bps": 100,
      "trading_end": "2024-12-31T23:59:59Z",
      "resolution_deadline": "2025-01-07T23:59:59Z"
    }
  ],
  "total": 42,
  "limit": 20,
  "offset": 0
}
```

### Get Market

```
GET /v1/markets/{market_id}
```

### Create Market

Requires Admin or Keeper role.

```
POST /v1/markets
```

**Request:**
```json
{
  "market_id": "btc-100k-2024",
  "question": "Will BTC reach $100k by end of 2024?",
  "description": "Bitcoin price prediction market",
  "category": "crypto",
  "oracle": "oracle_pubkey",
  "collateral_mint": "USDC_mint_address",
  "fee_bps": 100,
  "trading_end": 1735689599,
  "resolution_deadline": 1736294399
}
```

### Get Order Book

```
GET /v1/markets/{market_id}/orderbook
```

**Query Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| outcome | string | `yes` or `no` (default: yes) |
| depth | integer | Number of price levels (default: 20, max: 100) |

**Response:**
```json
{
  "market_id": "btc-100k-2024",
  "outcome": "yes",
  "timestamp": "2024-01-01T12:00:00Z",
  "bids": [
    {"price": 0.65, "quantity": 1000},
    {"price": 0.64, "quantity": 500}
  ],
  "asks": [
    {"price": 0.66, "quantity": 800},
    {"price": 0.67, "quantity": 1200}
  ],
  "spread": 0.01,
  "mid_price": 0.655
}
```

### Get Trades

```
GET /v1/markets/{market_id}/trades
```

**Query Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| outcome | string | Filter by outcome |
| limit | integer | Max results (default: 50, max: 100) |
| before | string | Cursor for pagination |

---

## Orders

All order endpoints require authentication.

### List Orders

```
GET /v1/orders
```

**Query Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| market_id | string | Filter by market |
| status | string | `open`, `filled`, `partially_filled`, `cancelled`, `expired` |
| limit | integer | Max results |
| offset | integer | Pagination offset |

### Get Order

```
GET /v1/orders/{order_id}
```

### Place Order

```
POST /v1/orders
```

**Request:**
```json
{
  "market_id": "btc-100k-2024",
  "side": "buy",
  "outcome": "yes",
  "order_type": "limit",
  "price": 0.65,
  "quantity": 100,
  "expires_at": "2024-01-02T00:00:00Z",
  "private": false
}
```

**Fields:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| market_id | string | Yes | Market identifier |
| side | string | Yes | `buy` or `sell` |
| outcome | string | Yes | `yes` or `no` |
| order_type | string | Yes | `limit` or `market` |
| price | number | Yes | Price (0.01-0.99 for limit orders) |
| quantity | integer | Yes | Amount in base units |
| expires_at | string | No | ISO 8601 expiration time |
| private | boolean | No | Use privacy features (default: false) |

**Response:**
```json
{
  "order_id": "uuid",
  "market_id": "btc-100k-2024",
  "side": "buy",
  "outcome": "yes",
  "order_type": "limit",
  "price": 0.65,
  "quantity": 100,
  "filled_quantity": 0,
  "status": "open",
  "created_at": "2024-01-01T12:00:00Z",
  "tx_signature": null
}
```

### Cancel Order

```
DELETE /v1/orders/{order_id}
```

**Response:**
```json
{
  "order_id": "uuid",
  "status": "cancelled",
  "cancelled_at": "2024-01-01T12:05:00Z",
  "tx_signature": null
}
```

---

## Positions

### List Positions

```
GET /v1/positions
```

Returns all positions for the authenticated user.

**Response:**
```json
{
  "positions": [
    {
      "market_id": "btc-100k-2024",
      "market_question": "Will BTC reach $100k?",
      "yes_balance": 500,
      "no_balance": 200,
      "avg_yes_cost": 0.60,
      "avg_no_cost": 0.35,
      "current_yes_price": 0.65,
      "current_no_price": 0.35,
      "unrealized_pnl": 25.0,
      "realized_pnl": 100.0
    }
  ]
}
```

### Get Position

```
GET /v1/positions/{market_id}
```

### Claim Winnings

```
POST /v1/positions/{market_id}/claim
```

Claim winnings from a resolved market.

---

## User

### Get Profile

```
GET /v1/user/profile
```

**Response:**
```json
{
  "wallet_address": "7xKX...",
  "total_volume": 50000.0,
  "total_trades": 125,
  "open_orders": 3,
  "created_at": "2024-01-01T00:00:00Z"
}
```

### Get Transactions

```
GET /v1/user/transactions
```

**Query Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| type | string | `deposit`, `withdrawal`, `trade`, `claim` |
| limit | integer | Max results |
| offset | integer | Pagination offset |

---

## WebSocket

Real-time updates via WebSocket connection.

```
ws://api.polyguard.cc/ws
```

### Subscribe to Market

```json
{
  "type": "subscribe",
  "channel": "market",
  "market_id": "btc-100k-2024"
}
```

### Message Types

**Price Update:**
```json
{
  "type": "price",
  "market_id": "btc-100k-2024",
  "yes_price": 0.65,
  "no_price": 0.35,
  "timestamp": "2024-01-01T12:00:00Z"
}
```

**Order Book Update:**
```json
{
  "type": "orderbook",
  "market_id": "btc-100k-2024",
  "outcome": "yes",
  "side": "bid",
  "price": 0.65,
  "quantity": 1000,
  "timestamp": "2024-01-01T12:00:00Z"
}
```

**Trade:**
```json
{
  "type": "trade",
  "market_id": "btc-100k-2024",
  "outcome": "yes",
  "price": 0.65,
  "quantity": 50,
  "timestamp": "2024-01-01T12:00:00Z"
}
```

---

## Health & Metrics

### Health Check

```
GET /health
```

**Response:**
```json
{
  "status": "healthy"
}
```

### Detailed Health

```
GET /health/detailed
```

**Response:**
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "uptime_seconds": 86400,
  "checks": {
    "database": {"status": "healthy", "latency_ms": 2},
    "redis": {"status": "healthy", "latency_ms": 1},
    "solana": {"status": "healthy", "latency_ms": 150}
  }
}
```

### Metrics

```
GET /metrics
```

Returns JSON metrics.

### Prometheus Metrics

```
GET /metrics/prometheus
```

Returns metrics in Prometheus format.

---

## Error Responses

All errors follow this format:

```json
{
  "error": {
    "code": "ERROR_CODE",
    "message": "Human readable message"
  }
}
```

**Common Error Codes:**

| Code | HTTP Status | Description |
|------|-------------|-------------|
| UNAUTHORIZED | 401 | Missing or invalid authentication |
| FORBIDDEN | 403 | Insufficient permissions |
| NOT_FOUND | 404 | Resource not found |
| INVALID_PRICE | 400 | Price out of valid range |
| INVALID_QUANTITY | 400 | Quantity too small or too large |
| ORDER_FILLED | 400 | Cannot cancel filled order |
| RATE_LIMITED | 429 | Too many requests |

---

## Rate Limits

- **Authentication endpoints:** 10 requests/minute per IP
- **Read endpoints:** 120 requests/minute per IP
- **Write endpoints:** 60 requests/minute per IP

Rate limit headers:
```
X-RateLimit-Limit: 60
X-RateLimit-Remaining: 55
X-RateLimit-Reset: 1704067260
```
