//! Fuzz target for arithmetic safety
//!
//! Tests for integer overflow, underflow, and division by zero
//! in financial calculations.

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::{fuzz_target, Corpus};
use log::info;
use std::sync::Once;

/// Price scale (10000 = 100%)
const PRICE_SCALE: u64 = 10000;

/// Maximum order quantity (prevents overflow in most calculations)
const MAX_QUANTITY: u64 = u64::MAX / PRICE_SCALE;

#[derive(Debug, Arbitrary, Clone)]
struct ArithmeticData {
    /// Quantities to test
    quantities: Vec<u64>,
    /// Prices to test (in basis points)
    prices: Vec<u16>,
    /// Operations to perform
    operations: Vec<Operation>,
}

#[derive(Debug, Arbitrary, Clone, Copy)]
enum Operation {
    /// Calculate cost: quantity * price / PRICE_SCALE
    CalculateCost,
    /// Calculate proceeds: quantity * (PRICE_SCALE - price) / PRICE_SCALE
    CalculateProceeds,
    /// Calculate fee: cost * fee_bps / PRICE_SCALE
    CalculateFee { fee_bps: u16 },
    /// Sum multiple costs
    SumCosts,
    /// Calculate shares from collateral: collateral * PRICE_SCALE / price
    CalculateShares,
}

fn constrain_price(price: u16) -> u64 {
    let p = (price % 9998) + 1; // 1-9999
    p as u64
}

fn constrain_quantity(quantity: u64) -> u64 {
    if quantity == 0 {
        1
    } else {
        quantity.min(MAX_QUANTITY)
    }
}

/// Safe cost calculation with overflow protection
fn calculate_cost_safe(quantity: u64, price: u64) -> Option<u64> {
    (quantity as u128)
        .checked_mul(price as u128)?
        .checked_div(PRICE_SCALE as u128)
        .map(|v| v as u64)
}

/// Safe proceeds calculation (for No outcome: collateral paid = quantity * (1 - price))
fn calculate_proceeds_safe(quantity: u64, price: u64) -> Option<u64> {
    let inverse_price = PRICE_SCALE.checked_sub(price)?;
    (quantity as u128)
        .checked_mul(inverse_price as u128)?
        .checked_div(PRICE_SCALE as u128)
        .map(|v| v as u64)
}

/// Safe fee calculation
fn calculate_fee_safe(cost: u64, fee_bps: u64) -> Option<u64> {
    (cost as u128)
        .checked_mul(fee_bps as u128)?
        .checked_div(PRICE_SCALE as u128)
        .map(|v| v as u64)
}

/// Safe shares calculation (inverse of cost)
fn calculate_shares_safe(collateral: u64, price: u64) -> Option<u64> {
    if price == 0 {
        return None;
    }
    (collateral as u128)
        .checked_mul(PRICE_SCALE as u128)?
        .checked_div(price as u128)
        .map(|v| v as u64)
}

fuzz_target!(|data: ArithmeticData| -> Corpus {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = env_logger::try_init();
    });

    if data.quantities.is_empty() || data.prices.is_empty() || data.operations.is_empty() {
        return Corpus::Reject;
    }

    run_arithmetic_fuzz(data)
});

