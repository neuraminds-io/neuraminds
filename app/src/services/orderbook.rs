use std::collections::{BTreeMap, HashMap};
use std::sync::RwLock;
use chrono::Utc;
use log::info;

use crate::models::{Order, OrderSide, Outcome, OrderBookLevel, MatchedTrade};
use super::database::OrderBookEntry;

/// In-memory order book for fast matching
/// In production, this would be backed by Redis for persistence and horizontal scaling
pub struct OrderBookService {
    /// Order books by market_id -> outcome -> side -> price -> orders
    books: RwLock<HashMap<String, MarketOrderBook>>,
}

struct MarketOrderBook {
    /// YES outcome order book
    yes: OutcomeOrderBook,
    /// NO outcome order book
    no: OutcomeOrderBook,
}

struct OutcomeOrderBook {
    /// Bids sorted by price descending (highest first)
    bids: BTreeMap<u16, Vec<OrderEntry>>,
    /// Asks sorted by price ascending (lowest first)
    asks: BTreeMap<u16, Vec<OrderEntry>>,
}

#[derive(Clone)]
struct OrderEntry {
    order_id: String,
    on_chain_id: u64,
    owner: String,
    #[allow(dead_code)]
    price_bps: u16,
    #[allow(dead_code)]
    quantity: u64,
    remaining: u64,
    #[allow(dead_code)]
    timestamp: i64,
}

impl OrderBookService {
    pub fn new() -> Self {
        Self {
            books: RwLock::new(HashMap::new()),
        }
    }

    /// Add an order to the order book and attempt to match
    pub fn add_order(&self, order: &Order) -> Vec<MatchedTrade> {
        let mut books = self.books.write().unwrap();

        // Get or create market order book
        let market_book = books
            .entry(order.market_id.clone())
            .or_insert_with(|| MarketOrderBook {
                yes: OutcomeOrderBook {
                    bids: BTreeMap::new(),
                    asks: BTreeMap::new(),
                },
                no: OutcomeOrderBook {
                    bids: BTreeMap::new(),
                    asks: BTreeMap::new(),
                },
            });

        let outcome_book = match order.outcome {
            Outcome::Yes => &mut market_book.yes,
            Outcome::No => &mut market_book.no,
        };

        let entry = OrderEntry {
            order_id: order.id.clone(),
            on_chain_id: order.order_id,
            owner: order.owner.clone(),
            price_bps: order.price_bps,
            quantity: order.quantity,
            remaining: order.remaining_quantity,
            timestamp: Utc::now().timestamp(),
        };

        // Try to match against existing orders
        let matches = self.match_order(outcome_book, &entry, order.side);

        // If there's remaining quantity, add to book
        if entry.remaining > 0 {
            match order.side {
                OrderSide::Buy => {
                    outcome_book.bids
                        .entry(order.price_bps)
                        .or_insert_with(Vec::new)
                        .push(entry);
                }
                OrderSide::Sell => {
                    outcome_book.asks
                        .entry(order.price_bps)
                        .or_insert_with(Vec::new)
                        .push(entry);
                }
            }
        }

        matches
    }

