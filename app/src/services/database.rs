use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use log::info;
use std::time::Duration;
use std::env;

use crate::models::{
    Market, MarketStatus, Outcome,
    Order, OrderSide, OrderType, OrderStatus,
    Position, Trade, Transaction, TransactionType, TransactionStatus,
};

/// Database connection pool configuration
pub struct PoolConfig {
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Minimum number of connections to maintain
    pub min_connections: u32,
    /// Maximum time to wait for a connection
    pub acquire_timeout: Duration,
    /// Maximum idle time before connection is closed
    pub idle_timeout: Duration,
    /// Maximum lifetime of a connection
    pub max_lifetime: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 20,
            min_connections: 5,
            acquire_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600),
            max_lifetime: Duration::from_secs(1800),
        }
    }
}

impl PoolConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            max_connections: env::var("DB_MAX_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(20),
            min_connections: env::var("DB_MIN_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
            acquire_timeout: Duration::from_secs(
                env::var("DB_ACQUIRE_TIMEOUT_SECS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(30)
            ),
            idle_timeout: Duration::from_secs(
                env::var("DB_IDLE_TIMEOUT_SECS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(600)
            ),
            max_lifetime: Duration::from_secs(
                env::var("DB_MAX_LIFETIME_SECS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1800)
            ),
        }
    }
}

pub struct DatabaseService {
    pool: PgPool,
}

impl DatabaseService {
    pub async fn new(database_url: &str) -> Result<Self> {
        Self::with_config(database_url, PoolConfig::from_env()).await
    }

    pub async fn with_config(database_url: &str, config: PoolConfig) -> Result<Self> {
        info!("Connecting to database with pool config:");
        info!("  max_connections: {}", config.max_connections);
        info!("  min_connections: {}", config.min_connections);
        info!("  acquire_timeout: {:?}", config.acquire_timeout);
        info!("  idle_timeout: {:?}", config.idle_timeout);
        info!("  max_lifetime: {:?}", config.max_lifetime);

        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(config.acquire_timeout)
            .idle_timeout(config.idle_timeout)
            .max_lifetime(config.max_lifetime)
            .connect(database_url)
            .await?;

        info!("Database connected successfully");

        // Run migrations (uncomment in production)
        // sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(Self { pool })
    }

    /// Get pool statistics for monitoring
    pub fn pool_stats(&self) -> PoolStats {
        PoolStats {
            size: self.pool.size(),
            idle_count: self.pool.num_idle(),
        }
    }

    // Markets
    pub async fn get_markets(
        &self,
        status: Option<MarketStatus>,
        category: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<Market>, i64)> {
        // Placeholder - would query database
        Ok((vec![], 0))
    }

    pub async fn get_market(&self, market_id: &str) -> Result<Option<Market>> {
        // Placeholder
        Ok(None)
    }

    pub async fn create_market(&self, market: &Market) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO markets (
                id, address, question, description, category, status,
                yes_price, no_price, volume_24h, total_volume, total_collateral,
                fee_bps, oracle, collateral_mint, yes_mint, no_mint,
                resolution_deadline, trading_end, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)
            "#,
        )
        .bind(&market.id)
        .bind(&market.address)
        .bind(&market.question)
        .bind(&market.description)
        .bind(&market.category)
        .bind(market.status as i16)
        .bind(market.yes_price)
        .bind(market.no_price)
        .bind(market.volume_24h)
        .bind(market.total_volume)
        .bind(market.total_collateral as i64)
        .bind(market.fee_bps as i16)
        .bind(&market.oracle)
        .bind(&market.collateral_mint)
        .bind(&market.yes_mint)
        .bind(&market.no_mint)
        .bind(market.resolution_deadline)
        .bind(market.trading_end)
        .bind(market.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_market_prices(
        &self,
        market_id: &str,
        yes_price: f64,
        no_price: f64,
    ) -> Result<()> {
        sqlx::query("UPDATE markets SET yes_price = $1, no_price = $2 WHERE id = $3")
            .bind(yes_price)
            .bind(no_price)
            .bind(market_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // Orders
    pub async fn get_orders(
        &self,
        owner: &str,
        market_id: Option<&str>,
        status: Option<OrderStatus>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<Order>, i64)> {
        // Placeholder
        Ok((vec![], 0))
    }

    pub async fn get_order(&self, order_id: &str) -> Result<Option<Order>> {
        // Placeholder
        Ok(None)
    }

    pub async fn create_order(&self, order: &Order) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO orders (
                id, order_id, market_id, owner, side, outcome, order_type,
                price, price_bps, quantity, filled_quantity, remaining_quantity,
                status, is_private, tx_signature, created_at, updated_at, expires_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            "#,
        )
        .bind(&order.id)
        .bind(order.order_id as i64)
        .bind(&order.market_id)
        .bind(&order.owner)
        .bind(order.side as i16)
        .bind(order.outcome as i16)
        .bind(order.order_type as i16)
        .bind(order.price)
        .bind(order.price_bps as i16)
        .bind(order.quantity as i64)
        .bind(order.filled_quantity as i64)
        .bind(order.remaining_quantity as i64)
        .bind(order.status as i16)
        .bind(order.is_private)
        .bind(&order.tx_signature)
        .bind(order.created_at)
        .bind(order.updated_at)
        .bind(order.expires_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_order_status(
        &self,
        order_id: &str,
        status: OrderStatus,
        filled_quantity: u64,
        remaining_quantity: u64,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE orders SET status = $1, filled_quantity = $2, remaining_quantity = $3, updated_at = $4 WHERE id = $5"
        )
        .bind(status as i16)
        .bind(filled_quantity as i64)
        .bind(remaining_quantity as i64)
        .bind(Utc::now())
        .bind(order_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // Positions
    pub async fn get_positions(&self, owner: &str) -> Result<Vec<Position>> {
        // Placeholder
        Ok(vec![])
    }

    pub async fn get_position(&self, owner: &str, market_id: &str) -> Result<Option<Position>> {
        // Placeholder
        Ok(None)
    }

    // Trades
    pub async fn create_trade(&self, trade: &Trade) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO trades (
                id, market_id, buy_order_id, sell_order_id, outcome,
                price, price_bps, quantity, collateral_amount,
                buyer, seller, tx_signature, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            "#,
        )
        .bind(&trade.id)
        .bind(&trade.market_id)
        .bind(&trade.buy_order_id)
        .bind(&trade.sell_order_id)
        .bind(trade.outcome as i16)
        .bind(trade.price)
        .bind(trade.price_bps as i16)
        .bind(trade.quantity as i64)
        .bind(trade.collateral_amount as i64)
        .bind(&trade.buyer)
        .bind(&trade.seller)
        .bind(&trade.tx_signature)
        .bind(trade.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_trades(
        &self,
        market_id: &str,
        outcome: Option<Outcome>,
        limit: i64,
        before: Option<&str>,
    ) -> Result<Vec<Trade>> {
        // Placeholder
        Ok(vec![])
    }

    // Transactions
    pub async fn get_transactions(
        &self,
        owner: &str,
        tx_type: Option<TransactionType>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<Transaction>, i64)> {
        // Placeholder
        Ok((vec![], 0))
    }
}

/// Database pool statistics for monitoring
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// Current number of connections in the pool
    pub size: u32,
    /// Number of idle connections
    pub idle_count: usize,
}
