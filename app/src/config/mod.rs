use std::env;
use log::warn;

#[derive(Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub redis_url: String,
    pub solana_rpc_url: String,
    pub solana_ws_url: String,
    pub keeper_keypair_path: String,
    pub market_program_id: String,
    pub orderbook_program_id: String,
    pub privacy_program_id: String,
    pub jwt_secret: String,
    /// Allowed CORS origins (comma-separated)
    pub cors_origins: Vec<String>,
    /// Whether running in development mode (enables unsafe defaults)
    pub is_development: bool,
    /// Whether to submit transactions to Solana (disable for testing without RPC)
    pub solana_enabled: bool,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let is_development = env::var("ENVIRONMENT")
            .unwrap_or_else(|_| "development".to_string())
            .to_lowercase() == "development";

        // SECURITY: In production, require critical env vars to be set explicitly
        let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| {
            if is_development {
                warn!("SECURITY WARNING: Using default JWT_SECRET. Set JWT_SECRET env var in production!");
                "dev-secret-key-do-not-use-in-production".to_string()
            } else {
                panic!("SECURITY ERROR: JWT_SECRET environment variable must be set in production");
            }
        });

        // Validate JWT secret strength
        if !is_development && jwt_secret.len() < 32 {
            panic!("SECURITY ERROR: JWT_SECRET must be at least 32 characters in production");
        }

        let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
            if is_development {
                warn!("SECURITY WARNING: Using default DATABASE_URL. Set DATABASE_URL env var in production!");
                "postgres://postgres:password@localhost:5432/polyguard".to_string()
            } else {
                panic!("SECURITY ERROR: DATABASE_URL environment variable must be set in production");
            }
        });

        // Parse CORS origins
        let cors_origins = env::var("CORS_ORIGINS")
            .unwrap_or_else(|_| {
                if is_development {
                    "*".to_string() // Allow all in development
                } else {
                    "".to_string() // Empty = no origins allowed by default
                }
            })
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Self {
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .expect("PORT must be a number"),
            database_url,
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            solana_rpc_url: env::var("SOLANA_RPC_URL")
                .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string()),
            solana_ws_url: env::var("SOLANA_WS_URL")
                .unwrap_or_else(|_| "wss://api.devnet.solana.com".to_string()),
            keeper_keypair_path: env::var("KEEPER_KEYPAIR_PATH")
                .unwrap_or_else(|_| "./keys/keeper.json".to_string()),
            market_program_id: env::var("MARKET_PROGRAM_ID")
                .unwrap_or_else(|_| "98jqxMe88XGjXzCY3bwV1Kuqzj32fcwdhPZa193RUffQ".to_string()),
            orderbook_program_id: env::var("ORDERBOOK_PROGRAM_ID")
                .unwrap_or_else(|_| "59LqZtVU2YBrhv8B2E1iASJMzcyBHWhY2JuaJsCXkAS8".to_string()),
            privacy_program_id: env::var("PRIVACY_PROGRAM_ID")
                .unwrap_or_else(|_| "9QGtHZJvmjMKTME1s3mVfNXtGpEdXDQZJTxsxqve9GsL".to_string()),
            jwt_secret,
            cors_origins,
            is_development,
            solana_enabled: env::var("SOLANA_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .to_lowercase() == "true",
        }
    }
}