    /// Match an incoming order against the book
    fn match_order(
        &self,
        book: &mut OutcomeOrderBook,
        order: &OrderEntry,
        side: OrderSide,
    ) -> Vec<MatchedTrade> {
        let mut matches = Vec::new();
        let mut remaining = order.remaining;

        match side {
            OrderSide::Buy => {
                // Match against asks (sellers)
                // Get asks at or below our buy price
                let matching_prices: Vec<u16> = book.asks
                    .range(..=order.price_bps)
                    .map(|(p, _)| *p)
                    .collect();

                for price in matching_prices {
                    if remaining == 0 {
                        break;
                    }

                    if let Some(asks) = book.asks.get_mut(&price) {
                        let mut i = 0;
                        while i < asks.len() && remaining > 0 {
                            let ask = &mut asks[i];
                            let fill_qty = remaining.min(ask.remaining);
                            let fill_price = price; // Price-time priority: use maker's price

                            matches.push(MatchedTrade {
                                buy_order_id: order.on_chain_id,
                                sell_order_id: ask.on_chain_id,
                                market_id: String::new(), // Would be set by caller
                                outcome: Outcome::Yes, // Would be set by caller
                                fill_price_bps: fill_price,
                                fill_quantity: fill_qty,
                                buyer: order.owner.clone(),
                                seller: ask.owner.clone(),
                            });

                            remaining -= fill_qty;
                            ask.remaining -= fill_qty;

                            if ask.remaining == 0 {
                                asks.remove(i);
                            } else {
                                i += 1;
                            }
                        }
                    }

                    // Remove empty price level
                    if book.asks.get(&price).map_or(false, |v| v.is_empty()) {
                        book.asks.remove(&price);
                    }
                }
            }
            OrderSide::Sell => {
                // Match against bids (buyers)
                // Get bids at or above our sell price
                let matching_prices: Vec<u16> = book.bids
                    .range(order.price_bps..)
                    .map(|(p, _)| *p)
                    .rev() // Start with highest bid
                    .collect();

                for price in matching_prices {
                    if remaining == 0 {
                        break;
                    }

                    if let Some(bids) = book.bids.get_mut(&price) {
                        let mut i = 0;
                        while i < bids.len() && remaining > 0 {
                            let bid = &mut bids[i];
                            let fill_qty = remaining.min(bid.remaining);
                            let fill_price = price;

                            matches.push(MatchedTrade {
                                buy_order_id: bid.on_chain_id,
                                sell_order_id: order.on_chain_id,
                                market_id: String::new(),
                                outcome: Outcome::Yes,
                                fill_price_bps: fill_price,
                                fill_quantity: fill_qty,
                                buyer: bid.owner.clone(),
                                seller: order.owner.clone(),
                            });

                            remaining -= fill_qty;
                            bid.remaining -= fill_qty;

                            if bid.remaining == 0 {
                                bids.remove(i);
                            } else {
                                i += 1;
                            }
                        }
                    }

                    if book.bids.get(&price).map_or(false, |v| v.is_empty()) {
                        book.bids.remove(&price);
                    }
                }
            }
        }

        if !matches.is_empty() {
            info!(
                "Matched {} trades for order {}",
                matches.len(),
                order.order_id
            );
        }

        matches
    }

    /// Remove an order from the book (for cancellations)
    pub fn remove_order(&self, market_id: &str, outcome: Outcome, side: OrderSide, order_id: &str) {
        let mut books = self.books.write().unwrap();

        if let Some(market_book) = books.get_mut(market_id) {
            let outcome_book = match outcome {
                Outcome::Yes => &mut market_book.yes,
                Outcome::No => &mut market_book.no,
            };

            let book = match side {
                OrderSide::Buy => &mut outcome_book.bids,
                OrderSide::Sell => &mut outcome_book.asks,
            };

            // Find and remove the order
            for (_, orders) in book.iter_mut() {
                orders.retain(|o| o.order_id != order_id);
            }

            // Clean up empty price levels
            book.retain(|_, orders| !orders.is_empty());
        }
    }

    /// Get order book depth for a market/outcome
    pub fn get_depth(
        &self,
        market_id: &str,
        outcome: Outcome,
        levels: usize,
    ) -> (Vec<OrderBookLevel>, Vec<OrderBookLevel>) {
        let books = self.books.read().unwrap();

        let empty_bids = Vec::new();
        let empty_asks = Vec::new();

        let (bids, asks) = if let Some(market_book) = books.get(market_id) {
            let outcome_book = match outcome {
                Outcome::Yes => &market_book.yes,
                Outcome::No => &market_book.no,
            };
            (&outcome_book.bids, &outcome_book.asks)
        } else {
            return (empty_bids, empty_asks);
        };

        // Aggregate bids (highest first)
        let bid_levels: Vec<OrderBookLevel> = bids
            .iter()
            .rev()
            .take(levels)
            .map(|(price, orders)| OrderBookLevel {
                price: *price as f64 / 10000.0,
                quantity: orders.iter().map(|o| o.remaining).sum(),
                orders: orders.len() as u32,
            })
            .collect();

        // Aggregate asks (lowest first)
        let ask_levels: Vec<OrderBookLevel> = asks
            .iter()
            .take(levels)
            .map(|(price, orders)| OrderBookLevel {
                price: *price as f64 / 10000.0,
                quantity: orders.iter().map(|o| o.remaining).sum(),
                orders: orders.len() as u32,
            })
            .collect();

        (bid_levels, ask_levels)
    }

