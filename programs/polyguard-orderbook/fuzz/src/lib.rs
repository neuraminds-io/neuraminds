//! Fuzz testing utilities for Polyguard Orderbook
//!
//! Provides state management and test infrastructure for fuzzing
//! order placement, matching, and settlement operations.

use anchor_lang::prelude::Pubkey;
use arbitrary::Arbitrary;
use std::collections::HashMap;

/// Price scale (10000 = 100%)
pub const PRICE_SCALE: u64 = 10000;

/// Initial balance for fuzz users
pub const INITIAL_BALANCE: u64 = 1_000_000_000;

/// Maximum users in fuzz tests
pub const MAX_USERS: usize = 8;

/// User identifier for fuzz tests
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Arbitrary)]
pub struct UserId(pub u8);

impl From<UserId> for usize {
    fn from(id: UserId) -> Self {
        (id.0 % MAX_USERS as u8) as usize
    }
}

/// Order side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Arbitrary)]
pub enum Side {
    Buy,
    Sell,
}

/// Market outcome
#[derive(Debug, Clone, Copy, PartialEq, Eq, Arbitrary)]
pub enum Outcome {
    Yes,
    No,
}

/// Order type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Arbitrary)]
pub enum OrderType {
    Limit,
    Market,
    PostOnly,
    ImmediateOrCancel,
    FillOrKill,
}

/// Simulated user state
#[derive(Debug, Clone)]
pub struct FuzzUser {
    pub pubkey: Pubkey,
    pub collateral_balance: u64,
    pub yes_balance: u64,
    pub no_balance: u64,
    pub collateral_locked: u64,
    pub yes_locked: u64,
    pub no_locked: u64,
    pub open_orders: Vec<FuzzOrder>,
}

impl FuzzUser {
    pub fn new(seed: u8) -> Self {
        Self {
            pubkey: Pubkey::new_from_array([seed; 32]),
            collateral_balance: INITIAL_BALANCE,
            yes_balance: 0,
            no_balance: 0,
            collateral_locked: 0,
            yes_locked: 0,
            no_locked: 0,
            open_orders: Vec::new(),
        }
    }

    pub fn free_collateral(&self) -> u64 {
        self.collateral_balance.saturating_sub(self.collateral_locked)
    }

    pub fn free_yes(&self) -> u64 {
        self.yes_balance.saturating_sub(self.yes_locked)
    }

    pub fn free_no(&self) -> u64 {
        self.no_balance.saturating_sub(self.no_locked)
    }
}

/// Simulated order
#[derive(Debug, Clone)]
pub struct FuzzOrder {
    pub id: u64,
    pub side: Side,
    pub outcome: Outcome,
    pub price: u64,
    pub quantity: u64,
    pub filled: u64,
}

impl FuzzOrder {
    pub fn remaining(&self) -> u64 {
        self.quantity.saturating_sub(self.filled)
    }
}

/// Orderbook state for fuzzing
#[derive(Debug, Clone)]
pub struct FuzzOrderbook {
    /// Bids: (price, quantity, user_id, order_id)
    pub yes_bids: Vec<(u64, u64, UserId, u64)>,
    pub yes_asks: Vec<(u64, u64, UserId, u64)>,
    pub no_bids: Vec<(u64, u64, UserId, u64)>,
    pub no_asks: Vec<(u64, u64, UserId, u64)>,
    pub next_order_id: u64,
}

impl FuzzOrderbook {
    pub fn new() -> Self {
        Self {
            yes_bids: Vec::new(),
            yes_asks: Vec::new(),
            no_bids: Vec::new(),
            no_asks: Vec::new(),
            next_order_id: 1,
        }
    }

    pub fn get_book_mut(
        &mut self,
        outcome: Outcome,
        side: Side,
    ) -> &mut Vec<(u64, u64, UserId, u64)> {
        match (outcome, side) {
            (Outcome::Yes, Side::Buy) => &mut self.yes_bids,
            (Outcome::Yes, Side::Sell) => &mut self.yes_asks,
            (Outcome::No, Side::Buy) => &mut self.no_bids,
            (Outcome::No, Side::Sell) => &mut self.no_asks,
        }
    }

    pub fn best_bid(&self, outcome: Outcome) -> Option<u64> {
        let book = match outcome {
            Outcome::Yes => &self.yes_bids,
            Outcome::No => &self.no_bids,
        };
        book.iter().map(|(p, _, _, _)| *p).max()
    }

