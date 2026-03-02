# Polyguard Deployment Guide

## Prerequisites

- Rust 1.75+
- Solana CLI 1.18+
- Anchor CLI 0.30+
- Node.js 18+
- Docker & Docker Compose
- PostgreSQL 16
- Redis 7

## Local Development

### 1. Clone and Install

```bash
git clone https://github.com/NVREVRDOTCOM/polyguard.git
cd polyguard

# Install Rust dependencies
cargo build

# Install Node dependencies
npm install

# Build Solana programs
anchor build
```

### 2. Start Local Validator

```bash
solana-test-validator --reset
```

### 3. Deploy Programs Locally

```bash
anchor deploy
```

### 4. Start Backend Services

```bash
# Start PostgreSQL and Redis
docker compose -f docker-compose.dev.yml up -d

# Run migrations
cd app && cargo run --bin migrate

# Start API server
cargo run --release
```

### 5. Run Tests

```bash
# Rust unit tests
cargo test --workspace

# Solana program tests
anchor test

# Integration tests
npm test
```

---

## Devnet Deployment

### 1. Configure Solana CLI

```bash
solana config set --url devnet
solana-keygen new -o ~/.config/solana/devnet.json
solana config set --keypair ~/.config/solana/devnet.json
```

### 2. Fund Deployer Account

```bash
solana airdrop 5 --url devnet
```

Required SOL:
- polyguard-market: ~1.8 SOL
- polyguard-orderbook: ~1.2 SOL
- polyguard-privacy: ~3.7 SOL

### 3. Build Programs

```bash
anchor build
```

### 4. Deploy Programs

```bash
# Deploy market program
solana program deploy target/deploy/polyguard_market.so --url devnet

# Deploy orderbook program
solana program deploy target/deploy/polyguard_orderbook.so --url devnet

# Deploy privacy program
solana program deploy target/deploy/polyguard_privacy.so --url devnet
```

### 5. Update Program IDs

After deployment, update `Anchor.toml` and `lib.rs` files with new program IDs:

```toml
# Anchor.toml
[programs.devnet]
polyguard_market = "NEW_MARKET_PROGRAM_ID"
polyguard_orderbook = "NEW_ORDERBOOK_PROGRAM_ID"
polyguard_privacy = "NEW_PRIVACY_PROGRAM_ID"
```

### 6. Initialize Programs

```bash
# Initialize oracle registry
anchor run init-registry -- --network devnet

# Add approved oracles
anchor run add-oracle -- --network devnet --oracle <ORACLE_PUBKEY>
```

---

## Render Deployment (Recommended)

### One-Click Deploy

1. Connect GitHub repo to Render dashboard
2. Render auto-detects `render.yaml` blueprint
3. Click "Apply" to create all services

### Manual Setup

**1. Create PostgreSQL Database**
```
Name: polyguard
Region: Singapore
Plan: Starter
PostgreSQL Version: 16
```

**2. Create Web Service**
```
Name: polyguard-api
Region: Singapore
Runtime: Docker
Dockerfile Path: ./app/Dockerfile
Docker Context: .
Plan: Starter
Health Check Path: /health
```

**3. Environment Variables**
```bash
DATABASE_URL=<from database internal URL>
ENVIRONMENT=production
RUST_LOG=info
PORT=8080
JWT_SECRET=<generate 64-char secret>
SOLANA_RPC_URL=https://api.devnet.solana.com
CORS_ORIGINS=https://polyguard.cc,https://app.polyguard.cc
```

**4. Run Migrations**

Migrations run automatically via sqlx. For manual run:
```bash
DATABASE_URL="postgres://user:pass@host/polyguard?sslmode=require" \
  sqlx migrate run --source migrations
```

### Render Files

- `render.yaml` - Infrastructure blueprint
- `app/Dockerfile` - Multi-stage Docker build
- `app/.dockerignore` - Build context optimization

---

## Production Deployment (Self-Hosted)

### Infrastructure Requirements

| Component | Specification | Count |
|-----------|---------------|-------|
| API Servers | 4 vCPU, 8GB RAM | 3+ |
| PostgreSQL | 8 vCPU, 32GB RAM, SSD | 1 primary + 1 replica |
| Redis | 4 vCPU, 16GB RAM | 1 cluster |
| Load Balancer | - | 1 |

### 1. Database Setup

```sql
-- Create database
CREATE DATABASE polyguard;
CREATE USER polyguard WITH ENCRYPTED PASSWORD 'secure_password';
GRANT ALL PRIVILEGES ON DATABASE polyguard TO polyguard;

-- Enable extensions
\c polyguard
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_stat_statements";
```

Run migrations:
```bash
DATABASE_URL="postgres://polyguard:password@host:5432/polyguard" \
  cargo run --bin migrate
```

### 2. Redis Setup

```bash
# redis.conf
maxmemory 4gb
maxmemory-policy allkeys-lru
appendonly yes
```

### 3. Environment Variables

Create `.env.production`:

```bash
# Server
HOST=0.0.0.0
PORT=8080
RUST_LOG=info

# Database
DATABASE_URL=postgres://user:pass@host:5432/polyguard
DATABASE_POOL_SIZE=20

# Redis
REDIS_URL=redis://host:6379

# Solana
SOLANA_RPC_URL=https://api.mainnet-beta.solana.com
KEEPER_KEYPAIR_PATH=/secrets/keeper.json

# Authentication
JWT_SECRET=your-256-bit-secret
JWT_EXPIRY_HOURS=24

# Security
CORS_ORIGINS=https://app.polyguard.ai
IS_DEVELOPMENT=false

# Program IDs
MARKET_PROGRAM_ID=...
ORDERBOOK_PROGRAM_ID=...
PRIVACY_PROGRAM_ID=...
```

