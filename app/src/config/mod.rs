use log::warn;
use std::env;

#[derive(Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub redis_url: String,
    pub solana_rpc_url: String,
    pub solana_ws_url: String,
    pub base_rpc_url: String,
    pub base_ws_url: String,
    pub base_chain_id: u64,
    pub siwe_domain: String,
    pub keeper_keypair_path: String,
    pub market_program_id: String,
    pub orderbook_program_id: String,
    pub privacy_program_id: String,
    pub neura_token_address: String,
    pub market_core_address: String,
    pub order_book_address: String,
    pub jwt_secret: String,
    /// Allowed CORS origins (comma-separated)
    pub cors_origins: Vec<String>,
    /// Whether running in development mode (enables unsafe defaults)
    pub is_development: bool,
    /// Whether to submit transactions to Solana (disable for testing without RPC)
    pub solana_enabled: bool,
    /// Whether to submit transactions to Base (disable until EVM path is enabled)
    pub evm_enabled: bool,
    /// Blindfold Finance webhook secret for signature verification
    pub blindfold_webhook_secret: String,
    /// Program vault address for USDC deposits
    pub program_vault_address: String,
    /// USDC mint address
    pub usdc_mint: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let is_development = env::var("ENVIRONMENT")
            .unwrap_or_else(|_| "development".to_string())
            .to_lowercase()
            == "development";

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
            base_rpc_url: env::var("BASE_RPC_URL")
                .unwrap_or_else(|_| "https://sepolia.base.org".to_string()),
            base_ws_url: env::var("BASE_WS_URL")
                .unwrap_or_else(|_| "wss://sepolia.base.org".to_string()),
            base_chain_id: env::var("BASE_CHAIN_ID")
                .unwrap_or_else(|_| "84532".to_string())
                .parse()
                .expect("BASE_CHAIN_ID must be a number"),
            siwe_domain: env::var("SIWE_DOMAIN").unwrap_or_else(|_| "localhost:3000".to_string()),
            keeper_keypair_path: env::var("KEEPER_KEYPAIR_PATH")
                .unwrap_or_else(|_| "./keys/keeper.json".to_string()),
            market_program_id: env::var("MARKET_PROGRAM_ID")
                .unwrap_or_else(|_| "98jqxMe88XGjXzCY3bwV1Kuqzj32fcwdhPZa193RUffQ".to_string()),
            orderbook_program_id: env::var("ORDERBOOK_PROGRAM_ID")
                .unwrap_or_else(|_| "59LqZtVU2YBrhv8B2E1iASJMzcyBHWhY2JuaJsCXkAS8".to_string()),
            privacy_program_id: env::var("PRIVACY_PROGRAM_ID")
                .unwrap_or_else(|_| "9QGtHZJvmjMKTME1s3mVfNXtGpEdXDQZJTxsxqve9GsL".to_string()),
            neura_token_address: env::var("NEURA_TOKEN_ADDRESS").unwrap_or_else(|_| "".to_string()),
            market_core_address: env::var("MARKET_CORE_ADDRESS").unwrap_or_else(|_| "".to_string()),
            order_book_address: env::var("ORDER_BOOK_ADDRESS").unwrap_or_else(|_| "".to_string()),
            jwt_secret,
            cors_origins,
            is_development,
            solana_enabled: env::var("SOLANA_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .to_lowercase()
                == "true",
            evm_enabled: env::var("EVM_ENABLED")
                .unwrap_or_else(|_| "false".to_string())
                .to_lowercase()
                == "true",
            blindfold_webhook_secret: env::var("BLINDFOLD_WEBHOOK_SECRET").unwrap_or_else(|_| {
                if is_development {
                    warn!("SECURITY WARNING: Using default BLINDFOLD_WEBHOOK_SECRET");
                    "dev-blindfold-secret".to_string()
                } else {
                    panic!("SECURITY ERROR: BLINDFOLD_WEBHOOK_SECRET must be set in production");
                }
            }),
            program_vault_address: env::var("PROGRAM_VAULT_ADDRESS")
                .unwrap_or_else(|_| "".to_string()),
            usdc_mint: env::var("USDC_MINT")
                .unwrap_or_else(|_| "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Global mutex to ensure env var tests run serially
    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    fn with_clean_env<F, R>(f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let _guard = TEST_MUTEX.lock().unwrap();

        // Save current values
        let env_vars = [
            "ENVIRONMENT",
            "JWT_SECRET",
            "DATABASE_URL",
            "CORS_ORIGINS",
            "HOST",
            "PORT",
            "REDIS_URL",
            "SOLANA_RPC_URL",
            "SOLANA_WS_URL",
            "SOLANA_ENABLED",
            "BASE_RPC_URL",
            "BASE_WS_URL",
            "BASE_CHAIN_ID",
            "SIWE_DOMAIN",
            "EVM_ENABLED",
            "ORDER_BOOK_ADDRESS",
        ];
        let saved: Vec<_> = env_vars
            .iter()
            .map(|k| (*k, std::env::var(*k).ok()))
            .collect();

        // Clear all
        for k in &env_vars {
            std::env::remove_var(*k);
        }

        let result = f();

        // Restore
        for (k, v) in saved {
            match v {
                Some(val) => std::env::set_var(k, val),
                None => std::env::remove_var(k),
            }
        }

        result
    }

    #[test]
    fn test_development_mode_defaults() {
        with_clean_env(|| {
            std::env::set_var("ENVIRONMENT", "development");

            let config = AppConfig::from_env();

            assert!(config.is_development);
            assert_eq!(config.host, "0.0.0.0");
            assert_eq!(config.port, 8080);
            assert!(config.cors_origins.contains(&"*".to_string()));
        });
    }

    #[test]
    fn test_cors_origins_parsing() {
        with_clean_env(|| {
            std::env::set_var("ENVIRONMENT", "development");
            std::env::set_var(
                "CORS_ORIGINS",
                "http://localhost:3000, https://app.example.com",
            );

            let config = AppConfig::from_env();

            assert_eq!(config.cors_origins.len(), 2);
            assert!(config
                .cors_origins
                .contains(&"http://localhost:3000".to_string()));
            assert!(config
                .cors_origins
                .contains(&"https://app.example.com".to_string()));
        });
    }

    #[test]
    fn test_cors_origins_empty_string_filtered() {
        with_clean_env(|| {
            std::env::set_var("ENVIRONMENT", "development");
            std::env::set_var("CORS_ORIGINS", "");

            let config = AppConfig::from_env();

            assert!(config.cors_origins.is_empty());
        });
    }

    #[test]
    fn test_solana_enabled_true() {
        with_clean_env(|| {
            std::env::set_var("ENVIRONMENT", "development");
            std::env::set_var("SOLANA_ENABLED", "true");

            let config = AppConfig::from_env();
            assert!(config.solana_enabled);
        });
    }

    #[test]
    fn test_solana_enabled_false() {
        with_clean_env(|| {
            std::env::set_var("ENVIRONMENT", "development");
            std::env::set_var("SOLANA_ENABLED", "false");

            let config = AppConfig::from_env();
            assert!(!config.solana_enabled);
        });
    }

    #[test]
    fn test_solana_enabled_case_insensitive() {
        with_clean_env(|| {
            std::env::set_var("ENVIRONMENT", "development");
            std::env::set_var("SOLANA_ENABLED", "TRUE");

            let config = AppConfig::from_env();
            assert!(config.solana_enabled);
        });
    }

    #[test]
    fn test_custom_host_and_port() {
        with_clean_env(|| {
            std::env::set_var("ENVIRONMENT", "development");
            std::env::set_var("HOST", "127.0.0.1");
            std::env::set_var("PORT", "3000");

            let config = AppConfig::from_env();

            assert_eq!(config.host, "127.0.0.1");
            assert_eq!(config.port, 3000);
        });
    }

    #[test]
    fn test_custom_redis_url() {
        with_clean_env(|| {
            std::env::set_var("ENVIRONMENT", "development");
            std::env::set_var("REDIS_URL", "redis://custom:6380");

            let config = AppConfig::from_env();

            assert_eq!(config.redis_url, "redis://custom:6380");
        });
    }

    #[test]
    fn test_custom_solana_urls() {
        with_clean_env(|| {
            std::env::set_var("ENVIRONMENT", "development");
            std::env::set_var("SOLANA_RPC_URL", "https://custom.rpc.com");
            std::env::set_var("SOLANA_WS_URL", "wss://custom.rpc.com");

            let config = AppConfig::from_env();

            assert_eq!(config.solana_rpc_url, "https://custom.rpc.com");
            assert_eq!(config.solana_ws_url, "wss://custom.rpc.com");
        });
    }

    #[test]
    fn test_custom_base_urls_and_chain_id() {
        with_clean_env(|| {
            std::env::set_var("ENVIRONMENT", "development");
            std::env::set_var("BASE_RPC_URL", "https://base.rpc.example");
            std::env::set_var("BASE_WS_URL", "wss://base.rpc.example");
            std::env::set_var("BASE_CHAIN_ID", "8453");
            std::env::set_var("SIWE_DOMAIN", "app.neuralminds.example");

            let config = AppConfig::from_env();

            assert_eq!(config.base_rpc_url, "https://base.rpc.example");
            assert_eq!(config.base_ws_url, "wss://base.rpc.example");
            assert_eq!(config.base_chain_id, 8453);
            assert_eq!(config.siwe_domain, "app.neuralminds.example");
        });
    }

    #[test]
    fn test_evm_enabled_flag() {
        with_clean_env(|| {
            std::env::set_var("ENVIRONMENT", "development");
            std::env::set_var("EVM_ENABLED", "true");

            let config = AppConfig::from_env();
            assert!(config.evm_enabled);
        });
    }

    #[test]
    fn test_environment_case_insensitive() {
        with_clean_env(|| {
            std::env::set_var("ENVIRONMENT", "DEVELOPMENT");

            let config = AppConfig::from_env();
            assert!(config.is_development);
        });
    }
}
