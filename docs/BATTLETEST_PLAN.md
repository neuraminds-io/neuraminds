# Devnet Battle Test Plan

Public stress testing on Solana devnet before mainnet launch.

## Overview

- **Duration:** 2-4 weeks
- **Network:** Solana Devnet
- **Goal:** Find bugs through real usage, build confidence without paid audit

## Phase 1: Deployment (Day 1-2)

### Smart Contracts
```bash
# Deploy to devnet
anchor build
anchor deploy --provider.cluster devnet

# Verify program
solana program show <PROGRAM_ID> --url devnet
```

### Backend API
```bash
# Deploy to staging with devnet RPC
kubectl apply -k infra/k8s/staging/
```

### Frontend
```bash
# Deploy to Vercel preview or staging subdomain
vercel --env NEXT_PUBLIC_SOLANA_NETWORK=devnet
```

## Phase 2: Internal Testing (Day 3-5)

### Functional Tests
- [ ] Create market with various parameters
- [ ] Place limit orders (buy/sell YES/NO)
- [ ] Place market orders
- [ ] Cancel orders
- [ ] Match orders (self-trade with 2 wallets)
- [ ] Resolve market (oracle and manual)
- [ ] Redeem winnings
- [ ] Check orderbook state consistency

### Edge Cases
- [ ] Order at min/max price (0.01%, 99.99%)
- [ ] Order at MAX_ORDER_QUANTITY
- [ ] Rapid order placement (rate limit test)
- [ ] Concurrent orders from multiple wallets
- [ ] Market resolution at exact threshold
- [ ] Partial fills

### Load Tests
```bash
# Run k6 load tests against devnet
k6 run tests/load/smoke.js --env API_URL=https://api-staging.polyguard.cc
k6 run tests/load/stress.js --env API_URL=https://api-staging.polyguard.cc
```

## Phase 3: Public Battle Test (Day 6-21)

### Announcement
Post on:
- Twitter/X
- Discord (Solana dev channels)
- Telegram
- Reddit r/solana

### Incentives
- Testnet leaderboard with prizes (real SOL for top traders)
- Bug bounty for critical issues (0.5-5 SOL per bug)
- NFT badges for early testers

### Monitoring

#### Metrics to Track
- Total transactions
- Unique wallets
- Orders placed/matched/cancelled
- Markets created
- API error rates
- WebSocket connection count
- RPC error rates

#### Alerts
- Program error rate > 1%
- API latency p99 > 2s
- WebSocket disconnects > 10/min
- Orderbook desync detected

### Data Collection
```bash
# Export metrics daily
curl https://api-staging.polyguard.cc/admin/metrics > metrics-$(date +%Y%m%d).json

# Check program logs
solana logs <PROGRAM_ID> --url devnet > program-logs-$(date +%Y%m%d).log
```

## Phase 4: Analysis (Day 22-28)

### Review Checklist
- [ ] All bugs reported during battle test fixed
- [ ] No critical issues found
- [ ] Load test results acceptable
- [ ] Fuzz campaign completed (10M+ iterations)
- [ ] Manual code review of high-risk areas

### High-Risk Areas for Manual Review
1. Order matching logic (`instructions/place_order.rs`)
2. Settlement calculations (`instructions/consume_events.rs`)
3. Token minting/burning (`instructions/redeem.rs`)
4. Oracle price handling (`state/oracle.rs`)
5. Fee calculations

### Go/No-Go Criteria
| Criteria | Threshold |
|----------|-----------|
| Critical bugs found | 0 |
| High bugs unresolved | 0 |
| Medium bugs unresolved | < 3 |
| Fuzz crashes | 0 |
| Load test pass rate | > 99% |
| Community feedback | Positive |

## Scripts

### Automated Battle Test Bot
```typescript
// scripts/battletest-bot.ts
// Runs automated trading scenarios

const scenarios = [
  'randomOrders',      // Random limit orders
  'arbitrage',         // Cross-market arb
  'marketMaking',      // Bid/ask spread
  'stressTest',        // Rapid fire orders
  'edgeCases',         // Boundary conditions
];

// Run continuously during battle test
for (const scenario of scenarios) {
  await runScenario(scenario, { duration: '1h' });
}
```

### Daily Report Generator
```bash
#!/bin/bash
# scripts/daily-report.sh

echo "=== Polyguard Battle Test Daily Report ==="
echo "Date: $(date)"
echo ""
echo "Transactions: $(curl -s $API_URL/stats | jq .totalTxs)"
echo "Unique Wallets: $(curl -s $API_URL/stats | jq .uniqueWallets)"
echo "Orders Placed: $(curl -s $API_URL/stats | jq .ordersPlaced)"
echo "Bugs Reported: $(gh issue list --label bug --json number | jq length)"
```

## Bug Bounty Program

### Scope
- Smart contracts (polyguard-orderbook, polyguard-market)
- Backend API
- WebSocket server

### Out of Scope
- Frontend UI bugs (unless security-related)
- Third-party dependencies
- Devnet-specific issues

### Rewards (in SOL)
| Severity | Reward |
|----------|--------|
| Critical (funds at risk) | 5 SOL |
| High (DoS, data leak) | 2 SOL |
| Medium (logic errors) | 0.5 SOL |
| Low (minor issues) | 0.1 SOL |

### Submission
Email: security@polyguard.cc
Or: GitHub Security Advisory

## Timeline

```
Week 1: Deploy + Internal Test
Week 2: Public Battle Test (Phase 1)
Week 3: Public Battle Test (Phase 2)
Week 4: Analysis + Fixes + Final Fuzz
Week 5: Mainnet Deploy (if all criteria met)
```

## Success Metrics

After battle test, we should have:
- 1000+ transactions processed
- 100+ unique wallets tested
- 50+ markets created
- 0 critical bugs
- 10M+ fuzz iterations with 0 crashes
- Positive community feedback
