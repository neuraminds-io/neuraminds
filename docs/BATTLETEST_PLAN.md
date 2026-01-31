# Devnet Battle Test Plan

Internal stress testing on Solana devnet before mainnet launch.

## Overview

- **Duration:** 1 week
- **Network:** Solana Devnet
- **Testers:** Us only
- **Goal:** Find bugs through automated + manual testing

## Phase 1: Deploy (Day 1)

```bash
# Deploy contracts to devnet
anchor build
anchor deploy --provider.cluster devnet

# Deploy backend to staging
kubectl apply -k infra/k8s/staging/

# Deploy frontend to staging
vercel --env NEXT_PUBLIC_SOLANA_NETWORK=devnet
```

## Phase 2: Automated Testing (Day 1-3)

### Fuzz Campaign
```bash
# Run all 7 fuzz targets for 1 hour each
./scripts/fuzz-campaign.sh --duration 3600

# Or run overnight (8 hours per target)
./scripts/fuzz-campaign.sh --duration 28800
```

### Load Tests
```bash
cd tests/load
k6 run smoke.js
k6 run load.js
k6 run stress.js
```

### E2E Tests
```bash
cd web
npm run test:e2e
```

## Phase 3: Manual Testing (Day 2-5)

### Trading Flows
- [ ] Create market with 24h expiry
- [ ] Place YES limit order at 50%
- [ ] Place NO limit order at 50%
- [ ] Orders match, check fills
- [ ] Cancel open order
- [ ] Place market order
- [ ] Check position updates
- [ ] Resolve market via oracle
- [ ] Redeem winning tokens
- [ ] Check balance correct

### Edge Cases
- [ ] Order at 0.01% price
- [ ] Order at 99.99% price
- [ ] Order at MAX_ORDER_QUANTITY (1B)
- [ ] Rapid order spam (hit rate limits)
- [ ] Order on resolved market (should fail)
- [ ] Redeem on unresolved market (should fail)
- [ ] Cancel already-filled order (should fail)
- [ ] Double redeem (should fail)

### Multi-Wallet Tests
- [ ] Wallet A buys YES, Wallet B buys NO
- [ ] Orders match between wallets
- [ ] Both wallets see correct positions
- [ ] Market resolves, winner redeems
- [ ] Loser balance unchanged

### WebSocket Tests
- [ ] Connect without auth (should timeout)
- [ ] Connect with valid JWT
- [ ] Subscribe to orderbook updates
- [ ] Place order, verify update received
- [ ] Disconnect/reconnect

### API Tests
- [ ] All endpoints return 200 with valid auth
- [ ] All endpoints return 401 without auth
- [ ] Rate limits trigger at thresholds
- [ ] Invalid inputs return 400

## Phase 4: Chaos Testing (Day 4-5)

### Simulate Failures
- [ ] Kill API pod, verify recovery
- [ ] Restart database, verify reconnect
- [ ] Flood with invalid requests
- [ ] Send malformed WebSocket messages

### Orderbook Stress
```bash
# Script to spam orders
for i in {1..100}; do
  curl -X POST $API/orders -d '{"side":"buy","price":50,"qty":10}'
  curl -X POST $API/orders -d '{"side":"sell","price":50,"qty":10}'
done
```

## Phase 5: Review (Day 6-7)

### Checklist
- [ ] All manual tests passed
- [ ] Fuzz campaign: 0 crashes
- [ ] Load tests: p99 < 500ms
- [ ] E2E tests: 100% pass
- [ ] No unhandled errors in logs
- [ ] Orderbook state consistent

### Go/No-Go
| Criteria | Required |
|----------|----------|
| Fuzz crashes | 0 |
| Critical bugs | 0 |
| High bugs | 0 |
| E2E pass rate | 100% |
| Manual tests | All pass |

## Quick Commands

```bash
# Check program logs
solana logs <PROGRAM_ID> -u devnet

# Check API health
curl https://api-staging.polyguard.cc/health

# Run single fuzz target
cd programs/polyguard-orderbook
cargo +nightly fuzz run orderbook_operations -- -max_total_time=300

# Get devnet SOL
solana airdrop 2 -u devnet
```

## After Battle Test

If all criteria met:
1. Run final fuzz campaign (overnight)
2. Deploy to mainnet
3. Transfer upgrade authority to multisig
4. Monitor first 24h closely
