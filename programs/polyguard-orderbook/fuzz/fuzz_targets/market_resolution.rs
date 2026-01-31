//! Fuzz target for market resolution logic
//!
//! Tests oracle price evaluation and threshold comparison:
//! - GreaterThan: price > threshold => YES
//! - GreaterThanOrEqual: price >= threshold => YES
//! - LessThan: price < threshold => YES
//! - LessThanOrEqual: price <= threshold => YES
//! - Equal: price == threshold => YES
//! - Boundary conditions at threshold

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::{fuzz_target, Corpus};
use log::info;
use std::sync::Once;

#[derive(Debug, Arbitrary, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum ComparisonOp {
    GreaterThan = 0,
    GreaterThanOrEqual = 1,
    LessThan = 2,
    LessThanOrEqual = 3,
    Equal = 4,
}

#[derive(Debug, Arbitrary, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum ExpectedOutcome {
    Yes = 1,
    No = 2,
}

#[derive(Debug, Arbitrary, Clone)]
struct ResolutionData {
    scenarios: Vec<ResolutionScenario>,
}

#[derive(Debug, Arbitrary, Clone)]
struct ResolutionScenario {
    /// Oracle price (i128 to handle Switchboard's 18 decimals)
    price: i128,
    /// Resolution threshold
    threshold: i128,
    /// Comparison operator
    comparison: ComparisonOp,
    /// Confidence interval (for staleness checks)
    confidence: u64,
    /// Maximum allowed confidence
    max_confidence: u64,
    /// Oracle slot
    oracle_slot: u64,
    /// Current slot
    current_slot: u64,
    /// Maximum staleness in slots
    max_staleness: u64,
}

#[derive(Debug, Clone)]
struct OraclePrice {
    price: i128,
    confidence: u64,
    slot: u64,
}

impl OraclePrice {
    fn is_stale(&self, current_slot: u64, max_staleness: u64) -> bool {
        current_slot.saturating_sub(self.slot) > max_staleness
    }
}

fn evaluate_threshold(price: i128, threshold: i128, op: ComparisonOp) -> ExpectedOutcome {
    let condition_met = match op {
        ComparisonOp::GreaterThan => price > threshold,
        ComparisonOp::GreaterThanOrEqual => price >= threshold,
        ComparisonOp::LessThan => price < threshold,
        ComparisonOp::LessThanOrEqual => price <= threshold,
        ComparisonOp::Equal => price == threshold,
    };

    if condition_met {
        ExpectedOutcome::Yes
    } else {
        ExpectedOutcome::No
    }
}

fuzz_target!(|data: ResolutionData| -> Corpus {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = env_logger::try_init();
    });

    if data.scenarios.is_empty() || data.scenarios.len() > 100 {
        return Corpus::Reject;
    }

    run_resolution_fuzz(data)
});

