# PolySecure Backend Development Setup

> Backend Team Documentation - January 2026

## Prerequisites

### Required Tools

| Tool | Version | Installation |
|------|---------|--------------|
| Rust | 1.91.x | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Solana CLI | 3.0.x | `sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"` |
| Anchor CLI | 0.32.x | `cargo install --git https://github.com/coral-xyz/anchor anchor-cli` |
| Node.js | 24.x | `nvm install 24` |
| Docker | Latest | [docker.com](https://docker.com) |
| PostgreSQL | 16.x | Via Docker or local install |
| Redis | 7.x | Via Docker or local install |

### Verify Installation

```bash
# Check all tools
rustc --version        # rustc 1.91.x
solana --version       # solana-cli 3.0.x
anchor --version       # anchor-cli 0.32.x
node --version         # v24.x.x
docker --version       # Docker version 2x.x.x
```

## Project Setup

### 1. Clone Repository

```bash
git clone https://github.com/polysecure/polysecure-backend.git
cd polysecure-backend
```

### 2. Project Structure

```
polysecure-backend/
├── programs/                    # Solana programs (Anchor)
│   ├── polysecure-market/
│   ├── polysecure-orderbook/
│   └── polysecure-privacy/
├── app/                         # Off-chain backend services
│   ├── api/                     # REST API server
│   ├── orderbook/               # Order matching engine
│   ├── settlement/              # Solana transaction service
│   └── websocket/               # Real-time updates
├── sdk/                         # TypeScript SDK for clients
├── tests/                       # Integration tests
├── migrations/                  # Database migrations
├── docker/                      # Docker configurations
├── scripts/                     # Utility scripts
├── Anchor.toml
├── Cargo.toml
└── package.json
```

### 3. Environment Configuration

Create `.env` file:

```bash
cp .env.example .env
```

Edit `.env`:

```env
# Solana Configuration
SOLANA_RPC_URL=https://api.devnet.solana.com
SOLANA_WS_URL=wss://api.devnet.solana.com
CLUSTER=devnet

# For mainnet (when ready):
# SOLANA_RPC_URL=https://mainnet.helius-rpc.com/?api-key=YOUR_KEY
# CLUSTER=mainnet

# Program IDs (update after deployment)
MARKET_PROGRAM_ID=MarketXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
ORDERBOOK_PROGRAM_ID=OrderbkXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
PRIVACY_PROGRAM_ID=PrivacyXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX

# Database
DATABASE_URL=postgres://postgres:password@localhost:5432/polysecure
REDIS_URL=redis://localhost:6379

# API Configuration
API_PORT=8080
WS_PORT=8081
API_SECRET=your-secret-key-here

# Arcium (for privacy features)
ARCIUM_MXE_ID=MxeXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX

# Keeper wallet (for settlement transactions)
KEEPER_KEYPAIR_PATH=./keys/keeper.json
```

### 4. Generate Keypairs

```bash
# Create keys directory
mkdir -p keys

# Generate keeper wallet (used for settlement transactions)
solana-keygen new -o keys/keeper.json

# Fund keeper on devnet
solana airdrop 5 $(solana-keygen pubkey keys/keeper.json) --url devnet

# Generate program keypairs (optional - Anchor can generate these)
solana-keygen new -o keys/market-program.json
solana-keygen new -o keys/orderbook-program.json
solana-keygen new -o keys/privacy-program.json
```

## Local Development

### 1. Start Infrastructure

```bash
# Start PostgreSQL and Redis via Docker
docker-compose up -d postgres redis

# Or use the full stack:
docker-compose up -d
```

`docker-compose.yml`:

```yaml
version: '3.8'

services:
  postgres:
    image: postgres:16
    environment:
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: password
      POSTGRES_DB: polysecure
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis_data:/data

  # Optional: Local Solana validator
  solana:
    image: solanalabs/solana:v2.1
    command: solana-test-validator --reset
    ports:
      - "8899:8899"
      - "8900:8900"

volumes:
  postgres_data:
  redis_data:
```

### 2. Database Setup

```bash
# Run migrations
cd app
cargo install sqlx-cli
sqlx database create
sqlx migrate run
```

### 3. Build Solana Programs

```bash
# Build all programs
anchor build

# Build specific program
anchor build -p polysecure-market
```

### 4. Run Tests

```bash
# Run all tests (spins up local validator)
anchor test

# Run specific test file
anchor test tests/market.ts

# Run Rust unit tests
cargo test

# Run with local validator already running
anchor test --skip-local-validator
```

### 5. Deploy to Devnet

```bash
# Configure for devnet
solana config set --url devnet

# Deploy programs
anchor deploy --provider.cluster devnet

# Or deploy specific program
anchor deploy -p polysecure-market --provider.cluster devnet
```

### 6. Start Backend Services

```bash
# Terminal 1: API Server
cd app/api
cargo run

# Terminal 2: Order Matching Engine
cd app/orderbook
cargo run

# Terminal 3: Settlement Service
cd app/settlement
cargo run

# Terminal 4: WebSocket Server
cd app/websocket
cargo run

# Or use the combined runner:
cargo run --bin polysecure-all
```

## Development Workflow

### Making Changes to Programs

```bash
# 1. Edit program code in programs/

# 2. Build
anchor build

# 3. Run tests
anchor test

# 4. Deploy to devnet (if tests pass)
anchor deploy --provider.cluster devnet

# 5. Update program IDs in .env if needed
```

### Making Changes to Backend

```bash
# 1. Edit code in app/

# 2. Run tests
cargo test

# 3. Start with hot-reload (using cargo-watch)
cargo install cargo-watch
cargo watch -x run
```

### Database Migrations

```bash
# Create new migration
sqlx migrate add create_orders_table

# Edit migrations/YYYYMMDD_create_orders_table.sql

# Run migrations
sqlx migrate run

# Revert last migration
sqlx migrate revert
```

## Useful Commands

### Solana CLI

```bash
# Check balance
solana balance

# Get account info
solana account <PUBKEY>

# Get program info
solana program show <PROGRAM_ID>

# View recent transactions
solana transaction-history <PUBKEY>

# Airdrop (devnet only)
solana airdrop 2
```

### Anchor CLI

```bash
# Initialize new program
anchor new <program-name>

# Generate IDL
anchor idl init <PROGRAM_ID> --provider.cluster devnet

# Upgrade program
anchor upgrade target/deploy/polysecure_market.so \
  --program-id <PROGRAM_ID> \
  --provider.cluster devnet
```

### Debugging

```bash
# View program logs
solana logs <PROGRAM_ID>

# Decode transaction
solana confirm -v <TX_SIGNATURE>

# Simulate transaction (dry run)
solana simulate <TX_FILE>
```

## IDE Setup

### VS Code Extensions

- **rust-analyzer**: Rust language support
- **Solana**: Solana development support
- **Even Better TOML**: TOML file support
- **SQLx**: SQL support with compile-time checking

### Recommended Settings

`.vscode/settings.json`:

```json
{
  "rust-analyzer.cargo.features": ["devnet"],
  "rust-analyzer.checkOnSave.command": "clippy",
  "editor.formatOnSave": true,
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  }
}
```

## Troubleshooting

### Common Issues

**1. "Program too large"**
```bash
# Optimize program size
[profile.release]
lto = true
opt-level = "z"
```

**2. "Insufficient funds for transaction"**
```bash
# Airdrop more SOL (devnet)
solana airdrop 5

# Or fund from faucet: https://faucet.solana.com
```

**3. "Account not found"**
```bash
# Make sure you're on the right cluster
solana config get

# Switch clusters
solana config set --url devnet
```

**4. "IDL not found"**
```bash
# Rebuild and init IDL
anchor build
anchor idl init <PROGRAM_ID> --filepath target/idl/program.json
```

**5. Database connection issues**
```bash
# Check PostgreSQL is running
docker ps

# Check connection
psql $DATABASE_URL -c "SELECT 1"
```

## Continuous Integration

GitHub Actions workflow (`.github/workflows/ci.yml`):

```yaml
name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest

    services:
      postgres:
        image: postgres:16
        env:
          POSTGRES_PASSWORD: password
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
        ports:
          - 5432:5432

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Install Solana
        run: |
          sh -c "$(curl -sSfL https://release.anza.xyz/v2.1.0/install)"
          echo "$HOME/.local/share/solana/install/active_release/bin" >> $GITHUB_PATH

      - name: Install Anchor
        run: cargo install --git https://github.com/coral-xyz/anchor anchor-cli

      - name: Build programs
        run: anchor build

      - name: Run tests
        run: anchor test

      - name: Run backend tests
        run: cargo test
        env:
          DATABASE_URL: postgres://postgres:password@localhost:5432/polysecure
```

## Next Steps

1. **Set up your local environment** following this guide
2. **Read the architecture docs** (`01-ARCHITECTURE.md`)
3. **Review the program specs** (`02-SOLANA-PROGRAMS.md`)
4. **Understand the API** (`03-API-LAYER.md`)
5. **Study Arcium integration** (`04-ARCIUM-INTEGRATION.md`)
6. **Start with the market program** - it's the foundation

## Resources

- [Solana Documentation](https://solana.com/docs)
- [Anchor Book](https://book.anchor-lang.com/)
- [Anchor Framework GitHub](https://github.com/solana-foundation/anchor)
- [Solana Cookbook](https://solanacookbook.com/)
- [Helius Documentation](https://docs.helius.dev/)
- [Arcium Documentation](https://docs.arcium.com/)
