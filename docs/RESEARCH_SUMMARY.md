# Research Summary

Analysis of reference implementations for Polyguard prediction market development.

## Repositories Analyzed

1. **OpenBook v2** - Production CLOB on Solana
2. **Manifest** - Modern orderbook with novel data structures
3. **Switchboard v2** - Decentralized oracle network
4. **Hedgehog Markets** - P2P prediction market escrow

---

## OpenBook v2

### Key Takeaways

**CRIT-BIT Tree for Orderbook**
- Binary tree with prefix compression
- 1024 nodes per side (bid/ask)
- Price encoded in top 64 bits of u128 key, sequence in lower 64

**Account Sizes**
- Market: 840 bytes
- BookSide: 90,944 bytes
- EventHeap: 91,280 bytes
- OpenOrdersAccount: 1,256 bytes

**Order Key Encoding**
```rust
pub fn new_node_key(side: Side, price_data: u64, seq_num: u64) -> u128 {
    let seq_num = if side == Side::Bid { !seq_num } else { seq_num };
    let upper = (price_data as u128) << 64;
    upper | (seq_num as u128)
}
```

**Matching Constraints**
- Max 8 orders matched per instruction
- Max 5 expired orders cleaned per iteration
- Self-trade behaviors: DecrementTake, CancelProvide, AbortTransaction

**Fee Structure**
- Fees in 10^-6 units (1,000,000 = 100%)
- Maker fees can be negative (rebates)
- Event heap penalty: 500 lamports per unapplied event

**Settlement Pattern**
- Fill events queued in heap
- `consume_events` instruction processes settlements
- Anyone can crank (permissionless)
- Max 8 events per crank call

---

## Manifest

### Key Takeaways

**HyperTree Innovation**
- All data in uniform 80-byte blocks
- Multiple structures interleave: orders, seats, free list
- Dynamic growth without pre-allocation

**Market Layout**
```
[256-byte Header | Dynamic 80-byte Blocks...]
```

**Account Structures**
- RestingOrder: 64 bytes (price, quantity, trader, expiration)
- ClaimedSeat: 64 bytes (trader, balances, volume)
- Global orders allow cross-market capital efficiency

**Order Types**
```rust
pub enum OrderType {
    Limit,
    ImmediateOrCancel,
    PostOnly,
    Global,                // Cross-market capital sharing
    Reverse,               // AMM-like auto-conversion
    ReverseTight,          // Tighter spread for stables
}
```

**Reverse Order Mechanism**
- Bid fills -> automatically places ask with spread
- AMM-like behavior in orderbook context
- Configurable spread (0.01% to 0.0001%)

**Performance**
- 2x better CU than competitors
- No crank requirements
- Cached best price index (O(1) match initiation)

---

## Switchboard v2

### Key Takeaways

**Data Precision**
- 18 decimal places (i128 fixed-point)
- All values scaled by 10^18

**PullFeedAccountData**
```rust
pub struct PullFeedAccountData {
    pub submissions: [OracleSubmission; 32],
    pub feed_hash: [u8; 32],
    pub max_variance: u64,
    pub min_responses: u32,
    pub max_staleness: u32,
    pub result: CurrentResult,
}

pub struct CurrentResult {
    pub value: i128,
    pub std_dev: i128,
    pub mean: i128,
    pub range: i128,
    pub num_samples: u8,
    pub slot: u64,
}
```

**Reading Oracle Data**
```rust
// Simple read
let price = feed.value(clock.slot)?;

// With verification
let mut verifier = QuoteVerifier::new();
verifier
    .queue(&queue)
    .slothash_sysvar(&slothashes)
    .ix_sysvar(&instructions)
    .max_age(150);

let quote = verifier.verify_instruction_at(0)?;
```

**Validation Checks**
1. Staleness: `feed.is_result_valid(slot)`
2. Sample count: `result.num_samples >= min`
3. Variance: `max_value - min_value <= threshold`
4. Signature verification via ED25519

**Resolution Pattern**
```rust
let oracle_value = feed.result.value;
let outcome = if oracle_value >= market.resolution_threshold {
    Outcome::Yes
} else {
    Outcome::No
};
```

