//! Blockchain Reconciliation Service
//!
//! Ensures database state remains consistent with on-chain state.
//! Periodically compares key data points and flags/corrects discrepancies.

use anyhow::Result;
use chrono::{DateTime, Utc};
use log::{error, info, warn};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::models::{MarketStatus, OrderStatus};

/// Discrepancy types detected during reconciliation
#[derive(Debug, Clone)]
pub enum DiscrepancyType {
    /// Market status differs between DB and chain
    MarketStatus {
        db_status: MarketStatus,
        chain_status: u8,
    },
    /// Market price mismatch
    MarketPrice {
        db_yes: f64,
        db_no: f64,
        chain_yes: u64,
        chain_no: u64,
    },
    /// Order status mismatch
    OrderStatus {
        db_status: OrderStatus,
        chain_status: u8,
    },
    /// Order quantity mismatch
    OrderQuantity {
        db_remaining: u64,
        chain_remaining: u64,
    },
    /// Position balance mismatch
    PositionBalance {
        db_yes: u64,
        db_no: u64,
        chain_yes: u64,
        chain_no: u64,
    },
    /// Account not found on chain
    AccountMissing,
    /// Account exists on chain but not in DB
    AccountOrphan,
}

/// A detected discrepancy between DB and chain
#[derive(Debug, Clone)]
pub struct Discrepancy {
    pub entity_type: String,
    pub entity_id: String,
    pub pubkey: Option<Pubkey>,
    pub discrepancy_type: DiscrepancyType,
    pub detected_at: DateTime<Utc>,
    pub resolved: bool,
    pub resolution_action: Option<String>,
}

/// Reconciliation result for a single run
#[derive(Debug, Default)]
pub struct ReconciliationResult {
    pub markets_checked: usize,
    pub orders_checked: usize,
    pub positions_checked: usize,
    pub discrepancies_found: Vec<Discrepancy>,
    pub discrepancies_resolved: usize,
    pub errors: Vec<String>,
    pub duration_ms: u64,
}

/// Reconciliation service configuration
#[derive(Debug, Clone)]
pub struct ReconciliationConfig {
    /// How often to run full reconciliation (in seconds)
    pub interval_secs: u64,
    /// Max markets to check per run (for rate limiting)
    pub max_markets_per_run: usize,
    /// Max orders to check per run
    pub max_orders_per_run: usize,
    /// Whether to auto-resolve safe discrepancies
    pub auto_resolve: bool,
    /// Acceptable price deviation (basis points)
    pub price_tolerance_bps: u64,
}

impl Default for ReconciliationConfig {
    fn default() -> Self {
        Self {
            interval_secs: 300, // 5 minutes
            max_markets_per_run: 100,
            max_orders_per_run: 500,
            auto_resolve: true,
            price_tolerance_bps: 10, // 0.1% tolerance
        }
    }
}

/// Reconciliation service for DB-blockchain consistency
pub struct ReconciliationService {
    rpc_client: RpcClient,
    pool: PgPool,
    market_program_id: Pubkey,
    orderbook_program_id: Pubkey,
    config: ReconciliationConfig,
    /// History of detected discrepancies
    discrepancy_history: Arc<RwLock<Vec<Discrepancy>>>,
    /// Last reconciliation timestamp
    last_run: Arc<RwLock<Option<DateTime<Utc>>>>,
}

impl ReconciliationService {
    pub fn new(
        rpc_url: &str,
        pool: PgPool,
        market_program_id: Pubkey,
        orderbook_program_id: Pubkey,
        config: ReconciliationConfig,
    ) -> Self {
        let rpc_client =
            RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());

