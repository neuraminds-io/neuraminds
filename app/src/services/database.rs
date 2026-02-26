use anyhow::Result;
use chrono::Utc;
use log::info;
use sqlx::{postgres::PgPoolOptions, PgPool, Postgres, Row};
use std::env;
use std::path::PathBuf;
use std::time::Duration;

use crate::models::{
    Market, MarketStatus, Order, OrderSide, OrderStatus, OrderType, Outcome, Position, Trade,
    Transaction as ModelTransaction, TransactionType,
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
                    .unwrap_or(30),
            ),
            idle_timeout: Duration::from_secs(
                env::var("DB_IDLE_TIMEOUT_SECS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(600),
            ),
            max_lifetime: Duration::from_secs(
                env::var("DB_MAX_LIFETIME_SECS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1800),
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

        // Run migrations automatically
        info!("Running database migrations...");
        let migrations_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../migrations");
        sqlx::migrate::Migrator::new(migrations_path.as_path())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to load migrations: {}", e))?
            .run(&pool)
            .await
            .map_err(|e| {
                log::error!("Migration failed: {}", e);
                anyhow::anyhow!("Database migration failed: {}", e)
            })?;
        info!("Database migrations completed");

        Ok(Self { pool })
    }

    /// Get pool statistics for monitoring
    pub fn pool_stats(&self) -> PoolStats {
        PoolStats {
            size: self.pool.size(),
            idle_count: self.pool.num_idle(),
        }
    }

    /// Begin a new database transaction
    pub async fn begin_transaction(&self) -> Result<sqlx::Transaction<'_, Postgres>> {
        Ok(self.pool.begin().await?)
    }

    /// Get reference to the pool for advanced operations
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    // Markets
    pub async fn get_markets(
        &self,
        status: Option<MarketStatus>,
        category: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<Market>, i64)> {
        let mut query = String::from("SELECT * FROM markets WHERE 1=1");
        let mut count_query = String::from("SELECT COUNT(*) as total FROM markets WHERE 1=1");

        if status.is_some() {
            query.push_str(" AND status = $1");
            count_query.push_str(" AND status = $1");
        }
        if category.is_some() {
            let idx = if status.is_some() { "2" } else { "1" };
            query.push_str(&format!(" AND category = ${}", idx));
            count_query.push_str(&format!(" AND category = ${}", idx));
        }

        query.push_str(" ORDER BY created_at DESC LIMIT $3 OFFSET $4");

        // Build and execute query based on parameters
        let rows = match (status, category) {
            (Some(s), Some(c)) => {
                sqlx::query(&query)
                    .bind(s as i16)
                    .bind(c)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&self.pool)
                    .await?
            }
            (Some(s), None) => {
                let q = "SELECT * FROM markets WHERE status = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3";
                sqlx::query(q)
                    .bind(s as i16)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&self.pool)
                    .await?
            }
            (None, Some(c)) => {
                let q = "SELECT * FROM markets WHERE category = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3";
                sqlx::query(q)
                    .bind(c)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&self.pool)
                    .await?
            }
            (None, None) => {
                let q = "SELECT * FROM markets ORDER BY created_at DESC LIMIT $1 OFFSET $2";
                sqlx::query(q)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&self.pool)
                    .await?
            }
        };

        // Get total count
        let total: i64 = match (status, category) {
            (Some(s), Some(c)) => {
                let q = "SELECT COUNT(*) as total FROM markets WHERE status = $1 AND category = $2";
                sqlx::query_scalar(q)
                    .bind(s as i16)
                    .bind(c)
                    .fetch_one(&self.pool)
                    .await?
            }
            (Some(s), None) => {
                let q = "SELECT COUNT(*) as total FROM markets WHERE status = $1";
                sqlx::query_scalar(q)
                    .bind(s as i16)
                    .fetch_one(&self.pool)
                    .await?
            }
            (None, Some(c)) => {
                let q = "SELECT COUNT(*) as total FROM markets WHERE category = $1";
                sqlx::query_scalar(q).bind(c).fetch_one(&self.pool).await?
            }
            (None, None) => {
                sqlx::query_scalar("SELECT COUNT(*) as total FROM markets")
                    .fetch_one(&self.pool)
                    .await?
            }
        };

        let markets = rows.iter().map(|row| self.row_to_market(row)).collect();
        Ok((markets, total))
    }

    fn row_to_market(&self, row: &sqlx::postgres::PgRow) -> Market {
        Market {
            id: row.get("id"),
            address: row.get("address"),
            question: row.get("question"),
            description: row.get("description"),
            category: row.get("category"),
            status: MarketStatus::from(row.get::<i16, _>("status") as u8),
            yes_price: row.get("yes_price"),
            no_price: row.get("no_price"),
            yes_supply: row.get::<i64, _>("yes_supply") as u64,
            no_supply: row.get::<i64, _>("no_supply") as u64,
            volume_24h: row.get("volume_24h"),
            total_volume: row.get("total_volume"),
            total_collateral: row.get::<i64, _>("total_collateral") as u64,
            fee_bps: row.get::<i16, _>("fee_bps") as u16,
            oracle: row.get("oracle"),
            collateral_mint: row.get("collateral_mint"),
            yes_mint: row.get("yes_mint"),
            no_mint: row.get("no_mint"),
            resolution_deadline: row.get("resolution_deadline"),
            trading_end: row.get("trading_end"),
            resolved_outcome: row
                .try_get::<i16, _>("resolved_outcome")
                .ok()
                .map(|v| Outcome::from(v as u8)),
            created_at: row.get("created_at"),
            resolved_at: row.try_get("resolved_at").ok(),
        }
    }

    pub async fn get_market(&self, market_id: &str) -> Result<Option<Market>> {
        let row = sqlx::query("SELECT * FROM markets WHERE id = $1")
            .bind(market_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| self.row_to_market(&r)))
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
        let base_where = "WHERE owner = $1";

        let rows = match (market_id, status) {
            (Some(m), Some(s)) => {
                let q = format!("SELECT * FROM orders {} AND market_id = $2 AND status = $3 ORDER BY created_at DESC LIMIT $4 OFFSET $5", base_where);
                sqlx::query(&q)
                    .bind(owner)
                    .bind(m)
                    .bind(s as i16)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&self.pool)
                    .await?
            }
            (Some(m), None) => {
                let q = format!("SELECT * FROM orders {} AND market_id = $2 ORDER BY created_at DESC LIMIT $3 OFFSET $4", base_where);
                sqlx::query(&q)
                    .bind(owner)
                    .bind(m)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&self.pool)
                    .await?
            }
            (None, Some(s)) => {
                let q = format!("SELECT * FROM orders {} AND status = $2 ORDER BY created_at DESC LIMIT $3 OFFSET $4", base_where);
                sqlx::query(&q)
                    .bind(owner)
                    .bind(s as i16)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&self.pool)
                    .await?
            }
            (None, None) => {
                let q = format!(
                    "SELECT * FROM orders {} ORDER BY created_at DESC LIMIT $2 OFFSET $3",
                    base_where
                );
                sqlx::query(&q)
                    .bind(owner)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&self.pool)
                    .await?
            }
        };

        let total: i64 = match (market_id, status) {
            (Some(m), Some(s)) => sqlx::query_scalar(
                "SELECT COUNT(*) FROM orders WHERE owner = $1 AND market_id = $2 AND status = $3",
            )
            .bind(owner)
            .bind(m)
            .bind(s as i16)
            .fetch_one(&self.pool)
            .await?,
            (Some(m), None) => {
                sqlx::query_scalar(
                    "SELECT COUNT(*) FROM orders WHERE owner = $1 AND market_id = $2",
                )
                .bind(owner)
                .bind(m)
                .fetch_one(&self.pool)
                .await?
            }
            (None, Some(s)) => {
                sqlx::query_scalar("SELECT COUNT(*) FROM orders WHERE owner = $1 AND status = $2")
                    .bind(owner)
                    .bind(s as i16)
                    .fetch_one(&self.pool)
                    .await?
            }
            (None, None) => {
                sqlx::query_scalar("SELECT COUNT(*) FROM orders WHERE owner = $1")
                    .bind(owner)
                    .fetch_one(&self.pool)
                    .await?
            }
        };

        let orders = rows.iter().map(|row| self.row_to_order(row)).collect();
        Ok((orders, total))
    }

    fn row_to_order(&self, row: &sqlx::postgres::PgRow) -> Order {
        Order {
            id: row.get("id"),
            order_id: row.get::<i64, _>("order_id") as u64,
            market_id: row.get("market_id"),
            owner: row.get("owner"),
            side: OrderSide::from(row.get::<i16, _>("side") as u8),
            outcome: Outcome::from(row.get::<i16, _>("outcome") as u8),
            order_type: OrderType::from(row.get::<i16, _>("order_type") as u8),
            price: row.get("price"),
            price_bps: row.get::<i16, _>("price_bps") as u16,
            quantity: row.get::<i64, _>("quantity") as u64,
            filled_quantity: row.get::<i64, _>("filled_quantity") as u64,
            remaining_quantity: row.get::<i64, _>("remaining_quantity") as u64,
            status: OrderStatus::from(row.get::<i16, _>("status") as u8),
            is_private: row.get("is_private"),
            tx_signature: row.try_get("tx_signature").ok(),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            expires_at: row.try_get("expires_at").ok(),
        }
    }

    pub async fn get_order(&self, order_id: &str) -> Result<Option<Order>> {
        let row = sqlx::query("SELECT * FROM orders WHERE id = $1")
            .bind(order_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| self.row_to_order(&r)))
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
        let rows = sqlx::query("SELECT * FROM positions WHERE owner = $1 ORDER BY created_at DESC")
            .bind(owner)
            .fetch_all(&self.pool)
            .await?;

        let positions = rows.iter().map(|row| self.row_to_position(row)).collect();
        Ok(positions)
    }

    fn row_to_position(&self, row: &sqlx::postgres::PgRow) -> Position {
        Position {
            market_id: row.get("market_id"),
            market_question: row.try_get("market_question").unwrap_or_default(),
            owner: row.get("owner"),
            yes_balance: row.get::<i64, _>("yes_balance") as u64,
            no_balance: row.get::<i64, _>("no_balance") as u64,
            avg_yes_cost: row.try_get("avg_yes_cost").unwrap_or(0.0),
            avg_no_cost: row.try_get("avg_no_cost").unwrap_or(0.0),
            current_yes_price: row.try_get("current_yes_price").unwrap_or(0.5),
            current_no_price: row.try_get("current_no_price").unwrap_or(0.5),
            unrealized_pnl: row.try_get("unrealized_pnl").unwrap_or(0.0),
            realized_pnl: row.try_get("realized_pnl").unwrap_or(0.0),
            total_deposited: row
                .try_get::<i64, _>("total_deposited")
                .map(|v| v as u64)
                .unwrap_or(0),
            total_withdrawn: row
                .try_get::<i64, _>("total_withdrawn")
                .map(|v| v as u64)
                .unwrap_or(0),
            open_order_count: row
                .try_get::<i32, _>("open_order_count")
                .map(|v| v as u32)
                .unwrap_or(0),
            total_trades: row
                .try_get::<i32, _>("total_trades")
                .map(|v| v as u32)
                .unwrap_or(0),
            created_at: row.get("created_at"),
        }
    }

    pub async fn get_position(&self, owner: &str, market_id: &str) -> Result<Option<Position>> {
        let row = sqlx::query("SELECT * FROM positions WHERE owner = $1 AND market_id = $2")
            .bind(owner)
            .bind(market_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| self.row_to_position(&r)))
    }

    // Trades

    /// Create trade with position updates in a single transaction
    /// HIGH-024: Transaction boundaries for atomicity
    pub async fn create_trade_with_positions(
        &self,
        trade: &Trade,
        buyer_yes_delta: i64,
        buyer_no_delta: i64,
        seller_yes_delta: i64,
        seller_no_delta: i64,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Insert trade
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
        .execute(&mut *tx)
        .await?;

        // Update buyer position
        sqlx::query(
            r#"
            INSERT INTO positions (market_id, owner, yes_balance, no_balance, total_trades)
            VALUES ($1, $2, $3, $4, 1)
            ON CONFLICT (market_id, owner) DO UPDATE SET
                yes_balance = positions.yes_balance + $3,
                no_balance = positions.no_balance + $4,
                total_trades = positions.total_trades + 1
            "#,
        )
        .bind(&trade.market_id)
        .bind(&trade.buyer)
        .bind(buyer_yes_delta)
        .bind(buyer_no_delta)
        .execute(&mut *tx)
        .await?;

        // Update seller position
        sqlx::query(
            r#"
            INSERT INTO positions (market_id, owner, yes_balance, no_balance, total_trades)
            VALUES ($1, $2, $3, $4, 1)
            ON CONFLICT (market_id, owner) DO UPDATE SET
                yes_balance = positions.yes_balance + $3,
                no_balance = positions.no_balance + $4,
                total_trades = positions.total_trades + 1
            "#,
        )
        .bind(&trade.market_id)
        .bind(&trade.seller)
        .bind(seller_yes_delta)
        .bind(seller_no_delta)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

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

    fn row_to_trade(&self, row: &sqlx::postgres::PgRow) -> Trade {
        Trade {
            id: row.get("id"),
            market_id: row.get("market_id"),
            buy_order_id: row.get("buy_order_id"),
            sell_order_id: row.get("sell_order_id"),
            outcome: Outcome::from(row.get::<i16, _>("outcome") as u8),
            price: row.get("price"),
            price_bps: row.get::<i16, _>("price_bps") as u16,
            quantity: row.get::<i64, _>("quantity") as u64,
            collateral_amount: row.get::<i64, _>("collateral_amount") as u64,
            buyer: row.get("buyer"),
            seller: row.get("seller"),
            tx_signature: row
                .try_get("tx_signature")
                .unwrap_or_else(|_| String::new()),
            created_at: row.get("created_at"),
        }
    }

    pub async fn get_trades(
        &self,
        market_id: &str,
        outcome: Option<Outcome>,
        limit: i64,
        before: Option<&str>,
    ) -> Result<Vec<Trade>> {
        let rows = match (outcome, before) {
            (Some(o), Some(b)) => {
                sqlx::query("SELECT * FROM trades WHERE market_id = $1 AND outcome = $2 AND id < $3 ORDER BY created_at DESC LIMIT $4")
                    .bind(market_id).bind(o as i16).bind(b).bind(limit).fetch_all(&self.pool).await?
            }
            (Some(o), None) => {
                sqlx::query("SELECT * FROM trades WHERE market_id = $1 AND outcome = $2 ORDER BY created_at DESC LIMIT $3")
                    .bind(market_id).bind(o as i16).bind(limit).fetch_all(&self.pool).await?
            }
            (None, Some(b)) => {
                sqlx::query("SELECT * FROM trades WHERE market_id = $1 AND id < $2 ORDER BY created_at DESC LIMIT $3")
                    .bind(market_id).bind(b).bind(limit).fetch_all(&self.pool).await?
            }
            (None, None) => {
                sqlx::query("SELECT * FROM trades WHERE market_id = $1 ORDER BY created_at DESC LIMIT $2")
                    .bind(market_id).bind(limit).fetch_all(&self.pool).await?
            }
        };

        let trades = rows.iter().map(|row| self.row_to_trade(row)).collect();
        Ok(trades)
    }

    // Transactions
    pub async fn get_transactions(
        &self,
        owner: &str,
        tx_type: Option<TransactionType>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<ModelTransaction>, i64)> {
        let rows = match tx_type {
            Some(t) => {
                sqlx::query("SELECT * FROM transactions WHERE owner = $1 AND tx_type = $2 ORDER BY created_at DESC LIMIT $3 OFFSET $4")
                    .bind(owner).bind(t as i16).bind(limit).bind(offset).fetch_all(&self.pool).await?
            }
            None => {
                sqlx::query("SELECT * FROM transactions WHERE owner = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3")
                    .bind(owner).bind(limit).bind(offset).fetch_all(&self.pool).await?
            }
        };

        let total: i64 = match tx_type {
            Some(t) => {
                sqlx::query_scalar(
                    "SELECT COUNT(*) FROM transactions WHERE owner = $1 AND tx_type = $2",
                )
                .bind(owner)
                .bind(t as i16)
                .fetch_one(&self.pool)
                .await?
            }
            None => {
                sqlx::query_scalar("SELECT COUNT(*) FROM transactions WHERE owner = $1")
                    .bind(owner)
                    .fetch_one(&self.pool)
                    .await?
            }
        };

        let transactions = rows
            .iter()
            .map(|row| ModelTransaction {
                id: row.get("id"),
                owner: row.get("owner"),
                market_id: row.try_get("market_id").ok(),
                tx_type: TransactionType::from(row.get::<i16, _>("tx_type") as u8),
                amount: row.get::<i64, _>("amount") as u64,
                fee: row.try_get::<i64, _>("fee").map(|v| v as u64).unwrap_or(0),
                tx_signature: row.try_get::<String, _>("tx_signature").ok(),
                status: row
                    .try_get("status")
                    .unwrap_or_else(|_| "pending".to_string()),
                created_at: row.get("created_at"),
            })
            .collect();

        Ok((transactions, total))
    }

    // Order Book Persistence
    /// Add order to persistent order book
    pub async fn add_orderbook_entry(
        &self,
        order_id: &str,
        market_id: &str,
        outcome: Outcome,
        side: OrderSide,
        price_bps: u16,
        remaining_quantity: u64,
        owner: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO orderbook_entries (market_id, order_id, outcome, side, price_bps, remaining_quantity, owner)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (order_id) DO UPDATE SET remaining_quantity = $6
            "#,
        )
        .bind(market_id)
        .bind(order_id)
        .bind(outcome as i16)
        .bind(side as i16)
        .bind(price_bps as i16)
        .bind(remaining_quantity as i64)
        .bind(owner)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Remove order from persistent order book
    pub async fn remove_orderbook_entry(&self, order_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM orderbook_entries WHERE order_id = $1")
            .bind(order_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Update remaining quantity in persistent order book
    pub async fn update_orderbook_entry_quantity(
        &self,
        order_id: &str,
        remaining_quantity: u64,
    ) -> Result<()> {
        if remaining_quantity == 0 {
            self.remove_orderbook_entry(order_id).await
        } else {
            sqlx::query("UPDATE orderbook_entries SET remaining_quantity = $1 WHERE order_id = $2")
                .bind(remaining_quantity as i64)
                .bind(order_id)
                .execute(&self.pool)
                .await?;
            Ok(())
        }
    }

    /// Load all open order book entries for recovery
    pub async fn load_orderbook_entries(&self) -> Result<Vec<OrderBookEntry>> {
        let rows = sqlx::query(
            r#"
            SELECT o.id, o.order_id, o.market_id, o.owner, o.outcome, o.side,
                   o.price_bps, o.remaining_quantity, o.created_at
            FROM orderbook_entries oe
            JOIN orders o ON o.id = oe.order_id
            WHERE o.status = 0
            ORDER BY o.market_id, o.outcome, o.side, o.price_bps, o.created_at
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let entries = rows
            .iter()
            .map(|row| OrderBookEntry {
                order_id: row.get("id"),
                on_chain_id: row.get::<i64, _>("order_id") as u64,
                market_id: row.get("market_id"),
                owner: row.get("owner"),
                outcome: Outcome::from(row.get::<i16, _>("outcome") as u8),
                side: OrderSide::from(row.get::<i16, _>("side") as u8),
                price_bps: row.get::<i16, _>("price_bps") as u16,
                remaining_quantity: row.get::<i64, _>("remaining_quantity") as u64,
            })
            .collect();

        Ok(entries)
    }
}

/// Order book entry for persistence and recovery
#[derive(Debug, Clone)]
pub struct OrderBookEntry {
    pub order_id: String,
    pub on_chain_id: u64,
    pub market_id: String,
    pub owner: String,
    pub outcome: Outcome,
    pub side: OrderSide,
    pub price_bps: u16,
    pub remaining_quantity: u64,
}

/// Database pool statistics for monitoring
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// Current number of connections in the pool
    pub size: u32,
    /// Number of idle connections
    pub idle_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_config_default() {
        let config = PoolConfig::default();
        assert_eq!(config.max_connections, 20);
        assert_eq!(config.min_connections, 5);
        assert_eq!(config.acquire_timeout, Duration::from_secs(30));
        assert_eq!(config.idle_timeout, Duration::from_secs(600));
        assert_eq!(config.max_lifetime, Duration::from_secs(1800));
    }

    #[test]
    fn test_market_status_conversion() {
        assert_eq!(MarketStatus::from(0u8), MarketStatus::Active);
        assert_eq!(MarketStatus::from(1u8), MarketStatus::Paused);
        assert_eq!(MarketStatus::from(2u8), MarketStatus::Closed);
        assert_eq!(MarketStatus::from(3u8), MarketStatus::Resolved);
        assert_eq!(MarketStatus::from(4u8), MarketStatus::Cancelled);
        // Unknown values default to Active
        assert_eq!(MarketStatus::from(255u8), MarketStatus::Active);
    }

    #[test]
    fn test_order_status_conversion() {
        assert_eq!(OrderStatus::from(0u8), OrderStatus::Open);
        assert_eq!(OrderStatus::from(1u8), OrderStatus::PartiallyFilled);
        assert_eq!(OrderStatus::from(2u8), OrderStatus::Filled);
        assert_eq!(OrderStatus::from(3u8), OrderStatus::Cancelled);
        assert_eq!(OrderStatus::from(4u8), OrderStatus::Expired);
        assert_eq!(OrderStatus::from(255u8), OrderStatus::Open);
    }

    #[test]
    fn test_order_side_conversion() {
        assert_eq!(OrderSide::from(0u8), OrderSide::Buy);
        assert_eq!(OrderSide::from(1u8), OrderSide::Sell);
        assert_eq!(OrderSide::from(255u8), OrderSide::Buy);
    }

    #[test]
    fn test_outcome_conversion() {
        assert_eq!(Outcome::from(1u8), Outcome::Yes);
        assert_eq!(Outcome::from(2u8), Outcome::No);
        // Unknown values default to Yes
        assert_eq!(Outcome::from(0u8), Outcome::Yes);
        assert_eq!(Outcome::from(255u8), Outcome::Yes);
    }

    #[test]
    fn test_order_type_conversion() {
        assert_eq!(OrderType::from(0u8), OrderType::Limit);
        assert_eq!(OrderType::from(1u8), OrderType::Market);
        // Unknown values default to Limit
        assert_eq!(OrderType::from(255u8), OrderType::Limit);
    }
}