    pub fn best_ask(&self, outcome: Outcome) -> Option<u64> {
        let book = match outcome {
            Outcome::Yes => &self.yes_asks,
            Outcome::No => &self.no_asks,
        };
        book.iter().map(|(p, _, _, _)| *p).min()
    }
}

impl Default for FuzzOrderbook {
    fn default() -> Self {
        Self::new()
    }
}

/// Fuzz context manages simulated state
pub struct FuzzContext {
    pub users: HashMap<UserId, FuzzUser>,
    pub orderbook: FuzzOrderbook,
    pub market_vault: u64,
}

impl FuzzContext {
    pub fn new() -> Self {
        let mut users = HashMap::new();
        for i in 0..MAX_USERS {
            users.insert(UserId(i as u8), FuzzUser::new(i as u8));
        }

        Self {
            users,
            orderbook: FuzzOrderbook::new(),
            market_vault: 0,
        }
    }

    /// Calculate cost for a buy order
    pub fn calculate_cost(quantity: u64, price: u64) -> u64 {
        (quantity as u128)
            .checked_mul(price as u128)
            .and_then(|v| v.checked_div(PRICE_SCALE as u128))
            .unwrap_or(u64::MAX as u128) as u64
    }

    /// Place an order with validation
    pub fn place_order(
        &mut self,
        user_id: UserId,
        side: Side,
        outcome: Outcome,
        price: u64,
        quantity: u64,
        order_type: OrderType,
    ) -> Result<PlaceOrderResult, FuzzError> {
        // Validate price
        if price < 1 || price > 9999 {
            return Err(FuzzError::InvalidPrice);
        }

        // Validate quantity
        if quantity == 0 {
            return Err(FuzzError::InvalidQuantity);
        }

        let user = self.users.get_mut(&user_id).ok_or(FuzzError::UserNotFound)?;

        // Check sufficient funds
        match side {
            Side::Buy => {
                let cost = Self::calculate_cost(quantity, price);
                if cost > user.free_collateral() {
                    return Err(FuzzError::InsufficientCollateral);
                }
            }
            Side::Sell => {
                let free = match outcome {
                    Outcome::Yes => user.free_yes(),
                    Outcome::No => user.free_no(),
                };
                if quantity > free {
                    return Err(FuzzError::InsufficientBalance);
                }
            }
        }

        // Check for matches (simplified)
        let opposing_book = match (outcome, side) {
            (Outcome::Yes, Side::Buy) => &self.orderbook.yes_asks,
            (Outcome::Yes, Side::Sell) => &self.orderbook.yes_bids,
            (Outcome::No, Side::Buy) => &self.orderbook.no_asks,
            (Outcome::No, Side::Sell) => &self.orderbook.no_bids,
        };

        let would_match = opposing_book.iter().any(|(maker_price, _, _, _)| {
            match side {
                Side::Buy => *maker_price <= price,
                Side::Sell => *maker_price >= price,
            }
        });

        // PostOnly fails if would match
        if order_type == OrderType::PostOnly && would_match {
            return Ok(PlaceOrderResult {
                order_id: None,
                filled: 0,
                posted: 0,
            });
        }

        // Lock funds
        let user = self.users.get_mut(&user_id).unwrap();
        let order_id = self.orderbook.next_order_id;
        self.orderbook.next_order_id += 1;

        match side {
            Side::Buy => {
                let cost = Self::calculate_cost(quantity, price);
                user.collateral_locked += cost;
            }
            Side::Sell => match outcome {
                Outcome::Yes => user.yes_locked += quantity,
                Outcome::No => user.no_locked += quantity,
            },
        }

        // Add to orderbook
        let book = self.orderbook.get_book_mut(outcome, side);
        book.push((price, quantity, user_id, order_id));

        // Sort (bids descending, asks ascending)
        match side {
            Side::Buy => book.sort_by(|a, b| b.0.cmp(&a.0)),
            Side::Sell => book.sort_by(|a, b| a.0.cmp(&b.0)),
        }

        user.open_orders.push(FuzzOrder {
            id: order_id,
            side,
            outcome,
            price,
            quantity,
            filled: 0,
        });

        Ok(PlaceOrderResult {
            order_id: Some(order_id),
            filled: 0,
            posted: quantity,
        })
    }