    /// Get best bid price
    pub fn best_bid(&self, market_id: &str, outcome: Outcome) -> Option<f64> {
        let books = self.books.read().unwrap();
        books.get(market_id).and_then(|mb| {
            let book = match outcome {
                Outcome::Yes => &mb.yes,
                Outcome::No => &mb.no,
            };
            book.bids.keys().next_back().map(|p| *p as f64 / 10000.0)
        })
    }

    /// Get best ask price
    pub fn best_ask(&self, market_id: &str, outcome: Outcome) -> Option<f64> {
        let books = self.books.read().unwrap();
        books.get(market_id).and_then(|mb| {
            let book = match outcome {
                Outcome::Yes => &mb.yes,
                Outcome::No => &mb.no,
            };
            book.asks.keys().next().map(|p| *p as f64 / 10000.0)
        })
    }

    /// Calculate mid price
    pub fn mid_price(&self, market_id: &str, outcome: Outcome) -> Option<f64> {
        let bid = self.best_bid(market_id, outcome)?;
        let ask = self.best_ask(market_id, outcome)?;
        Some((bid + ask) / 2.0)
    }
}

impl Default for OrderBookService {
    fn default() -> Self {
        Self::new()
    }
}

impl OrderBookService {
    /// Restore order book from persisted entries (call on startup)
    pub fn restore_from_entries(&self, entries: Vec<OrderBookEntry>) {
        let mut books = self.books.write().unwrap();

        for entry in entries {
            let market_book = books
                .entry(entry.market_id.clone())
                .or_insert_with(|| MarketOrderBook {
                    yes: OutcomeOrderBook {
                        bids: BTreeMap::new(),
                        asks: BTreeMap::new(),
                    },
                    no: OutcomeOrderBook {
                        bids: BTreeMap::new(),
                        asks: BTreeMap::new(),
                    },
                });

            let outcome_book = match entry.outcome {
                Outcome::Yes => &mut market_book.yes,
                Outcome::No => &mut market_book.no,
            };

            let order_entry = OrderEntry {
                order_id: entry.order_id.clone(),
                on_chain_id: entry.on_chain_id,
                owner: entry.owner.clone(),
                price_bps: entry.price_bps,
                quantity: entry.remaining_quantity,
                remaining: entry.remaining_quantity,
                timestamp: Utc::now().timestamp(),
            };

            match entry.side {
                OrderSide::Buy => {
                    outcome_book.bids
                        .entry(entry.price_bps)
                        .or_insert_with(Vec::new)
                        .push(order_entry);
                }
                OrderSide::Sell => {
                    outcome_book.asks
                        .entry(entry.price_bps)
                        .or_insert_with(Vec::new)
                        .push(order_entry);
                }
            }
        }

        let total_orders: usize = books.values()
            .map(|mb| {
                mb.yes.bids.values().map(|v| v.len()).sum::<usize>() +
                mb.yes.asks.values().map(|v| v.len()).sum::<usize>() +
                mb.no.bids.values().map(|v| v.len()).sum::<usize>() +
                mb.no.asks.values().map(|v| v.len()).sum::<usize>()
            })
            .sum();

        info!("Order book restored: {} markets, {} orders", books.len(), total_orders);
    }

    /// Get all open orders for a market (for persistence/sync)
    pub fn get_all_orders(&self, market_id: &str) -> Vec<(String, Outcome, OrderSide, u16, u64)> {
        let books = self.books.read().unwrap();
        let mut orders = Vec::new();

        if let Some(market_book) = books.get(market_id) {
            for (price, entries) in &market_book.yes.bids {
                for e in entries {
                    orders.push((e.order_id.clone(), Outcome::Yes, OrderSide::Buy, *price, e.remaining));
                }
            }
            for (price, entries) in &market_book.yes.asks {
                for e in entries {
                    orders.push((e.order_id.clone(), Outcome::Yes, OrderSide::Sell, *price, e.remaining));
                }
            }
            for (price, entries) in &market_book.no.bids {
                for e in entries {
                    orders.push((e.order_id.clone(), Outcome::No, OrderSide::Buy, *price, e.remaining));
                }
            }
            for (price, entries) in &market_book.no.asks {
                for e in entries {
                    orders.push((e.order_id.clone(), Outcome::No, OrderSide::Sell, *price, e.remaining));
                }
            }
        }

        orders
    }
}
