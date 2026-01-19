use actix_cors::Cors;
use actix_web::{web, App, HttpServer, middleware, http::header};
use actix_governor::{Governor, GovernorConfigBuilder};
use dotenv::dotenv;
use log::{info, warn};
use std::sync::Arc;

mod api;
mod config;
mod models;
mod services;

use api::JwtService;
use config::AppConfig;
use services::{DatabaseService, SolanaService, OrderBookService, RedisService};

pub struct AppState {
    pub config: AppConfig,
    pub db: DatabaseService,
    pub solana: SolanaService,
    pub orderbook: OrderBookService,
    pub redis: RedisService,
    pub jwt: JwtService,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

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

    let jwt = JwtService::new(&config.jwt_secret);

    let app_state = Arc::new(AppState {
        config: config.clone(),
        db,
        solana,
        orderbook,
        redis,
        jwt,
    });

    info!("Starting HTTP server on {}", bind_addr);

    // SECURITY: Configure rate limiting - 60 requests per minute per IP
    let governor_conf = GovernorConfigBuilder::default()
        .per_second(1)
        .burst_size(60)
        .finish()
        .unwrap();

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
            .wrap(cors)
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            // SECURITY: Limit request body size to 4KB for JSON
            .app_data(web::JsonConfig::default().limit(4096))
            // Health check
            .route("/health", web::get().to(api::health::health_check))
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
