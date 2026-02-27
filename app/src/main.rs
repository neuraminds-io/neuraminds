use actix_cors::Cors;
use actix_governor::{Governor, GovernorConfigBuilder};
use actix_web::{http::header, middleware as actix_middleware, web, App, HttpServer};
use dotenvy::dotenv;
use log::{info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

mod api;
mod config;
mod middleware;
mod models;
mod services;

use api::JwtService;
use config::AppConfig;
use services::{
    DatabaseService, EvmIndexerService, EvmRpcService, MetricsService, OrderBookService,
    RedisService, WebSocketHub,
};

pub struct AppState {
    pub config: AppConfig,
    pub db: DatabaseService,
    pub evm_rpc: EvmRpcService,
    pub evm_indexer: EvmIndexerService,
    pub orderbook: OrderBookService,
    pub redis: RedisService,
    pub jwt: JwtService,
    pub metrics: MetricsService,
    pub ws_hub: WebSocketHub,
    pub is_shutting_down: Arc<AtomicBool>,
}

async fn graceful_shutdown(state: Arc<AppState>) {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install CTRL+C handler");
    info!("Shutdown signal received, initiating graceful shutdown...");

    state.is_shutting_down.store(true, Ordering::SeqCst);

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

    let db = DatabaseService::new(&config.database_url)
        .await
        .expect("Failed to connect to database");

    let redis = RedisService::new(&config.redis_url)
        .await
        .expect("Failed to connect to Redis");
    let evm_rpc = EvmRpcService::new(&config.base_rpc_url);
    let evm_indexer = EvmIndexerService::new(evm_rpc.clone(), 20_000);

    let orderbook = OrderBookService::new();

    match db.load_orderbook_entries().await {
        Ok(entries) => {
            let count = entries.len();
            orderbook.restore_from_entries(entries);
            if count > 0 {
                info!("Restored {} order book entries from database", count);
            }
        }
        Err(e) => {
            warn!(
                "Failed to restore order book (table may not exist yet): {}",
                e
            );
        }
    }

    let jwt = JwtService::new(&config.jwt_secret);
    let metrics = MetricsService::new();
    let ws_hub = WebSocketHub::new();

    let app_state = Arc::new(AppState {
        config: config.clone(),
        db,
        evm_rpc,
        evm_indexer: evm_indexer.clone(),
        orderbook,
        redis,
        jwt,
        metrics,
        ws_hub,
        is_shutting_down: Arc::new(AtomicBool::new(false)),
    });

    if config.evm_enabled && config.evm_reads_enabled {
        let market_core = config.market_core_address.clone();
        let order_book = config.order_book_address.clone();
        let indexer = evm_indexer.clone();

        if market_core.is_empty() || order_book.is_empty() {
            warn!(
                "Skipping EVM indexer start: MARKET_CORE_ADDRESS or ORDER_BOOK_ADDRESS is missing"
            );
        } else {
            info!("Starting EVM log indexer background loop");
            tokio::spawn(async move {
                const TOPICS: [&str; 6] = [
                    "0x550857481380e1875f94e5eac6470eff69ecd368405067d9d5dfdf645d3d1f8e", // MarketCreated
                    "0xbc7c1013df472d2b00db2b9da4c476dbf8f0bc22116913d78750cf21d2c80fc2", // MarketResolved
                    "0xac1c16fb14f9a45ec49f65d268ff0d0f1945c504b82df54a9c6ad9f01b059be5", // OrderPlaced
                    "0x9384174c8517f5537b08e79211fc039e8a098571a3a2b4cb21dfa6f3237e8de1", // OrderCanceled
                    "0x5aac01386940f75e601757cfe5dc1d4ab2bac84f98d30664486114a8abb38a45", // OrderFilled
                    "0x93c1c30a0fa404e7a08a9f6a9d68323786a7e120f3adc0c16eb8855922e35dfa", // Claimed
                ];

                loop {
                    if let Err(err) = indexer
                        .sync(&market_core, &order_book, 25_000, &TOPICS)
                        .await
                    {
                        warn!("EVM indexer sync failed: {}", err);
                    }
                    tokio::time::sleep(tokio::time::Duration::from_secs(20)).await;
                }
            });
        }
    } else {
        info!("EVM indexer disabled by config toggles");
    }

    let shutdown_state = app_state.clone();
    tokio::spawn(async move {
        graceful_shutdown(shutdown_state).await;
    });

    info!("Starting HTTP server on {}", bind_addr);

    let governor_conf = GovernorConfigBuilder::default()
        .per_second(1)
        .burst_size(60)
        .finish()
        .expect("valid governor configuration");

    let config_clone = config.clone();

    HttpServer::new(move || {
        let cors = if config_clone.is_development {
            warn!("CORS: Development mode - allowing all origins");
            Cors::default()
                .allow_any_origin()
                .allowed_methods(vec!["GET", "POST", "DELETE", "OPTIONS"])
                .allowed_headers(vec![
                    header::AUTHORIZATION,
                    header::ACCEPT,
                    header::CONTENT_TYPE,
                ])
                .max_age(3600)
        } else {
            let mut cors = Cors::default()
                .allowed_methods(vec!["GET", "POST", "DELETE", "OPTIONS"])
                .allowed_headers(vec![
                    header::AUTHORIZATION,
                    header::ACCEPT,
                    header::CONTENT_TYPE,
                ])
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
            .wrap(Governor::new(&governor_conf))
            .wrap(crate::middleware::GeoBlock::new(
                !config_clone.is_development,
            ))
            .wrap(cors)
            .wrap(
                actix_middleware::DefaultHeaders::new()
                    .add(("X-Content-Type-Options", "nosniff"))
                    .add(("X-Frame-Options", "DENY"))
                    .add(("X-XSS-Protection", "1; mode=block"))
                    .add(("Referrer-Policy", "strict-origin-when-cross-origin"))
                    .add((
                        "Permissions-Policy",
                        "geolocation=(), microphone=(), camera=()",
                    )),
            )
            .wrap(actix_middleware::Compress::default())
            .wrap(crate::middleware::AccessLog)
            .wrap(crate::middleware::RequestIdMiddleware)
            .app_data(web::JsonConfig::default().limit(4096))
            .route("/health", web::get().to(api::health::health_check))
            .route(
                "/health/detailed",
                web::get().to(api::health::health_detailed),
            )
            .route("/metrics", web::get().to(api::health::get_metrics))
            .route(
                "/metrics/prometheus",
                web::get().to(api::health::get_metrics_prometheus),
            )
            .route("/ws", web::get().to(api::ws_handler))
            .service(
                web::scope("/v1")
                    .service(
                        web::scope("/markets")
                            .route("", web::get().to(api::markets::list_markets))
                            .route("", web::post().to(api::markets::create_market))
                            .route("/{market_id}", web::get().to(api::markets::get_market))
                            .route(
                                "/{market_id}/orderbook",
                                web::get().to(api::markets::get_orderbook),
                            )
                            .route(
                                "/{market_id}/trades",
                                web::get().to(api::markets::get_trades),
                            ),
                    )
                    .service(
                        web::scope("/orders")
                            .route("", web::get().to(api::orders::list_orders))
                            .route("", web::post().to(api::orders::place_order))
                            .route("/{order_id}", web::get().to(api::orders::get_order))
                            .route("/{order_id}", web::delete().to(api::orders::cancel_order)),
                    )
                    .service(
                        web::scope("/positions")
                            .route("", web::get().to(api::positions::list_positions))
                            .route("/{market_id}", web::get().to(api::positions::get_position))
                            .route(
                                "/{market_id}/claim",
                                web::post().to(api::positions::claim_winnings),
                            ),
                    )
                    .service(
                        web::scope("/user")
                            .route("/profile", web::get().to(api::user::get_profile))
                            .route("/transactions", web::get().to(api::user::get_transactions)),
                    )
                    .service(
                        web::scope("/wallet")
                            .route("/balance", web::get().to(api::wallet::get_balance))
                            .route(
                                "/deposit/address",
                                web::get().to(api::wallet::get_deposit_address),
                            )
                            .route("/deposit", web::post().to(api::wallet::deposit))
                            .route("/withdraw", web::post().to(api::wallet::withdraw)),
                    )
                    .route(
                        "/webhooks/blindfold",
                        web::post().to(api::wallet::blindfold_webhook),
                    )
                    .service(
                        web::scope("/auth")
                            .route("/nonce", web::get().to(api::auth::get_nonce))
                            .route("/login", web::post().to(api::auth::login))
                            .route("/siwe/nonce", web::get().to(api::auth::get_siwe_nonce))
                            .route("/siwe/login", web::post().to(api::auth::siwe_login))
                            .route("/refresh", web::post().to(api::auth::refresh_token))
                            .route("/logout", web::post().to(api::auth::logout)),
                    )
                    .service(
                        web::scope("/evm")
                            .route("/markets", web::get().to(api::evm::get_base_markets))
                            .route(
                                "/markets/{market_id}/orderbook",
                                web::get().to(api::evm::get_base_orderbook),
                            )
                            .route(
                                "/markets/{market_id}/trades",
                                web::get().to(api::evm::get_base_trades),
                            )
                            .route(
                                "/token/state",
                                web::get().to(api::evm::get_neura_token_state),
                            )
                            .service(
                                web::scope("/write")
                                    .route(
                                        "/markets/create",
                                        web::post().to(api::evm::prepare_create_market_write),
                                    )
                                    .route(
                                        "/orders/place",
                                        web::post().to(api::evm::prepare_place_order_write),
                                    )
                                    .route(
                                        "/orders/cancel",
                                        web::post().to(api::evm::prepare_cancel_order_write),
                                    )
                                    .route(
                                        "/orders/match",
                                        web::post().to(api::evm::prepare_match_orders_write),
                                    )
                                    .route(
                                        "/positions/claim",
                                        web::post().to(api::evm::prepare_claim_write),
                                    )
                                    .route(
                                        "/agents/create",
                                        web::post().to(api::evm::prepare_create_agent_write),
                                    )
                                    .route(
                                        "/agents/execute",
                                        web::post().to(api::evm::prepare_execute_agent_write),
                                    )
                                    .route("/relay", web::post().to(api::evm::relay_raw_transaction)),
                            ),
                    ),
            )
    })
    .bind(&bind_addr)?
    .run()
    .await
}
