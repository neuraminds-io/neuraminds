# Polyguard Orderbook Fuzz Testing

Property-based fuzz testing for the Polyguard orderbook smart contracts using libFuzzer.

## Prerequisites

```bash
rustup install nightly
cargo install cargo-fuzz
```

## Fuzz Targets

### Core Operations

#### `orderbook_operations`
Tests sequences of order placements and cancellations:
- State corruption detection
- Invariant violations
- Fund locking/unlocking errors
- Order lifecycle bugs

```bash
cargo +nightly fuzz run orderbook_operations
```

#### `price_matching`
Tests price matching logic:
- Correct bid/ask matching rules
- Price ordering invariants
- Match execution correctness

```bash
cargo +nightly fuzz run price_matching
```

#### `arithmetic_safety`
Tests financial calculations:
- Integer overflow/underflow
- Division by zero
- Rounding errors
- Cost/proceeds invariants

```bash
cargo +nightly fuzz run arithmetic_safety
```

### Settlement & Redemption

#### `settlement`
Tests trade settlement:
- Collateral transfers between buyer and seller
- Position updates after fills
- Partial fill handling
- Refund calculations when fill price < limit price

```bash
cargo +nightly fuzz run settlement
```

#### `redemption`
Tests token redemption after market resolution:
- YES outcome: YES tokens redeem 1:1
- NO outcome: NO tokens redeem 1:1
- Invalid market: 50/50 refund
- Collateral conservation

```bash
cargo +nightly fuzz run redemption
```

### Market Resolution

#### `market_resolution`
Tests oracle price evaluation:
- Threshold comparison operators (GT, GTE, LT, LTE, EQ)
- Boundary conditions (price == threshold)
- Staleness and confidence checks
- Deterministic outcome evaluation

```bash
cargo +nightly fuzz run market_resolution
```

### Fee Calculations

#### `fee_calculations`
Tests fee arithmetic:
- Maker/taker fee calculations
- Protocol fee splits
- Fee accumulation over many trades
- Overflow protection in fee math

```bash
cargo +nightly fuzz run fee_calculations
```

## Running Tests

### Quick Start

Run all targets with default settings (5 minutes each):
```bash
./scripts/fuzz-campaign.sh
```

### Individual Target

Run indefinitely until interrupted:
```bash
cd programs/polyguard-orderbook/fuzz
cargo +nightly fuzz run <target>
```

### Timed Run

```bash
cargo +nightly fuzz run <target> -- -max_total_time=300
```

### With Seed Corpus

```bash
cargo +nightly fuzz run <target> corpus/<target>
```

### Reproduce a Crash

```bash
cargo +nightly fuzz run <target> artifacts/<target>/crash-<hash>
```

## Coverage

Generate coverage report:
```bash
cargo +nightly fuzz coverage <target>
```

Or use the campaign script:
```bash
./scripts/fuzz-campaign.sh --duration 60 --coverage
```

## CI Integration

Add to GitHub Actions:
```yaml
fuzz-test:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@nightly
    - run: cargo install cargo-fuzz
    - run: ./scripts/fuzz-campaign.sh --ci
```

## Invariants Tested

### Balance Invariants
- Locked funds never exceed balance
- Total locked matches sum of open order requirements
- Collateral conserved through settlement

### Orderbook Invariants
- Bids sorted descending by price
- Asks sorted ascending by price
- Best bid/ask cached correctly
- Match only when bid >= ask

### Matching Invariants
- Execution at maker's price
- Cost + proceeds = quantity (for YES/NO pairs)
- Fill price within [sell_price, buy_price]

### Arithmetic Invariants
- No overflow in cost calculations
- Fees never exceed cost
- Shares roundtrip correctly
- Protocol + referrer = total fee

### Settlement Invariants
- Collateral transfer correctness
- Refund calculation accuracy
- Position update consistency

### Redemption Invariants
- All tokens burned
- Correct payout per outcome
- Remaining collateral returned

## Adding New Fuzz Targets

1. Create `fuzz_targets/<name>.rs`:
```rust
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Fuzzing logic here
});
```

2. Add to `Cargo.toml`:
```toml
[[bin]]
name = "<name>"
path = "fuzz_targets/<name>.rs"
test = false
doc = false
```

3. Run:
```bash
cargo +nightly fuzz run <name>
```

## Troubleshooting

### "error: could not compile"
Ensure you're using nightly:
```bash
cargo +nightly fuzz run <target>
```

### Out of memory
Reduce parallelism:
```bash
cargo +nightly fuzz run <target> --jobs 2
```

### Slow startup
Pre-compile:
```bash
cargo +nightly fuzz build
```

### Corpus too large
Minimize:
```bash
cargo +nightly fuzz cmin <target>
```
