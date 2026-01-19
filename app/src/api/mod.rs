pub mod health;
pub mod markets;
pub mod orders;
pub mod positions;
pub mod user;
pub mod error;
pub mod auth;
pub mod jwt;

pub use error::ApiError;
pub use auth::AuthenticatedUser;
pub use jwt::{JwtService, Claims, UserRole, TokenPair, check_role};
