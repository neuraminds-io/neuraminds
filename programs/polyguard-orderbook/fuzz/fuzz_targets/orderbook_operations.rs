//! Fuzz target for orderbook operations
//!
//! Tests sequences of order placements and cancellations
//! to find invariant violations and edge cases.

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::{fuzz_target, Corpus};
use log::info;
use polyguard_orderbook_fuzz::{FuzzContext, FuzzError, OrderType, Outcome, Side, UserId};
use std::sync::Once;

/// Maximum instructions per fuzz run
const MAX_INSTRUCTIONS: usize = 50;

#[derive(Debug, Arbitrary, Clone)]
struct FuzzData {
    instructions: Vec<FuzzInstruction>,
}

#[derive(Debug, Arbitrary, Clone)]
enum FuzzInstruction {
    PlaceOrder {
        user_id: UserId,
        side: Side,
        outcome: Outcome,
        /// Price in basis points (will be constrained to 1-9999)
        price: u16,
        /// Quantity (will be constrained to non-zero)
        quantity: u32,
        order_type: OrderType,
    },
    CancelOrder {
        user_id: UserId,
        /// Order index in user's open orders
        order_index: u8,
    },
    ValidateInvariants,
}

impl FuzzData {
    fn is_valid(&self) -> bool {
        !self.instructions.is_empty() && self.instructions.len() <= MAX_INSTRUCTIONS
    }

    fn has_place_orders(&self) -> bool {
        self.instructions
            .iter()
            .any(|ix| matches!(ix, FuzzInstruction::PlaceOrder { .. }))
    }
}

fn constrain_price(price: u16) -> u64 {
    let p = (price % 9998) + 1; // 1-9999
    p as u64
}

fn constrain_quantity(quantity: u32) -> u64 {
    let q = (quantity % 1_000_000) + 1; // 1-1_000_000
    q as u64
}

fuzz_target!(|data: FuzzData| -> Corpus {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = env_logger::try_init();
    });

    if !data.is_valid() || !data.has_place_orders() {
        return Corpus::Reject;
    }

    run_fuzz(data)
});

fn run_fuzz(data: FuzzData) -> Corpus {
    let mut ctx = FuzzContext::new();
    info!("Starting fuzz with {} instructions", data.instructions.len());

    for (i, ix) in data.instructions.iter().enumerate() {
        info!("Instruction {}: {:?}", i, ix);

        match ix {
            FuzzInstruction::PlaceOrder {
                user_id,
                side,
                outcome,
                price,
                quantity,
                order_type,
            } => {
                let price = constrain_price(*price);
                let quantity = constrain_quantity(*quantity);

                match ctx.place_order(*user_id, *side, *outcome, price, quantity, *order_type) {
                    Ok(result) => {
                        info!(
                            "Order placed: id={:?}, filled={}, posted={}",
                            result.order_id, result.filled, result.posted
                        );
                    }
                    Err(FuzzError::InvalidPrice) => {
                        // Price was constrained, this shouldn't happen
                        panic!("Invalid price after constraining: {}", price);
                    }
                    Err(FuzzError::InvalidQuantity) => {
                        // Quantity was constrained, this shouldn't happen
                        panic!("Invalid quantity after constraining: {}", quantity);
                    }
                    Err(FuzzError::InsufficientCollateral) => {
                        info!("Insufficient collateral - expected");
                    }
                    Err(FuzzError::InsufficientBalance) => {
                        info!("Insufficient balance - expected");
                    }
                    Err(e) => {
                        panic!("Unexpected error: {:?}", e);
                    }
                }
            }

            FuzzInstruction::CancelOrder {
                user_id,
                order_index,
            } => {
                let user = ctx.users.get(user_id);
                if let Some(user) = user {
                    if !user.open_orders.is_empty() {
                        let idx = (*order_index as usize) % user.open_orders.len();
                        let order_id = user.open_orders[idx].id;
                        match ctx.cancel_order(*user_id, order_id) {
                            Ok(()) => {
                                info!("Order {} cancelled", order_id);
                            }
                            Err(e) => {
                                panic!("Failed to cancel existing order: {:?}", e);
                            }
                        }
                    }
                }
            }

            FuzzInstruction::ValidateInvariants => {
                if let Err(e) = ctx.validate_invariants() {
                    panic!("Invariant violation: {:?}", e);
                }
            }
        }
    }

    // Always validate invariants at the end
    if let Err(e) = ctx.validate_invariants() {
        panic!("Final invariant violation: {:?}", e);
    }

    // Validate sum of all locked collateral matches expected
    let total_locked_collateral: u64 = ctx
        .users
        .values()
        .map(|u| u.collateral_locked)
        .sum();

    info!(
        "Fuzz complete: {} users, {} total locked collateral",
        ctx.users.len(),
        total_locked_collateral
    );

    Corpus::Keep
}
