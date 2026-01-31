use actix_cors::Cors;
use actix_web::{web, App, HttpServer, middleware as actix_middleware, http::header};
use actix_governor::{Governor, GovernorConfigBuilder};
use std::sync::atomic::{AtomicBool, Ordering};
use dotenv::dotenv;
use log::{info, warn};
use std::sync::Arc;

mod api;
mod config;
mod middleware;
mod models;
mod services;

use api::JwtService;
use config::AppConfig;
use services::{DatabaseService, SolanaService, OrderBookService, RedisService, MetricsService, WebSocketHub, ReconciliationService, ReconciliationConfig};

pub struct AppState {
    pub config: AppConfig,
    pub db: DatabaseService,
    pub solana: SolanaService,
    pub orderbook: OrderBookService,
    pub redis: RedisService,
    pub jwt: JwtService,
    pub metrics: MetricsService,
    pub ws_hub: WebSocketHub,
    pub reconciliation: Arc<ReconciliationService>,
    pub is_shutting_down: Arc<AtomicBool>,
}

/// Graceful shutdown handler
async fn graceful_shutdown(state: Arc<AppState>) {
    // Wait for shutdown signal
    tokio::signal::ctrl_c().await.expect("Failed to install CTRL+C handler");
    info!("Shutdown signal received, initiating graceful shutdown...");

    // Set shutdown flag
    state.is_shutting_down.store(true, Ordering::SeqCst);

    // Give in-flight requests time to complete
    info!("Waiting for in-flight requests to complete...");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    info!("Graceful shutdown complete");
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    services::logging::init();

    info!("Starting Polyguard Backend API...");

    let config = AppConfig::from_env();
    let bind_addr = format!("{}:{}", config.host, config.port);

    info!("Initializing services...");

    // Initialize services
    let db = DatabaseService::new(&config.database_url)
        .await
        .expect("Failed to connect to database");

    let solana = SolanaService::new(&config.solana_rpc_url, &config.keeper_keypair_path)
        .expect("Failed to initialize Solana service");

    let redis = RedisService::new(&config.redis_url)
        .await
        .expect("Failed to connect to Redis");

    let orderbook = OrderBookService::new();

    // Restore order book from database
    match db.load_orderbook_entries().await {
        Ok(entries) => {
            let count = entries.len();
            orderbook.restore_from_entries(entries);
            if count > 0 {
                info!("Restored {} order book entries from database", count);
            }
        }
        Err(e) => {
            warn!("Failed to restore order book (table may not exist yet): {}", e);
        }
    }

    let jwt = JwtService::new(&config.jwt_secret);

    let metrics = MetricsService::new();

    let ws_hub = WebSocketHub::new();

    // Initialize reconciliation service for DB-blockchain consistency
    let reconciliation_config = ReconciliationConfig::default();
    let reconciliation = Arc::new(ReconciliationService::new(
        &config.solana_rpc_url,
        db.pool().clone(),
        solana.market_program_id(),
        solana.orderbook_program_id(),
        reconciliation_config,
    ));

    // Start background reconciliation
    if !config.is_development {
        info!("Starting background reconciliation service");
        reconciliation.clone().start_background_reconciliation();
    } else {
        info!("Skipping reconciliation in development mode");
    }

    let app_state = Arc::new(AppState {
        config: config.clone(),
        db,
        solana,
        orderbook,
        redis,
        jwt,
        metrics,
        ws_hub,
        reconciliation,
        is_shutting_down: Arc::new(AtomicBool::new(false)),
    });

    // Spawn graceful shutdown handler
    let shutdown_state = app_state.clone();
    tokio::spawn(async move {
        graceful_shutdown(shutdown_state).await;
    });

    info!("Starting HTTP server on {}", bind_addr);

    // SECURITY: Configure rate limiting - 60 requests per minute per IP
    let governor_conf = GovernorConfigBuilder::default()
        .per_second(1)
        .burst_size(60)
        .finish()
        .expect("valid governor configuration");

    let config_clone = config.clone();

    HttpServer::new(move || {
        // SECURITY: Build CORS configuration based on environment
        let cors = if config_clone.is_development {
            warn!("CORS: Development mode - allowing all origins");
            Cors::default()
                .allow_any_origin()
                .allowed_methods(vec!["GET", "POST", "DELETE", "OPTIONS"])
                .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT, header::CONTENT_TYPE])
                .max_age(3600)
        } else {
            // Production: Only allow specific origins
            let mut cors = Cors::default()
                .allowed_methods(vec!["GET", "POST", "DELETE", "OPTIONS"])
                .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT, header::CONTENT_TYPE])
                .max_age(3600);

            for origin in &config_clone.cors_origins {
                if origin != "*" {
                    cors = cors.allowed_origin(origin);
                }
            }
            cors
        };

        App::new()
            .app_data(web::Data::new(app_state.clone()))
            // SECURITY: Add rate limiting
            .wrap(Governor::new(&governor_conf))
            // SECURITY: Geo-blocking (only in production)
            .wrap(crate::middleware::GeoBlock::new(!config_clone.is_development))
            .wrap(cors)
            // SECURITY: Add security headers
            .wrap(
                actix_middleware::DefaultHeaders::new()
                    .add(("X-Content-Type-Options", "nosniff"))
                    .add(("X-Frame-Options", "DENY"))
                    .add(("X-XSS-Protection", "1; mode=block"))
                    .add(("Referrer-Policy", "strict-origin-when-cross-origin"))
                    .add(("Permissions-Policy", "geolocation=(), microphone=(), camera=()"))
            )
            .wrap(actix_middleware::Compress::default())
            // Structured access log (replaces default Logger)
            .wrap(crate::middleware::AccessLog)
            // Request tracing with unique IDs
            .wrap(crate::middleware::RequestIdMiddleware)
            // SECURITY: Limit request body size to 4KB for JSON
            .app_data(web::JsonConfig::default().limit(4096))
            // Health check
            .route("/health", web::get().to(api::health::health_check))
            .route("/health/detailed", web::get().to(api::health::health_detailed))
            // Metrics endpoints
            .route("/metrics", web::get().to(api::health::get_metrics))
            .route("/metrics/prometheus", web::get().to(api::health::get_metrics_prometheus))
            // WebSocket endpoint
            .route("/ws", web::get().to(api::ws_handler))
            // API v1 routes
            .service(
                web::scope("/v1")
                    // Markets
                    .service(
                        web::scope("/markets")
                            .route("", web::get().to(api::markets::list_markets))
                            .route("", web::post().to(api::markets::create_market))
                            .route("/{market_id}", web::get().to(api::markets::get_market))
                            .route("/{market_id}/orderbook", web::get().to(api::markets::get_orderbook))
                            .route("/{market_id}/trades", web::get().to(api::markets::get_trades))
                    )
                    // Orders
                    .service(
                        web::scope("/orders")
                            .route("", web::get().to(api::orders::list_orders))
                            .route("", web::post().to(api::orders::place_order))
                            .route("/{order_id}", web::get().to(api::orders::get_order))
                            .route("/{order_id}", web::delete().to(api::orders::cancel_order))
                    )
                    // Positions
                    .service(
                        web::scope("/positions")
                            .route("", web::get().to(api::positions::list_positions))
                            .route("/{market_id}", web::get().to(api::positions::get_position))
                            .route("/{market_id}/claim", web::post().to(api::positions::claim_winnings))
                    )
                    // User
                    .service(
                        web::scope("/user")
                            .route("/profile", web::get().to(api::user::get_profile))
                            .route("/transactions", web::get().to(api::user::get_transactions))
                    )
                    // Wallet
                    .service(
                        web::scope("/wallet")
                            .route("/balance", web::get().to(api::wallet::get_balance))
                            .route("/deposit/address", web::get().to(api::wallet::get_deposit_address))
                            .route("/deposit", web::post().to(api::wallet::deposit))
                            .route("/withdraw", web::post().to(api::wallet::withdraw))
                    )
                    // Webhooks (no auth, signature verified)
                    .route("/webhooks/blindfold", web::post().to(api::wallet::blindfold_webhook))
                    // Authentication
                    .service(
                        web::scope("/auth")
                            .route("/nonce", web::get().to(api::auth::get_nonce))
                            .route("/login", web::post().to(api::auth::login))
                            .route("/refresh", web::post().to(api::auth::refresh_token))
                            .route("/logout", web::post().to(api::auth::logout))
                    )
            )
    })
    .bind(&bind_addr)?
    .run()
    .await
}