---

## Hedgehog Markets

### Key Takeaways

**Direct Escrow Model**
- No conditional tokens minted
- Positions tracked in UserPosition accounts
- Simpler but less composable than conditional tokens

**Market Structure**
```rust
pub struct Market {
    pub yes_amount: u64,              // Target for YES side
    pub no_amount: u64,               // Target for NO side
    pub yes_filled: u64,
    pub no_filled: u64,
    pub close_ts: u64,                // Deposits stop
    pub expiry_ts: u64,               // Resolution allowed
    pub resolution_delay: u32,        // Finalization delay
    pub outcome: Outcome,
}
```

**Resolution State Machine**
- Before expiry: Open <-> Invalid only
- After expiry: Cannot return to Open
- Auto-finalization triggers:
  - Unfunded after close_ts
  - 30 days past expiry
  - Resolution delay elapsed

**Settlement Formula**
```
winnings = (user_winning_amount / total_winning_side) * prize_pool
prize_pool = total_losing_side
```

**Fee Structure**
- Basis points (0-10000)
- Charged only on winnings at claim time
- Rounds up to ensure protocol captures all fees

**Security Patterns**
- PDA-based account derivation
- Resolver authentication
- Overflow protection (checked arithmetic)
- Position zeroing after redemption

---

## Comparison Matrix

| Feature | OpenBook v2 | Manifest | Hedgehog |
|---------|-------------|----------|----------|
| **Purpose** | Spot DEX | Spot DEX | Prediction |
| **Orderbook** | CRIT-BIT | Red-Black | N/A (escrow) |
| **Account Size** | Large (90KB+) | Dynamic | Small (~400B) |
| **Settlement** | Event-based | Atomic | Direct escrow |
| **Capital Efficiency** | Per-market | Global orders | Per-market |
| **Oracle Support** | Yes (pegged orders) | No | External |
| **Composability** | High | High | Low |

---

## Recommendations for Polyguard

### Phase 1: CLOB
- Use OpenBook's CRIT-BIT tree (proven, efficient)
- Adopt Manifest's cached best index optimization
- Implement per-market accounts (simpler than global orders)

### Oracle Integration
- Switchboard v2 On-Demand for price feeds
- Require 3+ oracle samples for resolution
- Implement resolution delay (24-72 hours)

### Settlement
- Event-based like OpenBook (scalable)
- Auto-cancel orders on resolution
- Pro-rata distribution like Hedgehog

### Account Design
```
Market: ~1KB (metadata + oracle config)
BookSide: ~90KB (1024 orders per side)
EventHeap: ~90KB (600 events)
UserAccount: ~2KB (24 open orders + position)
```

### Security
- Checked arithmetic everywhere
- PDA-based derivation
- Resolution delay prevents manipulation
- Multiple oracle confirmation

---

## Code Patterns to Adopt

### Order Key Encoding (OpenBook)
```rust
pub fn order_key(price_bps: u64, seq_num: u64, is_bid: bool) -> u128 {
    let seq = if is_bid { !seq_num } else { seq_num };
    ((price_bps as u128) << 64) | (seq as u128)
}
```

### Cached Best Price (Manifest)
```rust
pub struct BookSide {
    pub root_index: u32,
    pub best_index: u32,  // O(1) access to best price
    pub nodes: [OrderNode; 1024],
}
```

### Oracle Validation (Switchboard)
```rust
pub fn validate_oracle(feed: &PullFeed, slot: u64) -> Result<i128> {
    require!(feed.is_result_valid(slot), OracleStale);
    require!(feed.result.num_samples >= 3, InsufficientSamples);
    Ok(feed.result.value)
}
```

### Pro-Rata Settlement (Hedgehog)
```rust
let winnings = (user_stake as u128 * prize_pool as u128
               / total_winning_stake as u128) as u64;
```

---

## Next Steps

1. Implement CLOB with OpenBook-style tree
2. Integrate Switchboard oracle
3. Build settlement engine
4. Add event processing (crank)
5. Security audit
6. Mainnet deployment
