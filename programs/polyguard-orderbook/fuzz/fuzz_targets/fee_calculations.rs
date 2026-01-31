//! Fuzz target for fee calculations
//!
//! Tests fee arithmetic for:
//! - Maker/taker fee calculations
//! - Protocol fee splits
//! - Fee accumulation over many trades
//! - Overflow protection in fee math
//! - Rounding behavior

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::{fuzz_target, Corpus};
use log::info;
use std::sync::Once;

const PRICE_SCALE: u64 = 10000;
const MAX_FEE_BPS: u64 = 1000; // 10% max fee
const MAX_QUANTITY: u64 = 1_000_000_000_000;

#[derive(Debug, Arbitrary, Clone)]
struct FeeData {
    scenarios: Vec<FeeScenario>,
}

#[derive(Debug, Arbitrary, Clone)]
struct FeeScenario {
    /// Trade quantity
    quantity: u64,
    /// Trade price in basis points (1-9999)
    price: u16,
    /// Maker fee in basis points (0-1000)
    maker_fee_bps: u16,
    /// Taker fee in basis points (0-1000)
    taker_fee_bps: u16,
    /// Protocol's share of fees (0-10000 = 0-100%)
    protocol_share_bps: u16,
}

#[derive(Debug, Clone)]
struct FeeResult {
    trade_cost: u64,
    maker_fee: u64,
    taker_fee: u64,
    protocol_fee: u64,
    referrer_fee: u64,
}

fn constrain_price(price: u16) -> u64 {
    let p = (price % 9998) + 1;
    p as u64
}

fn constrain_fee_bps(fee: u16) -> u64 {
    (fee as u64) % (MAX_FEE_BPS + 1)
}

fn constrain_quantity(quantity: u64) -> u64 {
    if quantity == 0 {
        1
    } else {
        quantity % MAX_QUANTITY + 1
    }
}

/// Calculate trade cost: quantity * price / PRICE_SCALE
fn calculate_cost(quantity: u64, price: u64) -> Option<u64> {
    (quantity as u128)
        .checked_mul(price as u128)?
        .checked_div(PRICE_SCALE as u128)
        .map(|v| v as u64)
}

/// Calculate fee: amount * fee_bps / PRICE_SCALE
fn calculate_fee(amount: u64, fee_bps: u64) -> Option<u64> {
    (amount as u128)
        .checked_mul(fee_bps as u128)?
        .checked_div(PRICE_SCALE as u128)
        .map(|v| v as u64)
}

/// Calculate protocol's share: fee * protocol_share_bps / PRICE_SCALE
fn calculate_protocol_share(fee: u64, share_bps: u64) -> Option<u64> {
    (fee as u128)
        .checked_mul(share_bps as u128)?
        .checked_div(PRICE_SCALE as u128)
        .map(|v| v as u64)
}

fn process_fee_scenario(scenario: &FeeScenario) -> Option<FeeResult> {
    let quantity = constrain_quantity(scenario.quantity);
    let price = constrain_price(scenario.price);
    let maker_fee_bps = constrain_fee_bps(scenario.maker_fee_bps);
    let taker_fee_bps = constrain_fee_bps(scenario.taker_fee_bps);
    let protocol_share_bps = (scenario.protocol_share_bps as u64).min(PRICE_SCALE);

    let trade_cost = calculate_cost(quantity, price)?;
    let maker_fee = calculate_fee(trade_cost, maker_fee_bps)?;
    let taker_fee = calculate_fee(trade_cost, taker_fee_bps)?;

    let total_fee = maker_fee.checked_add(taker_fee)?;
    let protocol_fee = calculate_protocol_share(total_fee, protocol_share_bps)?;
    let referrer_fee = total_fee.saturating_sub(protocol_fee);

    Some(FeeResult {
        trade_cost,
        maker_fee,
        taker_fee,
        protocol_fee,
        referrer_fee,
    })
}

fuzz_target!(|data: FeeData| -> Corpus {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = env_logger::try_init();
    });

    if data.scenarios.is_empty() || data.scenarios.len() > 100 {
        return Corpus::Reject;
    }

    run_fee_fuzz(data)
});