### 4. Docker Deployment

```dockerfile
# Dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin polyguard-api

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/polyguard-api /usr/local/bin/
CMD ["polyguard-api"]
```

```yaml
# docker-compose.prod.yml
version: '3.8'

services:
  api:
    image: polyguard-api:latest
    environment:
      - DATABASE_URL=${DATABASE_URL}
      - REDIS_URL=${REDIS_URL}
      - SOLANA_RPC_URL=${SOLANA_RPC_URL}
    ports:
      - "8080:8080"
    deploy:
      replicas: 3
      resources:
        limits:
          cpus: '2'
          memory: 4G
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
```

### 5. Kubernetes Deployment

```yaml
# k8s/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: polyguard-api
spec:
  replicas: 3
  selector:
    matchLabels:
      app: polyguard-api
  template:
    metadata:
      labels:
        app: polyguard-api
    spec:
      containers:
      - name: api
        image: polyguard-api:latest
        ports:
        - containerPort: 8080
        envFrom:
        - secretRef:
            name: polyguard-secrets
        resources:
          requests:
            memory: "2Gi"
            cpu: "1"
          limits:
            memory: "4Gi"
            cpu: "2"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /health/detailed
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 10
---
apiVersion: v1
kind: Service
metadata:
  name: polyguard-api
spec:
  selector:
    app: polyguard-api
  ports:
  - port: 80
    targetPort: 8080
  type: ClusterIP
---
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: polyguard-api
  annotations:
    kubernetes.io/ingress.class: nginx
    cert-manager.io/cluster-issuer: letsencrypt-prod
spec:
  tls:
  - hosts:
    - api.polyguard.ai
    secretName: polyguard-tls
  rules:
  - host: api.polyguard.ai
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: polyguard-api
            port:
              number: 80
```

### 6. Mainnet Program Deployment

```bash
# Configure for mainnet
solana config set --url mainnet-beta

# Use hardware wallet for mainnet deploys
solana config set --keypair usb://ledger

# Deploy with verifiable build
anchor build --verifiable
anchor deploy --provider.cluster mainnet

# Verify deployed bytecode
anchor verify <PROGRAM_ID>
```

---

## Monitoring Setup

### Start Monitoring Stack

```bash
cd monitoring
docker compose up -d
```

Access:
- Prometheus: http://localhost:9090
- Grafana: http://localhost:3000 (admin/polyguard)
- Alertmanager: http://localhost:9093

### Configure Alerting

Edit `monitoring/alertmanager/alertmanager.yml` with your notification channels:

```yaml
receivers:
  - name: 'critical'
    slack_configs:
      - api_url: 'https://hooks.slack.com/services/...'
        channel: '#polyguard-alerts'
    pagerduty_configs:
      - service_key: 'xxx'
```

---

## Security Checklist

### Pre-Deployment
- [ ] All secrets in environment variables or secret manager
- [ ] TLS configured with valid certificates
- [ ] Rate limiting enabled
- [ ] CORS restricted to known origins
- [ ] Database credentials rotated
- [ ] Keeper keypair secured (HSM for mainnet)

### Post-Deployment
- [ ] Verify health endpoints responding
- [ ] Check Prometheus metrics flowing
- [ ] Test alerting (trigger test alert)
- [ ] Verify log aggregation working
- [ ] Run smoke tests against deployment

### Mainnet Specific
- [ ] External security audit completed
- [ ] Bug bounty program active
- [ ] Multisig configured for admin operations
- [ ] Emergency pause procedure documented
- [ ] Incident response plan in place

---

## Rollback Procedure

### API Rollback

```bash
# Kubernetes
kubectl rollout undo deployment/polyguard-api

# Docker
docker compose -f docker-compose.prod.yml down
docker compose -f docker-compose.prod.yml up -d --scale api=3
```

### Program Rollback

Solana programs cannot be rolled back directly. Options:
1. Deploy previous version to new program ID
2. Use program upgrade authority to deploy fix
3. Pause via admin instruction while fixing

```bash
# Pause all markets (emergency)
anchor run pause-all -- --network mainnet
```

---

## Troubleshooting

### API Not Starting

```bash
# Check logs
docker logs polyguard-api

# Common issues:
# - DATABASE_URL incorrect
# - REDIS_URL unreachable
# - KEEPER_KEYPAIR_PATH missing
```

### High Latency

```bash
# Check database connections
SELECT count(*) FROM pg_stat_activity WHERE state = 'active';

# Check Redis memory
redis-cli INFO memory

# Check Solana RPC
curl -X POST -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' \
  $SOLANA_RPC_URL
```

### Transaction Failures

```bash
# Check keeper balance
solana balance --url mainnet-beta

# Check program logs
solana logs --url mainnet-beta <PROGRAM_ID>
```

---

## Backup & Recovery

### Database Backup

```bash
# Daily backup
pg_dump -Fc polyguard > backup_$(date +%Y%m%d).dump

# Restore
pg_restore -d polyguard backup_20260119.dump
```

### Redis Backup

```bash
# Trigger RDB snapshot
redis-cli BGSAVE

# Copy RDB file
cp /var/lib/redis/dump.rdb /backups/
```

### Keeper Keypair Backup

Store encrypted backup in multiple secure locations:
- Hardware security module (HSM)
- Encrypted cloud storage
- Physical safe (paper wallet)

Never store unencrypted keypairs in version control or cloud storage.