fn run_resolution_fuzz(data: ResolutionData) -> Corpus {
    info!("Testing {} resolution scenarios", data.scenarios.len());

    for (i, scenario) in data.scenarios.iter().enumerate() {
        let oracle_price = OraclePrice {
            price: scenario.price,
            confidence: scenario.confidence,
            slot: scenario.oracle_slot,
        };

        // Check staleness
        let is_stale = oracle_price.is_stale(scenario.current_slot, scenario.max_staleness);

        // Check confidence
        let confidence_exceeded =
            scenario.max_confidence > 0 && oracle_price.confidence > scenario.max_confidence;

        info!(
            "Scenario {}: price={} threshold={} op={:?} stale={} conf_exceeded={}",
            i, scenario.price, scenario.threshold, scenario.comparison, is_stale, confidence_exceeded
        );

        // Skip resolution if oracle is stale or confidence exceeded
        if is_stale || confidence_exceeded {
            // These would fail in the actual program
            continue;
        }

        // Evaluate threshold
        let outcome = evaluate_threshold(scenario.price, scenario.threshold, scenario.comparison);

        // Verify evaluation is deterministic
        let outcome2 = evaluate_threshold(scenario.price, scenario.threshold, scenario.comparison);
        assert_eq!(
            outcome, outcome2,
            "Non-deterministic evaluation"
        );

        // Verify inverse relationship for opposite operators
        match scenario.comparison {
            ComparisonOp::GreaterThan => {
                let inverse = evaluate_threshold(
                    scenario.price,
                    scenario.threshold,
                    ComparisonOp::LessThanOrEqual,
                );
                assert_ne!(
                    outcome, inverse,
                    "GT and LTE should be opposites"
                );
            }
            ComparisonOp::GreaterThanOrEqual => {
                let inverse = evaluate_threshold(
                    scenario.price,
                    scenario.threshold,
                    ComparisonOp::LessThan,
                );
                assert_ne!(
                    outcome, inverse,
                    "GTE and LT should be opposites"
                );
            }
            ComparisonOp::LessThan => {
                let inverse = evaluate_threshold(
                    scenario.price,
                    scenario.threshold,
                    ComparisonOp::GreaterThanOrEqual,
                );
                assert_ne!(
                    outcome, inverse,
                    "LT and GTE should be opposites"
                );
            }
            ComparisonOp::LessThanOrEqual => {
                let inverse = evaluate_threshold(
                    scenario.price,
                    scenario.threshold,
                    ComparisonOp::GreaterThan,
                );
                assert_ne!(
                    outcome, inverse,
                    "LTE and GT should be opposites"
                );
            }
            ComparisonOp::Equal => {
                // Equal has no simple inverse
            }
        }

        // Test boundary conditions
        if scenario.price == scenario.threshold {
            match scenario.comparison {
                ComparisonOp::GreaterThan => {
                    assert_eq!(outcome, ExpectedOutcome::No, "price == threshold should not be GT");
                }
                ComparisonOp::GreaterThanOrEqual => {
                    assert_eq!(outcome, ExpectedOutcome::Yes, "price == threshold should be GTE");
                }
                ComparisonOp::LessThan => {
                    assert_eq!(outcome, ExpectedOutcome::No, "price == threshold should not be LT");
                }
                ComparisonOp::LessThanOrEqual => {
                    assert_eq!(outcome, ExpectedOutcome::Yes, "price == threshold should be LTE");
                }
                ComparisonOp::Equal => {
                    assert_eq!(outcome, ExpectedOutcome::Yes, "price == threshold should be EQ");
                }
            }
        }

        // Test staleness calculation doesn't overflow
        let _stale_diff = scenario.current_slot.saturating_sub(scenario.oracle_slot);
    }

    Corpus::Keep
}

#[cfg(test)]
mod tests {
    use super::*;

    const BTC_50K: i128 = 50_000_000_000_000_000_000_000; // $50,000 with 18 decimals

    #[test]
    fn test_greater_than() {
        // BTC > $50,000 market
        let price_55k = 55_000_000_000_000_000_000_000i128;
        let price_45k = 45_000_000_000_000_000_000_000i128;

        assert_eq!(
            evaluate_threshold(price_55k, BTC_50K, ComparisonOp::GreaterThan),
            ExpectedOutcome::Yes
        );
        assert_eq!(
            evaluate_threshold(price_45k, BTC_50K, ComparisonOp::GreaterThan),
            ExpectedOutcome::No
        );
        // Exactly at threshold
        assert_eq!(
            evaluate_threshold(BTC_50K, BTC_50K, ComparisonOp::GreaterThan),
            ExpectedOutcome::No
        );
    }

    #[test]
    fn test_less_than() {
        // Temperature < 100F market
        let threshold = 100i128;
        assert_eq!(
            evaluate_threshold(99, threshold, ComparisonOp::LessThan),
            ExpectedOutcome::Yes
        );
        assert_eq!(
            evaluate_threshold(101, threshold, ComparisonOp::LessThan),
            ExpectedOutcome::No
        );
    }

    #[test]
    fn test_staleness() {
        let oracle = OraclePrice {
            price: 0,
            confidence: 0,
            slot: 1000,
        };

        // Within staleness window
        assert!(!oracle.is_stale(1100, 150));
        // Outside staleness window
        assert!(oracle.is_stale(1200, 150));
    }

    #[test]
    fn test_i128_extremes() {
        let max = i128::MAX;
        let min = i128::MIN;

        // Should not panic on extreme values
        let _ = evaluate_threshold(max, 0, ComparisonOp::GreaterThan);
        let _ = evaluate_threshold(min, 0, ComparisonOp::LessThan);
        let _ = evaluate_threshold(max, min, ComparisonOp::GreaterThan);
    }
}