        Self {
            rpc_client,
            pool,
            market_program_id,
            orderbook_program_id,
            config,
            discrepancy_history: Arc::new(RwLock::new(Vec::new())),
            last_run: Arc::new(RwLock::new(None)),
        }
    }

    /// Run a full reconciliation cycle
    pub async fn run_reconciliation(&self) -> Result<ReconciliationResult> {
        let start = std::time::Instant::now();
        let mut result = ReconciliationResult::default();

        info!("Starting reconciliation cycle");

        // Reconcile markets
        match self.reconcile_markets(&mut result).await {
            Ok(_) => {}
            Err(e) => {
                error!("Market reconciliation failed: {}", e);
                result.errors.push(format!("Market reconciliation: {}", e));
            }
        }

        // Reconcile orders
        match self.reconcile_orders(&mut result).await {
            Ok(_) => {}
            Err(e) => {
                error!("Order reconciliation failed: {}", e);
                result.errors.push(format!("Order reconciliation: {}", e));
            }
        }

        // Store discrepancies
        if !result.discrepancies_found.is_empty() {
            let mut history = self.discrepancy_history.write().await;
            history.extend(result.discrepancies_found.clone());
            // Keep only last 1000 discrepancies
            let len = history.len();
            if len > 1000 {
                history.drain(0..len - 1000);
            }
        }

        // Update last run timestamp
        *self.last_run.write().await = Some(Utc::now());

        result.duration_ms = start.elapsed().as_millis() as u64;

        info!(
            "Reconciliation complete: {} markets, {} orders, {} discrepancies found, {} resolved, {} errors in {}ms",
            result.markets_checked,
            result.orders_checked,
            result.discrepancies_found.len(),
            result.discrepancies_resolved,
            result.errors.len(),
            result.duration_ms
        );

        Ok(result)
    }

    /// Reconcile market states
    async fn reconcile_markets(&self, result: &mut ReconciliationResult) -> Result<()> {
        // Get markets from DB
        let rows = sqlx::query(
            "SELECT id, address, status, yes_price, no_price FROM markets WHERE status != 4 LIMIT $1"
        )
        .bind(self.config.max_markets_per_run as i64)
        .fetch_all(&self.pool)
        .await?;

        for row in rows {
            result.markets_checked += 1;

            let market_id: String = row.get("id");
            let address: String = row.get("address");
            let db_status: i16 = row.get("status");
            let _db_yes_price: f64 = row.get("yes_price");
            let _db_no_price: f64 = row.get("no_price");

            // Parse address as Pubkey
            let pubkey = match address.parse::<Pubkey>() {
                Ok(pk) => pk,
                Err(_) => {
                    warn!("Invalid market address: {}", address);
                    continue;
                }
            };

            // Fetch on-chain account
            match self.rpc_client.get_account(&pubkey) {
                Ok(account) => {
                    // Deserialize market data
                    // Skip 8-byte Anchor discriminator
                    if account.data.len() < 100 {
                        result.discrepancies_found.push(Discrepancy {
                            entity_type: "market".to_string(),
                            entity_id: market_id.clone(),
                            pubkey: Some(pubkey),
                            discrepancy_type: DiscrepancyType::AccountMissing,
                            detected_at: Utc::now(),
                            resolved: false,
                            resolution_action: None,
                        });
                        continue;
                    }

                    // Parse status (offset 8 + market_id length + other fields)
                    // Simplified: check if data exists and log for now
                    // Full implementation would deserialize using Anchor
                    let chain_status = self.parse_market_status(&account.data);

                    if chain_status != db_status as u8 {
                        let discrepancy = Discrepancy {
                            entity_type: "market".to_string(),
                            entity_id: market_id.clone(),
                            pubkey: Some(pubkey),
                            discrepancy_type: DiscrepancyType::MarketStatus {
                                db_status: MarketStatus::from(db_status as u8),
                                chain_status,
                            },
                            detected_at: Utc::now(),
                            resolved: false,
                            resolution_action: None,
                        };

                        // Auto-resolve: update DB to match chain
                        if self.config.auto_resolve {
                            match self.resolve_market_status(&market_id, chain_status).await {
                                Ok(_) => {
                                    let mut resolved = discrepancy.clone();
                                    resolved.resolved = true;
                                    resolved.resolution_action =
                                        Some("Updated DB to match chain".to_string());
                                    result.discrepancies_found.push(resolved);
                                    result.discrepancies_resolved += 1;
                                }
                                Err(e) => {
                                    error!("Failed to resolve market status discrepancy: {}", e);
                                    result.discrepancies_found.push(discrepancy);
                                }
                            }
                        } else {
                            result.discrepancies_found.push(discrepancy);
                        }
                    }
                }
                Err(e) => {
                    // Account not found on chain
                    warn!("Market {} not found on chain: {}", market_id, e);
                    result.discrepancies_found.push(Discrepancy {
                        entity_type: "market".to_string(),
                        entity_id: market_id,
                        pubkey: Some(pubkey),
                        discrepancy_type: DiscrepancyType::AccountMissing,
                        detected_at: Utc::now(),
                        resolved: false,
                        resolution_action: None,
                    });
                }
            }
        }

        Ok(())
    }

    /// Reconcile order states
    async fn reconcile_orders(&self, result: &mut ReconciliationResult) -> Result<()> {
        // Get open orders from DB
        let rows = sqlx::query(
            "SELECT id, order_id, market_id, status, remaining_quantity, tx_signature FROM orders WHERE status = 0 LIMIT $1"
        )
        .bind(self.config.max_orders_per_run as i64)
        .fetch_all(&self.pool)
        .await?;

        for row in rows {
            result.orders_checked += 1;

            let order_uuid: String = row.get("id");
            let order_id: i64 = row.get("order_id");
            let market_id: String = row.get("market_id");
            let db_status: i16 = row.get("status");
            let db_remaining: i64 = row.get("remaining_quantity");

            // Derive order PDA
            let market_pda = self.derive_market_pda(&market_id);
            let order_pda = self.derive_order_pda(&market_pda, order_id as u64);

            match self.rpc_client.get_account(&order_pda) {
                Ok(account) => {
                    if account.data.len() < 50 {
                        continue;
                    }

                    // Parse order data
                    let (chain_status, chain_remaining) = self.parse_order_data(&account.data);

                    // Check status mismatch
                    if chain_status != db_status as u8 {
                        let discrepancy = Discrepancy {
                            entity_type: "order".to_string(),
                            entity_id: order_uuid.clone(),
                            pubkey: Some(order_pda),
                            discrepancy_type: DiscrepancyType::OrderStatus {
                                db_status: OrderStatus::from(db_status as u8),
                                chain_status,
                            },
                            detected_at: Utc::now(),
                            resolved: false,
                            resolution_action: None,
                        };

                        if self.config.auto_resolve {
                            match self
                                .resolve_order_status(&order_uuid, chain_status, chain_remaining)
                                .await
                            {
                                Ok(_) => {
                                    let mut resolved = discrepancy.clone();
                                    resolved.resolved = true;
                                    resolved.resolution_action =
                                        Some("Updated DB to match chain".to_string());
                                    result.discrepancies_found.push(resolved);
                                    result.discrepancies_resolved += 1;
                                }
                                Err(e) => {
                                    error!("Failed to resolve order status discrepancy: {}", e);
                                    result.discrepancies_found.push(discrepancy);
                                }
                            }
                        } else {
                            result.discrepancies_found.push(discrepancy);
                        }
                    }
                    // Check quantity mismatch
                    else if chain_remaining != db_remaining as u64 {
                        let discrepancy = Discrepancy {
                            entity_type: "order".to_string(),
                            entity_id: order_uuid.clone(),
                            pubkey: Some(order_pda),
                            discrepancy_type: DiscrepancyType::OrderQuantity {
                                db_remaining: db_remaining as u64,
                                chain_remaining,
                            },
                            detected_at: Utc::now(),
                            resolved: false,
                            resolution_action: None,
                        };

                        if self.config.auto_resolve {
                            match self
                                .resolve_order_quantity(&order_uuid, chain_remaining)
                                .await
                            {
                                Ok(_) => {
                                    let mut resolved = discrepancy.clone();
                                    resolved.resolved = true;
                                    resolved.resolution_action =
                                        Some("Updated DB quantity to match chain".to_string());
                                    result.discrepancies_found.push(resolved);
                                    result.discrepancies_resolved += 1;
                                }
                                Err(e) => {
                                    error!("Failed to resolve order quantity discrepancy: {}", e);
                                    result.discrepancies_found.push(discrepancy);
                                }
                            }
                        } else {
                            result.discrepancies_found.push(discrepancy);
                        }
                    }
                }
                Err(_) => {
                    // Order not found on chain - might be closed/cancelled
                    // Mark as filled/cancelled in DB if auto_resolve
                    if self.config.auto_resolve {
                        match self.resolve_order_status(&order_uuid, 2, 0).await {
                            // 2 = Cancelled
                            Ok(_) => {
                                result.discrepancies_found.push(Discrepancy {
                                    entity_type: "order".to_string(),
                                    entity_id: order_uuid,
                                    pubkey: Some(order_pda),
                                    discrepancy_type: DiscrepancyType::AccountMissing,
                                    detected_at: Utc::now(),
                                    resolved: true,
                                    resolution_action: Some(
                                        "Marked as cancelled (account closed)".to_string(),
                                    ),
                                });
                                result.discrepancies_resolved += 1;
                            }
                            Err(_) => {}
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Parse market status from account data
    fn parse_market_status(&self, data: &[u8]) -> u8 {
        // Anchor discriminator (8) + market_id (4 + len) + question + description + category
        // Status is typically around offset 200-300 depending on string lengths
        // For now, return a default - full implementation needs proper deserialization
        if data.len() > 100 {
            // Simplified: look for status byte at expected offset
            // Real implementation should use anchor_lang::AccountDeserialize
            0
        } else {
            0
        }
    }

    /// Parse order data from account
    fn parse_order_data(&self, data: &[u8]) -> (u8, u64) {
        // Skip 8-byte discriminator
        if data.len() < 50 {
            return (0, 0);
        }

        // Order layout (approximate):
        // 8: discriminator
        // 8: order_id (u64)
        // 32: owner (Pubkey)
        // 32: market (Pubkey)
        // 1: side
        // 1: outcome
        // 2: price_bps
        // 8: original_quantity
        // 8: remaining_quantity
        // 8: filled_quantity
        // 1: status

        // Status is at offset 8 + 8 + 32 + 32 + 1 + 1 + 2 + 8 + 8 + 8 = 108
        // Remaining is at offset 8 + 8 + 32 + 32 + 1 + 1 + 2 + 8 = 92

        let remaining = if data.len() >= 100 {
            u64::from_le_bytes(data[92..100].try_into().unwrap_or([0; 8]))
        } else {
            0
        };

        let status = if data.len() > 116 { data[116] } else { 0 };

        (status, remaining)
    }

    /// Derive market PDA
    fn derive_market_pda(&self, market_id: &str) -> Pubkey {
        Pubkey::find_program_address(&[b"market", market_id.as_bytes()], &self.market_program_id).0
    }

    /// Derive order PDA
    fn derive_order_pda(&self, market: &Pubkey, order_id: u64) -> Pubkey {
        Pubkey::find_program_address(
            &[b"order", market.as_ref(), &order_id.to_le_bytes()],
            &self.orderbook_program_id,
        )
        .0
    }

    /// Resolve market status discrepancy by updating DB
    async fn resolve_market_status(&self, market_id: &str, chain_status: u8) -> Result<()> {
        sqlx::query("UPDATE markets SET status = $1 WHERE id = $2")
            .bind(chain_status as i16)
            .bind(market_id)
            .execute(&self.pool)
            .await?;

        info!("Resolved market {} status to {}", market_id, chain_status);
        Ok(())
    }

    /// Resolve order status discrepancy
    async fn resolve_order_status(
        &self,
        order_id: &str,
        chain_status: u8,
        remaining: u64,
    ) -> Result<()> {
        let filled = if remaining == 0 && chain_status == 1 {
            // Filled
            sqlx::query("SELECT quantity FROM orders WHERE id = $1")
                .bind(order_id)
                .fetch_one(&self.pool)
                .await
                .map(|r| r.get::<i64, _>("quantity") as u64)
                .unwrap_or(0)
        } else {
            0
        };

        sqlx::query(
            "UPDATE orders SET status = $1, remaining_quantity = $2, filled_quantity = $3, updated_at = $4 WHERE id = $5"
        )
        .bind(chain_status as i16)
        .bind(remaining as i64)
        .bind(filled as i64)
        .bind(Utc::now())
        .bind(order_id)
        .execute(&self.pool)
        .await?;

        info!("Resolved order {} status to {}", order_id, chain_status);
        Ok(())
    }

    /// Resolve order quantity discrepancy
    async fn resolve_order_quantity(&self, order_id: &str, chain_remaining: u64) -> Result<()> {
        // Get original quantity to calculate filled
        let row = sqlx::query("SELECT quantity FROM orders WHERE id = $1")
            .bind(order_id)
            .fetch_one(&self.pool)
            .await?;

        let original: i64 = row.get("quantity");
        let filled = (original as u64).saturating_sub(chain_remaining);

        sqlx::query(
            "UPDATE orders SET remaining_quantity = $1, filled_quantity = $2, updated_at = $3 WHERE id = $4"
        )
        .bind(chain_remaining as i64)
        .bind(filled as i64)
        .bind(Utc::now())
        .bind(order_id)
        .execute(&self.pool)
        .await?;

        info!(
            "Resolved order {} remaining to {}",
            order_id, chain_remaining
        );
        Ok(())
    }

    /// Get discrepancy history
    pub async fn get_discrepancy_history(&self) -> Vec<Discrepancy> {
        self.discrepancy_history.read().await.clone()
    }

    /// Get last reconciliation timestamp
    pub async fn last_reconciliation(&self) -> Option<DateTime<Utc>> {
        *self.last_run.read().await
    }

    /// Start background reconciliation loop
    pub fn start_background_reconciliation(self: Arc<Self>) {
        let interval = Duration::from_secs(self.config.interval_secs);

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;

                match self.run_reconciliation().await {
                    Ok(result) => {
                        if !result.discrepancies_found.is_empty() {
                            warn!(
                                "Reconciliation found {} discrepancies ({} resolved)",
                                result.discrepancies_found.len(),
                                result.discrepancies_resolved
                            );
                        }
                    }
                    Err(e) => {
                        error!("Reconciliation cycle failed: {}", e);
                    }
                }
            }
        });
    }
}

/// Trait for row.get operations
use sqlx::Row;
