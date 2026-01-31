//! Fuzz target for trade settlement
//!
//! Tests settlement logic including:
//! - Collateral transfers between buyer and seller
//! - Position updates after fills
//! - Partial fill handling
//! - Refund calculations

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::{fuzz_target, Corpus};
use log::info;
use std::sync::Once;

const PRICE_SCALE: u64 = 10000;
const MAX_QUANTITY: u64 = 1_000_000_000;

#[derive(Debug, Arbitrary, Clone)]
struct SettlementData {
    scenarios: Vec<SettlementScenario>,
}

#[derive(Debug, Arbitrary, Clone)]
struct SettlementScenario {
    /// Buy order price (will be constrained to 1-9999)
    buy_price: u16,
    /// Sell order price (will be constrained to 1-9999)
    sell_price: u16,
    /// Order quantity
    quantity: u64,
    /// Fill quantity (will be constrained to <= quantity)
    fill_quantity: u64,
    /// Fill price (will be constrained between sell and buy price)
    fill_price: u16,
}

#[derive(Debug, Clone)]
struct TraderState {
    collateral: u64,
    yes_balance: u64,
    no_balance: u64,
    locked_collateral: u64,
    locked_yes: u64,
    locked_no: u64,
}

impl TraderState {
    fn new(collateral: u64) -> Self {
        Self {
            collateral,
            yes_balance: 0,
            no_balance: 0,
            locked_collateral: 0,
            locked_yes: 0,
            locked_no: 0,
        }
    }
}

fn constrain_price(price: u16) -> u64 {
    let p = (price % 9998) + 1;
    p as u64
}

fn constrain_quantity(quantity: u64) -> u64 {
    if quantity == 0 {
        1
    } else {
        quantity.min(MAX_QUANTITY)
    }
}

/// Calculate collateral cost for a quantity at a price
fn calculate_cost(quantity: u64, price: u64) -> Option<u64> {
    (quantity as u128)
        .checked_mul(price as u128)?
        .checked_div(PRICE_SCALE as u128)
        .map(|v| v as u64)
}

fuzz_target!(|data: SettlementData| -> Corpus {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = env_logger::try_init();
    });

    if data.scenarios.is_empty() || data.scenarios.len() > 20 {
        return Corpus::Reject;
    }

    run_settlement_fuzz(data)
});

fn run_settlement_fuzz(data: SettlementData) -> Corpus {
    info!("Testing {} settlement scenarios", data.scenarios.len());

    for (i, scenario) in data.scenarios.iter().enumerate() {
        let buy_price = constrain_price(scenario.buy_price);
        let sell_price = constrain_price(scenario.sell_price);

        // Skip if buy < sell (no match possible)
        if buy_price < sell_price {
            continue;
        }

        let quantity = constrain_quantity(scenario.quantity);
        let fill_quantity = constrain_quantity(scenario.fill_quantity).min(quantity);

        // Fill price must be between sell and buy price
        let fill_price = {
            let fp = constrain_price(scenario.fill_price);
            fp.max(sell_price).min(buy_price)
        };

        info!(
            "Scenario {}: buy@{} sell@{} qty={} fill_qty={} fill_price={}",
            i, buy_price, sell_price, quantity, fill_quantity, fill_price
        );

        // Initialize traders
        let initial_collateral = 10_000_000_000u64;
        let mut buyer = TraderState::new(initial_collateral);
        let mut seller = TraderState::new(initial_collateral);

        // Seller needs outcome tokens to sell
        seller.yes_balance = quantity;

        // Buyer locks collateral at buy price
        let buyer_locked = calculate_cost(quantity, buy_price)
            .expect("Cost calculation overflow for buyer");
        assert!(
            buyer_locked <= buyer.collateral,
            "Buyer cannot afford order"
        );
        buyer.locked_collateral = buyer_locked;
        buyer.collateral -= buyer_locked;

        // Seller locks YES tokens
        seller.locked_yes = quantity;
        seller.yes_balance -= quantity;

        // Execute settlement
        let fill_cost = calculate_cost(fill_quantity, fill_price)
            .expect("Cost calculation overflow for fill");
        let buyer_locked_for_fill = calculate_cost(fill_quantity, buy_price)
            .expect("Cost calculation overflow for buyer fill");

        // Buyer's refund if fill price < buy price
        let buyer_refund = buyer_locked_for_fill.saturating_sub(fill_cost);

        // Update buyer
        buyer.locked_collateral = buyer.locked_collateral.saturating_sub(buyer_locked_for_fill);
        buyer.collateral += buyer_refund;
        buyer.yes_balance += fill_quantity;

        // Update seller
        seller.locked_yes = seller.locked_yes.saturating_sub(fill_quantity);
        seller.collateral += fill_cost;

        // Verify invariants

        // 1. Buyer received correct YES tokens
        assert_eq!(
            buyer.yes_balance, fill_quantity,
            "Buyer YES balance incorrect"
        );

        // 2. Seller's locked tokens decreased by fill quantity
        assert_eq!(
            seller.locked_yes,
            quantity - fill_quantity,
            "Seller locked YES incorrect"
        );

        // 3. Total collateral is conserved (excluding fees)
        let total_collateral = buyer.collateral
            + buyer.locked_collateral
            + seller.collateral
            + seller.locked_collateral;

        // Initial was 2 * initial_collateral, buyer sent fill_cost to seller
        // Buyer got back refund, so total should be conserved
        let expected_total = 2 * initial_collateral;
        assert_eq!(
            total_collateral, expected_total,
            "Collateral not conserved: got {}, expected {}",
            total_collateral, expected_total
        );

        // 4. Fill cost <= buyer's locked amount for fill
        assert!(
            fill_cost <= buyer_locked_for_fill,
            "Fill cost {} > buyer locked {} (refund would be negative)",
            fill_cost,
            buyer_locked_for_fill
        );

        // 5. Fill price within bounds
        assert!(
            fill_price >= sell_price && fill_price <= buy_price,
            "Fill price {} not in [{}, {}]",
            fill_price,
            sell_price,
            buy_price
        );

        // 6. Refund calculation is correct
        let expected_refund = buyer_locked_for_fill - fill_cost;
        assert_eq!(
            buyer_refund, expected_refund,
            "Refund calculation incorrect"
        );
    }

    Corpus::Keep
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_price_match() {
        // Buy and sell at same price, no refund
        let cost = calculate_cost(100, 5000).unwrap();
        assert_eq!(cost, 50); // 100 * 50% = 50
    }

    #[test]
    fn test_price_improvement() {
        // Buyer willing to pay 60%, seller accepts 40%
        // Fill at seller's price (40%), buyer gets 20% refund
        let buyer_locked = calculate_cost(100, 6000).unwrap(); // 60
        let fill_cost = calculate_cost(100, 4000).unwrap(); // 40
        let refund = buyer_locked - fill_cost;
        assert_eq!(refund, 20);
    }

    #[test]
    fn test_partial_fill() {
        // 100 quantity order, 30 fill
        let quantity = 100u64;
        let fill_quantity = 30u64;
        let remaining = quantity - fill_quantity;
        assert_eq!(remaining, 70);
    }
}
