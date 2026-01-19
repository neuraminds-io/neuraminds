use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use crate::api::ApiError;

/// JWT token claims
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// Subject (wallet address)
    pub sub: String,
    /// Issued at timestamp (Unix seconds)
    pub iat: i64,
    /// Expiration timestamp (Unix seconds)
    pub exp: i64,
    /// User role
    pub role: UserRole,
    /// Token ID for revocation tracking
    pub jti: String,
}

/// User roles for RBAC
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    /// Regular user - can trade and manage their own orders/positions
    User,
    /// Keeper - can settle trades and manage order book
    Keeper,
    /// Admin - full access to all operations
    Admin,
}

impl Default for UserRole {
    fn default() -> Self {
        UserRole::User
    }
}

/// Token expiration times
pub const ACCESS_TOKEN_EXPIRATION_HOURS: i64 = 1;
pub const REFRESH_TOKEN_EXPIRATION_DAYS: i64 = 7;

/// Token type for different purposes
#[derive(Debug, Clone, Copy)]
pub enum TokenType {
    Access,
    Refresh,
}

/// JWT service for token management
pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtService {
    /// Create a new JWT service with the given secret
    pub fn new(secret: &str) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
        }
    }

    /// Generate an access token for a user
    pub fn generate_access_token(
        &self,
        wallet_address: &str,
        role: UserRole,
    ) -> Result<String, ApiError> {
        self.generate_token(wallet_address, role, TokenType::Access)
    }

    /// Generate a refresh token for a user
    pub fn generate_refresh_token(
        &self,
        wallet_address: &str,
        role: UserRole,
    ) -> Result<String, ApiError> {
        self.generate_token(wallet_address, role, TokenType::Refresh)
    }

    /// Generate a token of the specified type
    fn generate_token(
        &self,
        wallet_address: &str,
        role: UserRole,
        token_type: TokenType,
    ) -> Result<String, ApiError> {
        let now = Utc::now();

        let expiration = match token_type {
            TokenType::Access => now + Duration::hours(ACCESS_TOKEN_EXPIRATION_HOURS),
            TokenType::Refresh => now + Duration::days(REFRESH_TOKEN_EXPIRATION_DAYS),
        };

        // Generate unique token ID
        let jti = format!("{:x}{:016x}", now.timestamp_nanos_opt().unwrap_or(0), rand::random::<u64>());

        let claims = Claims {
            sub: wallet_address.to_string(),
            iat: now.timestamp(),
            exp: expiration.timestamp(),
            role,
            jti,
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| {
                log::error!("Failed to generate JWT: {}", e);
                ApiError::internal("Failed to generate authentication token")
            })
    }

    /// Validate and decode a token
    pub fn validate_token(&self, token: &str) -> Result<Claims, ApiError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;

        decode::<Claims>(token, &self.decoding_key, &validation)
            .map(|data| data.claims)
            .map_err(|e| {
                log::debug!("JWT validation failed: {}", e);
                match e.kind() {
                    jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                        ApiError::unauthorized("Token expired")
                    }
                    jsonwebtoken::errors::ErrorKind::InvalidToken => {
                        ApiError::unauthorized("Invalid token format")
                    }
                    jsonwebtoken::errors::ErrorKind::InvalidSignature => {
                        ApiError::unauthorized("Invalid token signature")
                    }
                    _ => ApiError::unauthorized("Invalid token")
                }
            })
    }

    /// Extract claims without full validation (for debugging/introspection)
    /// WARNING: Do not use for authentication - this skips signature verification
    #[allow(dead_code)]
    pub fn decode_claims_unsafe(&self, token: &str) -> Result<Claims, ApiError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.insecure_disable_signature_validation();
        validation.validate_exp = false;

        decode::<Claims>(token, &self.decoding_key, &validation)
            .map(|data| data.claims)
            .map_err(|_| ApiError::unauthorized("Invalid token"))
    }
}

/// Token pair response for authentication endpoints
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

impl TokenPair {
    pub fn new(access_token: String, refresh_token: String) -> Self {
        Self {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: ACCESS_TOKEN_EXPIRATION_HOURS * 3600,
        }
    }
}

/// Check if user has required role
pub fn check_role(user_role: UserRole, required_role: UserRole) -> Result<(), ApiError> {
    let has_access = match required_role {
        UserRole::User => true, // Everyone can access user-level resources
        UserRole::Keeper => matches!(user_role, UserRole::Keeper | UserRole::Admin),
        UserRole::Admin => matches!(user_role, UserRole::Admin),
    };

    if has_access {
        Ok(())
    } else {
        Err(ApiError::forbidden("Insufficient permissions"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_jwt_service() -> JwtService {
        JwtService::new("test-secret-key-for-unit-tests-only-12345678")
    }

    #[test]
    fn test_generate_and_validate_access_token() {
        let service = test_jwt_service();
        let wallet = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";

        let token = service.generate_access_token(wallet, UserRole::User).unwrap();
        let claims = service.validate_token(&token).unwrap();

        assert_eq!(claims.sub, wallet);
        assert_eq!(claims.role, UserRole::User);
    }

    #[test]
    fn test_generate_and_validate_refresh_token() {
        let service = test_jwt_service();
        let wallet = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";

        let token = service.generate_refresh_token(wallet, UserRole::Admin).unwrap();
        let claims = service.validate_token(&token).unwrap();

        assert_eq!(claims.sub, wallet);
        assert_eq!(claims.role, UserRole::Admin);
    }

    #[test]
    fn test_invalid_token() {
        let service = test_jwt_service();

        let result = service.validate_token("invalid.token.here");
        assert!(result.is_err());
    }

    #[test]
    fn test_tampered_token() {
        let service = test_jwt_service();
        let wallet = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";

        let token = service.generate_access_token(wallet, UserRole::User).unwrap();

        // Tamper with the token
        let tampered = format!("{}x", token);

        let result = service.validate_token(&tampered);
        assert!(result.is_err());
    }

    #[test]
    fn test_check_role_user() {
        // Users can access user resources
        assert!(check_role(UserRole::User, UserRole::User).is_ok());
        // Keepers can access user resources
        assert!(check_role(UserRole::Keeper, UserRole::User).is_ok());
        // Admins can access user resources
        assert!(check_role(UserRole::Admin, UserRole::User).is_ok());
    }

    #[test]
    fn test_check_role_keeper() {
        // Users cannot access keeper resources
        assert!(check_role(UserRole::User, UserRole::Keeper).is_err());
        // Keepers can access keeper resources
        assert!(check_role(UserRole::Keeper, UserRole::Keeper).is_ok());
        // Admins can access keeper resources
        assert!(check_role(UserRole::Admin, UserRole::Keeper).is_ok());
    }

    #[test]
    fn test_check_role_admin() {
        // Users cannot access admin resources
        assert!(check_role(UserRole::User, UserRole::Admin).is_err());
        // Keepers cannot access admin resources
        assert!(check_role(UserRole::Keeper, UserRole::Admin).is_err());
        // Only admins can access admin resources
        assert!(check_role(UserRole::Admin, UserRole::Admin).is_ok());
    }

    #[test]
    fn test_unique_token_ids() {
        let service = test_jwt_service();
        let wallet = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";

        let token1 = service.generate_access_token(wallet, UserRole::User).unwrap();
        let token2 = service.generate_access_token(wallet, UserRole::User).unwrap();

        let claims1 = service.validate_token(&token1).unwrap();
        let claims2 = service.validate_token(&token2).unwrap();

        // Token IDs should be unique
        assert_ne!(claims1.jti, claims2.jti);
    }
}
