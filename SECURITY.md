# Security

This document describes the security testing approach for Polyguard smart contracts.

## Overview

Polyguard uses property-based fuzzing as the primary method for detecting vulnerabilities in the Solana program logic. While not a replacement for formal verification or third-party audits, extensive fuzzing provides strong assurance against common vulnerability classes.

## Fuzz Testing Infrastructure

### Targets

Seven fuzz targets cover critical program operations:

| Target | Description | Invariants Tested |
|--------|-------------|-------------------|
| `orderbook_operations` | Order placement and cancellation sequences | State consistency, fund locking/unlocking |
| `price_matching` | Bid/ask matching logic | Price ordering, match correctness |
| `arithmetic_safety` | Integer arithmetic in financial calculations | Overflow/underflow protection, rounding |
| `settlement` | Trade settlement and refunds | Collateral conservation, position updates |
| `redemption` | Token redemption after resolution | Correct payouts per outcome |
| `market_resolution` | Oracle threshold evaluation | Deterministic outcomes, boundary conditions |
| `fee_calculations` | Fee arithmetic and accumulation | Fee caps, protocol/referrer splits |

### Running Fuzz Tests

Prerequisites:
```bash
rustup install nightly
cargo install cargo-fuzz
```

Quick test (5 minutes per target):
```bash
./scripts/fuzz-campaign.sh
```

Extended campaign (recommended before releases):
```bash
./scripts/fuzz-campaign.sh --duration 3600
```

Single target deep testing:
```bash
./scripts/fuzz-campaign.sh --target arithmetic_safety --duration 7200
```

CI integration:
```bash
./scripts/fuzz-campaign.sh --ci
```

### What Gets Tested

**Order Lifecycle**
- Order placement with various types (limit, market, post-only, IOC, FOK)
- Order cancellation and fund release
- Partial fills and remaining quantity tracking
- Expiration handling

**Financial Arithmetic**
- Cost calculation: `quantity * price / 10000`
- Fee calculation: `cost * fee_bps / 10000`
- Refund calculation when fill price < limit price
- Accumulation over many operations without overflow

**Settlement Logic**
- Collateral transfer from escrow to seller
- Buyer refund when price improvement occurs
- Position balance updates (YES/NO tokens)
- Locked balance accounting

**Redemption Logic**
- YES outcome: YES tokens redeem 1:1
- NO outcome: NO tokens redeem 1:1
- Invalid market: 50% refund for all tokens
- Remaining collateral return

**Market Resolution**
- Oracle price evaluation against threshold
- Comparison operators (GT, GTE, LT, LTE, EQ)
- Boundary conditions (price == threshold)
- Staleness and confidence checks

### Reproducing Crashes

If a crash is found, reproduce with:
```bash
cd programs/polyguard-orderbook/fuzz
cargo +nightly fuzz run <target> artifacts/<target>/crash-<hash>
```

## Invariants

The following invariants are checked during fuzzing:

### Balance Invariants
- `locked_collateral <= collateral_balance` for all users
- `locked_yes <= yes_balance` for all users
- `locked_no <= no_balance` for all users
- Total locked matches sum of open order requirements

### Orderbook Invariants
- Bids sorted descending by price
- Asks sorted ascending by price
- Match only occurs when `bid_price >= ask_price`
- Order count in book matches user's open order count

### Arithmetic Invariants
- Cost calculation never overflows for constrained inputs
- `cost + proceeds = quantity` for YES/NO pair (within rounding)
- Fees never exceed trade cost
- Protocol fee + referrer fee = total fee

### Settlement Invariants
- Collateral is conserved (excluding fees)
- Fill price within [sell_price, buy_price]
- Buyer receives correct outcome tokens
- Seller receives correct collateral

### Redemption Invariants
- All tokens burned after redemption
- Collateral received matches expected for outcome
- No funds created or destroyed

## Coverage

After running fuzz tests, generate coverage:
```bash
./scripts/fuzz-campaign.sh --duration 60 --coverage
```

Coverage reports are in `programs/polyguard-orderbook/fuzz/coverage/`.

## Known Limitations

**Not covered by fuzzing:**
- Account constraint validation (handled by Anchor)
- Cross-program invocation (CPI) interactions
- Rent and account size calculations
- Network-level attacks (MEV, frontrunning)

**Requires manual review:**
- PDA derivation correctness
- Access control logic
- Event emission
- Upgrade safety

## Bug Bounty Program

We welcome responsible disclosure of security vulnerabilities.

### Scope
- Solana programs in `programs/`
- SDK in `sdk/`
- API server in `app/`

### Out of Scope
- Third-party dependencies
- Infrastructure (unless leading to fund loss)
- Social engineering
- Denial of service without fund impact

### Rewards

| Severity | Impact | Reward |
|----------|--------|--------|
| Critical | Direct fund loss | $10,000 - $50,000 |
| High | Fund loss under specific conditions | $5,000 - $10,000 |
| Medium | Incorrect state that could lead to loss | $1,000 - $5,000 |
| Low | Non-exploitable bugs | $100 - $1,000 |

Rewards are at our discretion based on impact, quality of report, and exploitability.

### Reporting

Email: security@polyguard.cc

Include:
1. Description of the vulnerability
2. Steps to reproduce
3. Potential impact
4. Suggested fix (optional)

We will acknowledge receipt within 48 hours and provide an initial assessment within 7 days.

### Safe Harbor

We will not pursue legal action against researchers who:
- Make good faith efforts to avoid privacy violations and service disruption
- Do not access or modify data belonging to other users
- Report vulnerabilities promptly and do not disclose publicly before resolution

## Security Contact

For security-related inquiries:
- Email: security@polyguard.cc
- Response time: 48 hours

For general questions, use GitHub issues.

## Audit History

| Date | Auditor | Scope | Status |
|------|---------|-------|--------|
| TBD | Internal fuzzing | Core program logic | Ongoing |

We plan to pursue formal audits as the protocol matures. This document will be updated with audit reports.
