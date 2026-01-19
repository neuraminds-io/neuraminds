pub mod health;
pub mod markets;
pub mod orders;
pub mod positions;
pub mod user;
pub mod error;
pub mod auth;
pub mod jwt;
pub mod validation;

pub use error::ApiError;
pub use auth::AuthenticatedUser;
pub use jwt::{JwtService, Claims, UserRole, TokenPair, check_role};
pub use validation::{
    validate_wallet_address,
    validate_market_id,
    validate_uuid,
    validate_order_price,
    validate_order_quantity,
    validate_market_question,
    validate_description,
    validate_fee_bps,
    validate_trading_end,
    validate_pagination,
    validate_tx_signature,
    sanitize_string,
    limits,
};
