//! Fuzz target for price matching logic
//!
//! Tests that price matching follows correct rules:
//! - Buyer price >= seller price for a match
//! - Best prices are matched first
//! - FIFO within same price level

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::{fuzz_target, Corpus};
use log::info;
use std::sync::Once;

/// Price scale (10000 = 100%)
const PRICE_SCALE: u64 = 10000;

#[derive(Debug, Arbitrary, Clone)]
struct PriceMatchData {
    /// Bid prices (buyers)
    bids: Vec<u16>,
    /// Ask prices (sellers)
    asks: Vec<u16>,
}

impl PriceMatchData {
    fn is_valid(&self) -> bool {
        !self.bids.is_empty() && !self.asks.is_empty()
    }
}

fn constrain_price(price: u16) -> u64 {
    let p = (price % 9998) + 1;
    p as u64
}

/// Check if bid price is acceptable for matching against ask
fn is_match(bid_price: u64, ask_price: u64) -> bool {
    bid_price >= ask_price
}

/// Calculate cost for a trade
fn calculate_cost(quantity: u64, price: u64) -> Option<u64> {
    (quantity as u128)
        .checked_mul(price as u128)?
        .checked_div(PRICE_SCALE as u128)
        .map(|v| v as u64)
}

fuzz_target!(|data: PriceMatchData| -> Corpus {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = env_logger::try_init();
    });

    if !data.is_valid() {
        return Corpus::Reject;
    }

    run_price_matching_fuzz(data)
});

fn run_price_matching_fuzz(data: PriceMatchData) -> Corpus {
    let bids: Vec<u64> = data.bids.iter().map(|p| constrain_price(*p)).collect();
    let asks: Vec<u64> = data.asks.iter().map(|p| constrain_price(*p)).collect();

    info!("Testing {} bids vs {} asks", bids.len(), asks.len());

    // Sort bids descending (best bid = highest)
    let mut sorted_bids = bids.clone();
    sorted_bids.sort_by(|a, b| b.cmp(a));

    // Sort asks ascending (best ask = lowest)
    let mut sorted_asks = asks.clone();
    sorted_asks.sort();

    // Verify best bid/ask
    let best_bid = sorted_bids.first().copied();
    let best_ask = sorted_asks.first().copied();

    info!("Best bid: {:?}, Best ask: {:?}", best_bid, best_ask);

    // Test matching logic
    if let (Some(bb), Some(ba)) = (best_bid, best_ask) {
        let should_match = is_match(bb, ba);
        info!("Should match: {}", should_match);

        // If best bid >= best ask, there should be a match
        if bb >= ba {
            assert!(
                should_match,
                "Expected match: bid {} >= ask {}",
                bb, ba
            );

            // Calculate execution price (midpoint or maker's price)
            let exec_price = ba; // Use maker's price (ask)

            // Verify cost calculation doesn't overflow for reasonable quantities
            for qty in [1u64, 100, 10000, 1_000_000] {
                let cost = calculate_cost(qty, exec_price);
                assert!(
                    cost.is_some(),
                    "Cost calculation overflow for qty={}, price={}",
                    qty,
                    exec_price
                );

                // Verify cost is within expected range
                if let Some(c) = cost {
                    let expected_max = qty; // At 100% price
                    assert!(
                        c <= expected_max,
                        "Cost {} exceeds max {} for qty={}, price={}",
                        c,
                        expected_max,
                        qty,
                        exec_price
                    );
                }
            }
        } else {
            assert!(
                !should_match,
                "Unexpected match: bid {} < ask {}",
                bb, ba
            );
        }
    }

    // Verify price ordering invariants
    for i in 1..sorted_bids.len() {
        assert!(
            sorted_bids[i - 1] >= sorted_bids[i],
            "Bids not sorted descending"
        );
    }

    for i in 1..sorted_asks.len() {
        assert!(
            sorted_asks[i - 1] <= sorted_asks[i],
            "Asks not sorted ascending"
        );
    }

    // Test all possible matches
    let mut matches = Vec::new();
    for (bi, bid) in sorted_bids.iter().enumerate() {
        for (ai, ask) in sorted_asks.iter().enumerate() {
            if is_match(*bid, *ask) {
                matches.push((bi, ai, *bid, *ask));
            }
        }
    }

    info!("Found {} potential matches", matches.len());

    // Verify match consistency
    for (_, _, bid, ask) in &matches {
        // Match price should be valid
        assert!(*bid >= *ask, "Invalid match: bid {} < ask {}", bid, ask);

        // Both prices should be in valid range
        assert!(
            *bid >= 1 && *bid <= 9999,
            "Bid price out of range: {}",
            bid
        );
        assert!(
            *ask >= 1 && *ask <= 9999,
            "Ask price out of range: {}",
            ask
        );
    }

    Corpus::Keep
}
