pub mod health;
pub mod markets;
pub mod orders;
pub mod positions;
pub mod user;
pub mod wallet;
pub mod error;
pub mod auth;
pub mod jwt;
pub mod validation;
pub mod ws;
pub mod rate_limit;

pub use error::ApiError;
pub use jwt::JwtService;
pub use validation::{validate_market_id, validate_uuid, validate_order_price, validate_order_quantity, validate_pagination};
pub use ws::ws_handler;
pub use rate_limit::check_auth_rate_limit;

#[allow(unused_imports)]
pub use rate_limit::{
    check_write_rate_limit, check_order_rate_limit, check_market_create_rate_limit,
    check_claim_rate_limit, RateLimitTier,
};