fn run_arithmetic_fuzz(data: ArithmeticData) -> Corpus {
    let quantities: Vec<u64> = data
        .quantities
        .iter()
        .map(|q| constrain_quantity(*q))
        .collect();
    let prices: Vec<u64> = data
        .prices
        .iter()
        .map(|p| constrain_price(*p))
        .collect();

    info!(
        "Testing {} quantities x {} prices x {} operations",
        quantities.len(),
        prices.len(),
        data.operations.len()
    );

    for op in &data.operations {
        match op {
            Operation::CalculateCost => {
                for qty in &quantities {
                    for price in &prices {
                        let result = calculate_cost_safe(*qty, *price);

                        // Should never overflow for constrained inputs
                        assert!(
                            result.is_some(),
                            "Cost overflow: qty={}, price={}",
                            qty,
                            price
                        );

                        let cost = result.unwrap();

                        // Cost should never exceed quantity (at 100% price)
                        assert!(
                            cost <= *qty,
                            "Cost {} > qty {} at price {}",
                            cost,
                            qty,
                            price
                        );

                        // Cost should be proportional to price
                        if *price == PRICE_SCALE / 2 {
                            // At 50% price, cost should be ~half quantity
                            let expected = *qty / 2;
                            let tolerance = 1; // Allow rounding error
                            assert!(
                                cost >= expected.saturating_sub(tolerance)
                                    && cost <= expected + tolerance,
                                "50% cost mismatch: got {}, expected ~{}",
                                cost,
                                expected
                            );
                        }
                    }
                }
            }

            Operation::CalculateProceeds => {
                for qty in &quantities {
                    for price in &prices {
                        let result = calculate_proceeds_safe(*qty, *price);

                        // Should always succeed for valid prices
                        assert!(
                            result.is_some(),
                            "Proceeds calculation failed: qty={}, price={}",
                            qty,
                            price
                        );

                        let proceeds = result.unwrap();

                        // Proceeds should never exceed quantity
                        assert!(
                            proceeds <= *qty,
                            "Proceeds {} > qty {} at price {}",
                            proceeds,
                            qty,
                            price
                        );

                        // Cost + proceeds should approximately equal quantity
                        let cost = calculate_cost_safe(*qty, *price).unwrap();
                        let total = cost + proceeds;
                        let tolerance = 2; // Allow rounding errors

                        assert!(
                            total >= qty.saturating_sub(tolerance) && total <= *qty + tolerance,
                            "Cost {} + Proceeds {} != Qty {} (got {})",
                            cost,
                            proceeds,
                            qty,
                            total
                        );
                    }
                }
            }

            Operation::CalculateFee { fee_bps } => {
                let fee_bps = (*fee_bps as u64) % 1000; // Max 10% fee

                for qty in &quantities {
                    for price in &prices {
                        if let Some(cost) = calculate_cost_safe(*qty, *price) {
                            let fee = calculate_fee_safe(cost, fee_bps);

                            assert!(
                                fee.is_some(),
                                "Fee overflow: cost={}, fee_bps={}",
                                cost,
                                fee_bps
                            );

                            let f = fee.unwrap();

                            // Fee should never exceed cost
                            assert!(
                                f <= cost,
                                "Fee {} > cost {} with fee_bps={}",
                                f,
                                cost,
                                fee_bps
                            );

                            // Fee should be proportional
                            if fee_bps == 100 && cost >= 100 {
                                // 1% fee
                                let expected = cost / 100;
                                assert!(
                                    f >= expected.saturating_sub(1) && f <= expected + 1,
                                    "1% fee mismatch: got {}, expected ~{}",
                                    f,
                                    expected
                                );
                            }
                        }
                    }
                }
            }

            Operation::SumCosts => {
                let mut total: u128 = 0;

                for qty in &quantities {
                    for price in &prices {
                        if let Some(cost) = calculate_cost_safe(*qty, *price) {
                            total = total.saturating_add(cost as u128);
                        }
                    }
                }

                info!("Total sum of costs: {}", total);

                // Even large sums should be trackable
                assert!(total <= u128::MAX);
            }

            Operation::CalculateShares => {
                for qty in &quantities {
                    for price in &prices {
                        // Use qty as collateral
                        let shares = calculate_shares_safe(*qty, *price);

                        assert!(
                            shares.is_some(),
                            "Shares calculation failed: collateral={}, price={}",
                            qty,
                            price
                        );

                        let s = shares.unwrap();

                        // Shares should be >= collateral (since price <= 100%)
                        assert!(
                            s >= *qty,
                            "Shares {} < collateral {} at price {}",
                            s,
                            qty,
                            price
                        );

                        // Roundtrip: shares -> cost should approximately equal original collateral
                        if let Some(cost_back) = calculate_cost_safe(s, *price) {
                            let diff = if cost_back >= *qty {
                                cost_back - *qty
                            } else {
                                *qty - cost_back
                            };

                            // Allow some rounding error
                            let tolerance = (*qty / 1000).max(2);
                            assert!(
                                diff <= tolerance,
                                "Roundtrip error: {} -> {} -> {}, diff={}",
                                qty,
                                s,
                                cost_back,
                                diff
                            );
                        }
                    }
                }
            }
        }
    }

    Corpus::Keep
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_cost() {
        // 100 tokens at 50% = 50 collateral
        assert_eq!(calculate_cost_safe(100, 5000), Some(50));

        // 1000 tokens at 75% = 750 collateral
        assert_eq!(calculate_cost_safe(1000, 7500), Some(750));
    }

    #[test]
    fn test_proceeds() {
        // 100 tokens at 50% price: proceeds = 100 * (10000-5000) / 10000 = 50
        assert_eq!(calculate_proceeds_safe(100, 5000), Some(50));

        // Cost + proceeds = quantity
        let qty = 1000u64;
        let price = 7500u64;
        let cost = calculate_cost_safe(qty, price).unwrap();
        let proceeds = calculate_proceeds_safe(qty, price).unwrap();
        assert_eq!(cost + proceeds, qty);
    }

    #[test]
    fn test_edge_cases() {
        // Minimum price
        assert!(calculate_cost_safe(100, 1).is_some());

        // Maximum price
        assert!(calculate_cost_safe(100, 9999).is_some());

        // Large quantity
        assert!(calculate_cost_safe(MAX_QUANTITY, 5000).is_some());
    }
}
