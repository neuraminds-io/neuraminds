# Deployment Plan

Lean deployment strategy for Polyguard mainnet launch.

## Cost Summary

| Scenario | Peak (deploy) | Final (locked) | Reclaimed |
|----------|---------------|----------------|-----------|
| Full stack (3 programs, 1.5x buffer) | 31 SOL | 15.5 SOL | 15.5 SOL |
| MVP no privacy (1.5x buffer) | 19.6 SOL | 9.8 SOL | 9.8 SOL |
| **Lean MVP (1.0x buffer)** | **8.33 SOL** | **6.57 SOL** | **6.57 SOL** |

## Launch Configuration

**Two-program staged deploy with 1.0x buffer (no upgrade headroom).**

### Programs

| Program | Size | Purpose | Status |
|---------|------|---------|--------|
| `polyguard-market` | 584 KB | Market lifecycle, resolution, disputes | Launch |
| `polyguard-orderbook` | 337 KB | Order placement, matching, settlement | Launch |
| `polyguard-privacy` | 529 KB | Confidential trading, ZK proofs | Deferred to v2 |

### Deployment Sequence

| Step | Action | SOL Required | Wallet After |
|------|--------|--------------|--------------|
| 1 | Deploy `market` | 8.33 | -8.33 |
| 2 | Close market buffer | +4.16 | -4.16 |
| 3 | Deploy `orderbook` | 4.81 | -8.97 |
| 4 | Close orderbook buffer | +2.40 | -6.57 |

**Initial capital: 8.33 SOL** (can reuse reclaimed buffer for orderbook, need +0.64 extra)
**Locked in programs: 6.57 SOL**
**Returned to wallet: 1.76 SOL**

## Feature Flags

The orderbook program uses Cargo features to exclude non-essential modules at compile time.

### Cargo.toml

```toml
[features]
default = ["core"]
core = []                    # Core trading only (~200 KB)
full = ["agents", "social", "enterprise", "defi", "events"]
agents = []                  # AI trading agents
social = []                  # Copy trading, profiles
enterprise = []              # Multi-tenant PaaS
defi = []                    # Yield vaults, margin
events = []                  # Event queue batching
```

### Build Commands

```bash
# Lean MVP (launch) - default
anchor build

# Full build (post-revenue)
anchor build -- --features full
```

### Module Mapping

| Feature | Module | Lines | Notes |
|---------|--------|-------|-------|
| `lean` (default) | Core trading instructions | ~2,000 | Always included |
| `agents` | trading_agent.rs | 508 | AI trading bots |
| `social` | social.rs | 604 | Copy trading |
| `enterprise` | enterprise.rs | 226 | Multi-tenancy |
| `defi` | defi.rs | 483 | Yield/margin |
| `events` | consume_events.rs | 397 | Event crank |

Note: Feature flags currently gate module compilation but not entrypoint exposure. The core lib.rs only exposes 6 instructions regardless of features. Optional instructions would need to be added to the `#[program]` block when enabled.

## Deferred Features

### Phase 2 (Post-Revenue)

1. **Privacy Layer**
   - Wait for Arcium MXE integration finalization
   - Or evaluate SPL Token-2022 Confidential Transfers
   - Deploy as separate program when ready

2. **Trading Agents** (`--features agents`)
   - Enable after validating core trading
   - Requires off-chain agent infrastructure

3. **Social Trading** (`--features social`)
   - Enable after user acquisition
   - Copy vaults need liquidity

4. **Enterprise PaaS** (`--features enterprise`)
   - Enable when enterprise customers arrive
   - API key management, multi-tenancy

5. **DeFi Integration** (`--features defi`)
   - Enable after collateral TVL grows
   - Yield vaults, margin trading

## Upgrade Path

Since we're deploying with 1.0x buffer (no headroom), upgrades require:

1. Deploy new version to fresh program ID
2. Migrate state accounts (if possible) or
3. Create new markets on new program, sunset old

This is acceptable for launch because:
- Core logic is battle-tested in tests
- Can redeploy with larger buffer once revenue flows
- Privacy layer will be separate program anyway

## Revenue Projections

Assuming pump.fun creator fees:

| Markets/Week | Fee/Market | Weekly Revenue | Break-even |
|--------------|------------|----------------|------------|
| 5 | 0.5 SOL | 2.5 SOL | 3 weeks |
| 10 | 0.5 SOL | 5 SOL | 1.5 weeks |
| 20 | 0.5 SOL | 10 SOL | < 1 week |

## Launch Checklist

### Programs (Done)

- [x] Add feature flags to `programs/polyguard-orderbook/Cargo.toml`
- [x] Gate modules with `#[cfg(feature = "...")]`
- [x] Build and verify lean binary size (337 KB)
- [x] Run tests (22 Rust tests + 42 Anchor tests)
- [x] Create deployment scripts

### Pre-Deployment

- [ ] Fund deployment wallet with ~9 SOL
- [ ] Run preflight check: `./scripts/preflight-check.sh`
- [ ] Choose RPC provider (Helius, Triton, or QuickNode recommended)

### Program Deployment

```bash
# 1. Generate new mainnet program keypairs
./scripts/deploy-mainnet.sh --generate-keys

# 2. Deploy programs (staged, ~9 SOL needed)
./scripts/deploy-mainnet.sh --rpc https://your-rpc-url

# 3. Initialize program state
./scripts/init-mainnet.sh
```

### Infrastructure

- [ ] Set up production PostgreSQL database
- [ ] Set up production Redis instance
- [ ] Configure secrets in AWS Secrets Manager / Vault
- [ ] Update `infra/k8s/secrets.yaml` with secret store refs
- [ ] Deploy backend: `kubectl apply -f infra/k8s/`
- [ ] Deploy frontend to Vercel/Cloudflare

### Post-Deploy Verification

- [ ] Verify programs on Solana Explorer
- [ ] Test health endpoints: `curl https://api.polyguard.cc/health/deep`
- [ ] Create test market
- [ ] Place test orders
- [ ] Verify WebSocket connectivity

## Scripts

| Script | Purpose |
|--------|---------|
| `scripts/preflight-check.sh` | Verify prerequisites before deployment |
| `scripts/deploy-mainnet.sh` | Deploy programs to mainnet (staged) |
| `scripts/init-mainnet.sh` | Initialize program configs after deploy |

## RPC Providers

For mainnet, use a dedicated RPC provider:

| Provider | Free Tier | Notes |
|----------|-----------|-------|
| [Helius](https://helius.xyz) | 100k req/day | Recommended, good DAS support |
| [Triton](https://triton.one) | Limited | High performance |
| [QuickNode](https://quicknode.com) | Trial | Easy setup |
| Public RPC | Yes | Rate limited, not for production |
