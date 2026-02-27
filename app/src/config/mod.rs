use log::warn;
use std::env;

#[derive(Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub redis_url: String,
    pub base_rpc_url: String,
    pub base_ws_url: String,
    pub base_chain_id: u64,
    pub siwe_domain: String,
    pub neura_token_address: String,
    pub market_core_address: String,
    pub order_book_address: String,
    pub agent_runtime_address: String,
    pub jwt_secret: String,
    pub cors_origins: Vec<String>,
    pub is_development: bool,
    pub evm_enabled: bool,
    pub evm_reads_enabled: bool,
    pub evm_writes_enabled: bool,
    pub blindfold_webhook_secret: String,
    pub program_vault_address: String,
    pub usdc_mint: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let is_development = env::var("ENVIRONMENT")
            .unwrap_or_else(|_| "development".to_string())
            .to_lowercase()
            == "development";

        let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| {
            if is_development {
                warn!(
                    "SECURITY WARNING: Using default JWT_SECRET. Set JWT_SECRET env var in production!"
                );
                "dev-secret-key-do-not-use-in-production".to_string()
            } else {
                panic!("SECURITY ERROR: JWT_SECRET environment variable must be set in production");
            }
        });

        if !is_development && jwt_secret.len() < 32 {
            panic!("SECURITY ERROR: JWT_SECRET must be at least 32 characters in production");
        }

        let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
            if is_development {
                warn!(
                    "SECURITY WARNING: Using default DATABASE_URL. Set DATABASE_URL env var in production!"
                );
                "postgres://postgres:password@localhost:5432/polyguard".to_string()
            } else {
                panic!("SECURITY ERROR: DATABASE_URL environment variable must be set in production");
            }
        });

        let cors_origins = env::var("CORS_ORIGINS")
            .unwrap_or_else(|_| {
                if is_development {
                    "*".to_string()
                } else {
                    "".to_string()
                }
            })
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let evm_enabled = env::var("EVM_ENABLED")
            .unwrap_or_else(|_| "true".to_string())
            .to_lowercase()
            == "true";

        Self {
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .expect("PORT must be a number"),
            database_url,
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            base_rpc_url: env::var("BASE_RPC_URL")
                .unwrap_or_else(|_| "https://mainnet.base.org".to_string()),
            base_ws_url: env::var("BASE_WS_URL")
                .unwrap_or_else(|_| "wss://mainnet.base.org".to_string()),
            base_chain_id: env::var("BASE_CHAIN_ID")
                .unwrap_or_else(|_| "8453".to_string())
                .parse()
                .expect("BASE_CHAIN_ID must be a number"),
            siwe_domain: env::var("SIWE_DOMAIN").unwrap_or_else(|_| "localhost:3000".to_string()),
            neura_token_address: env::var("NEURA_TOKEN_ADDRESS").unwrap_or_else(|_| "".to_string()),
            market_core_address: env::var("MARKET_CORE_ADDRESS").unwrap_or_else(|_| "".to_string()),
            order_book_address: env::var("ORDER_BOOK_ADDRESS").unwrap_or_else(|_| "".to_string()),
            agent_runtime_address: env::var("AGENT_RUNTIME_ADDRESS")
                .unwrap_or_else(|_| "".to_string()),
            jwt_secret,
            cors_origins,
            is_development,
            evm_enabled,
            evm_reads_enabled: env::var("EVM_READS_ENABLED")
                .unwrap_or_else(|_| if evm_enabled { "true" } else { "false" }.to_string())
                .to_lowercase()
                == "true",
            evm_writes_enabled: env::var("EVM_WRITES_ENABLED")
                .unwrap_or_else(|_| if evm_enabled { "true" } else { "false" }.to_string())
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
            usdc_mint: env::var("USDC_MINT").unwrap_or_else(|_| {
                "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".to_string()
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    fn with_clean_env<F, R>(f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let _guard = TEST_MUTEX.lock().unwrap();

        let env_vars = [
            "ENVIRONMENT",
            "JWT_SECRET",
            "DATABASE_URL",
            "CORS_ORIGINS",
            "HOST",
            "PORT",
            "REDIS_URL",
            "BASE_RPC_URL",
            "BASE_WS_URL",
            "BASE_CHAIN_ID",
            "SIWE_DOMAIN",
            "EVM_ENABLED",
            "EVM_READS_ENABLED",
            "EVM_WRITES_ENABLED",
            "ORDER_BOOK_ADDRESS",
            "AGENT_RUNTIME_ADDRESS",
        ];
        let saved: Vec<_> = env_vars
            .iter()
            .map(|k| (*k, std::env::var(*k).ok()))
            .collect();

        for k in &env_vars {
            std::env::remove_var(*k);
        }

        let result = f();

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
            assert!(config.evm_enabled);
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