fn run_fee_fuzz(data: FeeData) -> Corpus {
    info!("Testing {} fee scenarios", data.scenarios.len());

    let mut total_protocol_fees: u128 = 0;
    let mut total_referrer_fees: u128 = 0;
    let mut total_volume: u128 = 0;

    for (i, scenario) in data.scenarios.iter().enumerate() {
        let quantity = constrain_quantity(scenario.quantity);
        let price = constrain_price(scenario.price);
        let maker_fee_bps = constrain_fee_bps(scenario.maker_fee_bps);
        let taker_fee_bps = constrain_fee_bps(scenario.taker_fee_bps);

        info!(
            "Scenario {}: qty={} price={} maker_fee={}bps taker_fee={}bps",
            i, quantity, price, maker_fee_bps, taker_fee_bps
        );

        let result = process_fee_scenario(scenario);
        assert!(result.is_some(), "Fee calculation should not overflow for constrained inputs");

        let result = result.unwrap();

        // Invariant 1: Fees never exceed trade cost
        assert!(
            result.maker_fee <= result.trade_cost,
            "Maker fee {} > trade cost {}",
            result.maker_fee,
            result.trade_cost
        );
        assert!(
            result.taker_fee <= result.trade_cost,
            "Taker fee {} > trade cost {}",
            result.taker_fee,
            result.trade_cost
        );

        // Invariant 2: Total fee = maker + taker
        let total_fee = result.maker_fee + result.taker_fee;
        assert!(
            total_fee <= result.trade_cost * 2,
            "Total fee unreasonable"
        );

        // Invariant 3: Protocol + referrer = total fee
        let fee_split = result.protocol_fee + result.referrer_fee;
        assert_eq!(
            fee_split, total_fee,
            "Fee split {} != total fee {}",
            fee_split, total_fee
        );

        // Invariant 4: Protocol fee <= total fee
        assert!(
            result.protocol_fee <= total_fee,
            "Protocol fee {} > total fee {}",
            result.protocol_fee,
            total_fee
        );

        // Invariant 5: Fee proportional to fee rate
        if maker_fee_bps == 100 && result.trade_cost >= 100 {
            // 1% fee should be approximately 1% of trade cost
            let expected = result.trade_cost / 100;
            let tolerance = 1;
            assert!(
                result.maker_fee >= expected.saturating_sub(tolerance)
                    && result.maker_fee <= expected + tolerance,
                "1% maker fee mismatch: got {}, expected ~{}",
                result.maker_fee,
                expected
            );
        }

        // Invariant 6: Zero fee rate means zero fee
        if maker_fee_bps == 0 {
            assert_eq!(result.maker_fee, 0, "Zero fee rate should mean zero fee");
        }
        if taker_fee_bps == 0 {
            assert_eq!(result.taker_fee, 0, "Zero fee rate should mean zero fee");
        }

        // Accumulate for aggregate checks
        total_protocol_fees += result.protocol_fee as u128;
        total_referrer_fees += result.referrer_fee as u128;
        total_volume += result.trade_cost as u128;
    }

    info!(
        "Aggregate: volume={} protocol_fees={} referrer_fees={}",
        total_volume, total_protocol_fees, total_referrer_fees
    );

    // Aggregate invariant: total fees reasonable compared to volume
    let total_fees = total_protocol_fees + total_referrer_fees;
    let max_fee_rate = MAX_FEE_BPS * 2; // maker + taker
    let max_expected_fees = total_volume * max_fee_rate as u128 / PRICE_SCALE as u128;
    assert!(
        total_fees <= max_expected_fees,
        "Total fees {} exceed maximum expected {}",
        total_fees,
        max_expected_fees
    );

    Corpus::Keep
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_fee() {
        // 1000 tokens at 50% price = 500 collateral
        // 1% fee = 5 collateral
        let cost = calculate_cost(1000, 5000).unwrap();
        assert_eq!(cost, 500);

        let fee = calculate_fee(cost, 100).unwrap();
        assert_eq!(fee, 5);
    }

    #[test]
    fn test_protocol_split() {
        // 100 total fee, 80% to protocol
        let total = 100u64;
        let protocol = calculate_protocol_share(total, 8000).unwrap();
        assert_eq!(protocol, 80);
    }

    #[test]
    fn test_zero_fee() {
        let cost = calculate_cost(1000, 5000).unwrap();
        let fee = calculate_fee(cost, 0).unwrap();
        assert_eq!(fee, 0);
    }

    #[test]
    fn test_max_fee() {
        // 10% fee (max)
        let cost = calculate_cost(1000, 5000).unwrap(); // 500
        let fee = calculate_fee(cost, 1000).unwrap(); // 10% = 50
        assert_eq!(fee, 50);
    }

    #[test]
    fn test_rounding() {
        // Small amounts may round to 0
        let cost = calculate_cost(1, 100).unwrap(); // 0 (rounds down)
        assert_eq!(cost, 0);

        // But larger amounts work
        let cost = calculate_cost(100, 100).unwrap(); // 1
        assert_eq!(cost, 1);
    }
}