    /// Cancel an order
    pub fn cancel_order(&mut self, user_id: UserId, order_id: u64) -> Result<(), FuzzError> {
        let user = self.users.get_mut(&user_id).ok_or(FuzzError::UserNotFound)?;

        let order_idx = user
            .open_orders
            .iter()
            .position(|o| o.id == order_id)
            .ok_or(FuzzError::OrderNotFound)?;

        let order = user.open_orders.remove(order_idx);
        let remaining = order.remaining();

        // Unlock funds
        match order.side {
            Side::Buy => {
                let cost = Self::calculate_cost(remaining, order.price);
                user.collateral_locked = user.collateral_locked.saturating_sub(cost);
            }
            Side::Sell => match order.outcome {
                Outcome::Yes => user.yes_locked = user.yes_locked.saturating_sub(remaining),
                Outcome::No => user.no_locked = user.no_locked.saturating_sub(remaining),
            },
        }

        // Remove from orderbook
        let book = self.orderbook.get_book_mut(order.outcome, order.side);
        book.retain(|(_, _, _, id)| *id != order_id);

        Ok(())
    }

    /// Validate invariants
    pub fn validate_invariants(&self) -> Result<(), FuzzError> {
        for (user_id, user) in &self.users {
            // Check locked doesn't exceed balance
            if user.collateral_locked > user.collateral_balance {
                return Err(FuzzError::InvariantViolation(format!(
                    "User {:?} collateral locked ({}) > balance ({})",
                    user_id, user.collateral_locked, user.collateral_balance
                )));
            }

            if user.yes_locked > user.yes_balance {
                return Err(FuzzError::InvariantViolation(format!(
                    "User {:?} yes locked ({}) > balance ({})",
                    user_id, user.yes_locked, user.yes_balance
                )));
            }

            if user.no_locked > user.no_balance {
                return Err(FuzzError::InvariantViolation(format!(
                    "User {:?} no locked ({}) > balance ({})",
                    user_id, user.no_locked, user.no_balance
                )));
            }

            // Check order count matches
            let orderbook_orders: usize = self
                .orderbook
                .yes_bids
                .iter()
                .chain(self.orderbook.yes_asks.iter())
                .chain(self.orderbook.no_bids.iter())
                .chain(self.orderbook.no_asks.iter())
                .filter(|(_, _, uid, _)| uid == user_id)
                .count();

            if orderbook_orders != user.open_orders.len() {
                return Err(FuzzError::InvariantViolation(format!(
                    "User {:?} order count mismatch: orderbook={}, user={}",
                    user_id,
                    orderbook_orders,
                    user.open_orders.len()
                )));
            }
        }

        Ok(())
    }
}

impl Default for FuzzContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of placing an order
#[derive(Debug)]
pub struct PlaceOrderResult {
    pub order_id: Option<u64>,
    pub filled: u64,
    pub posted: u64,
}

/// Fuzz errors
#[derive(Debug, Clone)]
pub enum FuzzError {
    InvalidPrice,
    InvalidQuantity,
    InsufficientCollateral,
    InsufficientBalance,
    UserNotFound,
    OrderNotFound,
    InvariantViolation(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzz_context_initialization() {
        let ctx = FuzzContext::new();
        assert_eq!(ctx.users.len(), MAX_USERS);

        for user in ctx.users.values() {
            assert_eq!(user.collateral_balance, INITIAL_BALANCE);
            assert_eq!(user.yes_balance, 0);
            assert_eq!(user.no_balance, 0);
        }
    }

    #[test]
    fn test_place_order_validation() {
        let mut ctx = FuzzContext::new();
        let user_id = UserId(0);

        // Invalid price
        assert!(matches!(
            ctx.place_order(user_id, Side::Buy, Outcome::Yes, 0, 100, OrderType::Limit),
            Err(FuzzError::InvalidPrice)
        ));

        assert!(matches!(
            ctx.place_order(user_id, Side::Buy, Outcome::Yes, 10000, 100, OrderType::Limit),
            Err(FuzzError::InvalidPrice)
        ));

        // Invalid quantity
        assert!(matches!(
            ctx.place_order(user_id, Side::Buy, Outcome::Yes, 5000, 0, OrderType::Limit),
            Err(FuzzError::InvalidQuantity)
        ));

        // Valid order
        let result = ctx
            .place_order(user_id, Side::Buy, Outcome::Yes, 5000, 100, OrderType::Limit)
            .unwrap();
        assert!(result.order_id.is_some());
    }

    #[test]
    fn test_invariants() {
        let mut ctx = FuzzContext::new();
        let user_id = UserId(0);

        ctx.place_order(user_id, Side::Buy, Outcome::Yes, 5000, 100, OrderType::Limit)
            .unwrap();

        assert!(ctx.validate_invariants().is_ok());
    }
}
