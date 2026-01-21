use actix_web::{HttpRequest, HttpResponse, web};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use std::str::FromStr;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use crate::AppState;
use crate::api::{ApiError, jwt::{UserRole, TokenPair}, check_auth_rate_limit};

/// Message expiration time in seconds (5 minutes)
const MESSAGE_EXPIRATION_SECS: u64 = 300;

/// Maximum age for nonce cleanup (10 minutes)
const NONCE_CLEANUP_AGE_SECS: u64 = 600;

/// Represents an authenticated user
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    /// The user's wallet address (Solana pubkey)
    pub wallet_address: String,
}

/// Signed authentication message format
/// Expected message format: "polyguard:{wallet}:{timestamp}:{nonce}"
#[derive(Debug)]
struct AuthMessage {
    wallet: String,
    timestamp: u64,
    nonce: String,
}

impl AuthMessage {
    /// Parse message from string format
    fn parse(message: &str) -> Result<Self, ApiError> {
        let parts: Vec<&str> = message.split(':').collect();
        if parts.len() != 4 {
            return Err(ApiError::unauthorized(
                "Invalid message format. Expected: polyguard:{wallet}:{timestamp}:{nonce}"
            ));
        }

        if parts[0] != "polyguard" {
            return Err(ApiError::unauthorized("Invalid message prefix"));
        }

        let timestamp = parts[2].parse::<u64>()
            .map_err(|_| ApiError::unauthorized("Invalid timestamp in message"))?;

        Ok(Self {
            wallet: parts[1].to_string(),
            timestamp,
            nonce: parts[3].to_string(),
        })
    }

    /// Reconstruct the original message for verification
    fn to_string(&self) -> String {
        format!("polyguard:{}:{}:{}", self.wallet, self.timestamp, self.nonce)
    }
}

/// Extracts the authenticated user from the request (async version)
///
/// Authentication is done via wallet signature verification:
/// - Authorization header format: "Bearer <wallet_pubkey>:<signature>:<message>"
/// - The signature proves ownership of the wallet
/// - Message format: "polyguard:{wallet}:{timestamp}:{nonce}"
///
/// Nonce checking uses Redis for distributed, persistent storage.
pub async fn extract_authenticated_user(
    req: &HttpRequest,
    state: &web::Data<Arc<AppState>>,
) -> Result<AuthenticatedUser, ApiError> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| ApiError::unauthorized("Missing Authorization header"))?;

    // Check for Bearer prefix
    if !auth_header.starts_with("Bearer ") {
        return Err(ApiError::unauthorized("Invalid Authorization header format"));
    }

    let token = &auth_header[7..]; // Skip "Bearer "

    // Production mode: require signature verification
    // Format: "Bearer <wallet_pubkey>:<signature_base58>:<message>"
    let parts: Vec<&str> = token.splitn(3, ':').collect();
    if parts.len() != 3 {
        return Err(ApiError::unauthorized(
            "Invalid token format. Expected: wallet_pubkey:signature:message"
        ));
    }

    let wallet_pubkey = parts[0];
    let signature_str = parts[1];
    let message = parts[2];

    // Validate wallet address format
    validate_solana_address(wallet_pubkey)?;

    // Parse and validate the message
    let auth_message = AuthMessage::parse(message)?;

    // Verify wallet in message matches the claimed wallet
    if auth_message.wallet != wallet_pubkey {
        return Err(ApiError::unauthorized("Wallet address mismatch in message"));
    }

    // Verify timestamp (message not expired)
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ApiError::internal("System time error"))?
        .as_secs();

    if current_time > auth_message.timestamp + MESSAGE_EXPIRATION_SECS {
        return Err(ApiError::unauthorized("Authentication message expired"));
    }

    // Allow some clock skew (5 seconds into future)
    if auth_message.timestamp > current_time + 5 {
        return Err(ApiError::unauthorized("Authentication message timestamp in future"));
    }

    // Check nonce for replay protection (using Redis)
    check_and_record_nonce_redis(&state.redis, &auth_message.nonce).await?;

    // Verify the Ed25519 signature
    if !verify_ed25519_signature(wallet_pubkey, signature_str, message) {
        return Err(ApiError::unauthorized("Invalid signature"));
    }

    log::info!("Authenticated user: {}", wallet_pubkey);

    Ok(AuthenticatedUser {
        wallet_address: wallet_pubkey.to_string(),
    })
}

/// Validates that a string is a valid Solana address (base58, 32-44 chars)
fn validate_solana_address(address: &str) -> Result<(), ApiError> {
    // Solana addresses are base58 encoded and typically 32-44 characters
    if address.len() < 32 || address.len() > 44 {
        return Err(ApiError::bad_request(
            "INVALID_WALLET",
            "Invalid wallet address length"
        ));
    }

    // Check that all characters are valid base58
    const BASE58_CHARS: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    if !address.chars().all(|c| BASE58_CHARS.contains(c)) {
        return Err(ApiError::bad_request(
            "INVALID_WALLET",
            "Invalid wallet address format"
        ));
    }

    // Also validate it's a valid Solana pubkey
    Pubkey::from_str(address)
        .map_err(|_| ApiError::bad_request("INVALID_WALLET", "Invalid Solana public key"))?;

    Ok(())
}

/// Verifies an Ed25519 signature from a Solana wallet
///
/// The signature should be produced by signing the message bytes with the wallet's
/// private key using standard Ed25519 signing.
fn verify_ed25519_signature(wallet_pubkey: &str, signature_str: &str, message: &str) -> bool {
    // Parse the public key
    let pubkey = match Pubkey::from_str(wallet_pubkey) {
        Ok(pk) => pk,
        Err(e) => {
            log::warn!("Failed to parse public key: {}", e);
            return false;
        }
    };

    // Parse the signature (base58 encoded)
    let signature = match Signature::from_str(signature_str) {
        Ok(sig) => sig,
        Err(e) => {
            log::warn!("Failed to parse signature: {}", e);
            return false;
        }
    };

    // Verify the signature against the message bytes
    // Solana wallets sign raw bytes, not hashes
    let message_bytes = message.as_bytes();

    // Perform the Ed25519 verification
    match signature.verify(pubkey.as_ref(), message_bytes) {
        true => {
            log::debug!("Signature verified successfully for {}", wallet_pubkey);
            true
        }
        false => {
            log::warn!("Signature verification failed for {}", wallet_pubkey);
            false
        }
    }
}

/// Check if nonce has been used and record it for replay protection
/// Uses Redis for distributed, persistent nonce storage
async fn check_and_record_nonce_redis(
    redis: &crate::services::RedisService,
    nonce: &str,
) -> Result<(), ApiError> {
    // Nonce TTL: 10 minutes (matches NONCE_CLEANUP_AGE_SECS)
    let was_new = redis.check_and_record_nonce(nonce, NONCE_CLEANUP_AGE_SECS)
        .await
        .map_err(|e| {
            log::error!("Redis nonce check failed: {}", e);
            ApiError::internal("Nonce verification failed")
        })?;

    if !was_new {
        log::warn!("Replay attack detected: nonce {} already used", nonce);
        return Err(ApiError::unauthorized("Nonce already used (possible replay attack)"));
    }

    Ok(())
}

/// Generate a unique nonce for the client to use
/// This can be called via an API endpoint
pub fn generate_nonce() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    // Combine timestamp with random bytes for uniqueness
    let random_part: u64 = rand::random();

    format!("{:x}{:016x}", timestamp, random_part)
}

/// Helper macro to extract authenticated user in handlers
///
/// Usage:
/// ```
/// pub async fn my_handler(
///     req: HttpRequest,
///     state: web::Data<Arc<AppState>>,
/// ) -> Result<impl Responder, ApiError> {
///     let user = require_auth!(&req, &state);
///     // ... rest of handler
/// }
/// ```
#[macro_export]
macro_rules! require_auth {
    ($req:expr, $state:expr) => {
        crate::api::auth::extract_authenticated_user($req, $state).await?
    };
}

// ============================================================================
// Authentication Endpoints
// ============================================================================

/// Response for nonce endpoint
#[derive(Serialize)]
pub struct NonceResponse {
    pub nonce: String,
    pub expires_at: u64,
}

/// Request for login endpoint
#[derive(Deserialize)]
pub struct LoginRequest {
    /// The wallet public key (base58)
    pub wallet: String,
    /// The signature of the message (base58)
    pub signature: String,
    /// The signed message (format: polyguard:{wallet}:{timestamp}:{nonce})
    pub message: String,
}

/// Request for token refresh
#[derive(Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

/// GET /v1/auth/nonce
/// Returns a unique nonce for the client to include in their signed message
pub async fn get_nonce() -> Result<HttpResponse, ApiError> {
    let nonce = generate_nonce();
    let expires_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ApiError::internal("System time error"))?
        .as_secs() + MESSAGE_EXPIRATION_SECS;

    Ok(HttpResponse::Ok().json(NonceResponse { nonce, expires_at }))
}

/// POST /v1/auth/login
/// Authenticates a user with their wallet signature and returns JWT tokens
pub async fn login(
    http_req: HttpRequest,
    state: web::Data<Arc<AppState>>,
    body: web::Json<LoginRequest>,
) -> Result<HttpResponse, ApiError> {
    // Rate limit: 10 login attempts per minute per IP
    check_auth_rate_limit(&http_req, &state.redis).await?;

    let req = body.into_inner();

    // Validate wallet address
    validate_solana_address(&req.wallet)?;

    // Parse and validate the message
    let auth_message = AuthMessage::parse(&req.message)?;

    // Verify wallet in message matches the claimed wallet
    if auth_message.wallet != req.wallet {
        return Err(ApiError::unauthorized("Wallet address mismatch in message"));
    }

    // Verify timestamp (message not expired)
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ApiError::internal("System time error"))?
        .as_secs();

    if current_time > auth_message.timestamp + MESSAGE_EXPIRATION_SECS {
        return Err(ApiError::unauthorized("Authentication message expired"));
    }

    if auth_message.timestamp > current_time + 5 {
        return Err(ApiError::unauthorized("Authentication message timestamp in future"));
    }

    // Check nonce for replay protection (using Redis for distributed storage)
    check_and_record_nonce_redis(&state.redis, &auth_message.nonce).await?;

    // Verify the Ed25519 signature
    if !verify_ed25519_signature(&req.wallet, &req.signature, &req.message) {
        return Err(ApiError::unauthorized("Invalid signature"));
    }

    // Determine user role (in production, this would query a database)
    let role = determine_user_role(&req.wallet, &state).await;

    // Generate tokens
    let access_token = state.jwt.generate_access_token(&req.wallet, role)?;
    let refresh_token = state.jwt.generate_refresh_token(&req.wallet, role)?;

    log::info!("User logged in: {} with role {:?}", req.wallet, role);

    Ok(HttpResponse::Ok().json(TokenPair::new(access_token, refresh_token)))
}

/// POST /v1/auth/refresh
/// Refreshes an expired access token using a valid refresh token
pub async fn refresh_token(
    http_req: HttpRequest,
    state: web::Data<Arc<AppState>>,
    body: web::Json<RefreshRequest>,
) -> Result<HttpResponse, ApiError> {
    // Rate limit: 10 refresh attempts per minute per IP
    check_auth_rate_limit(&http_req, &state.redis).await?;

    let req = body.into_inner();

    // Validate the refresh token
    let claims = state.jwt.validate_token(&req.refresh_token)?;

    // Generate new access token with same role
    let access_token = state.jwt.generate_access_token(&claims.sub, claims.role)?;
    let refresh_token = state.jwt.generate_refresh_token(&claims.sub, claims.role)?;

    log::info!("Tokens refreshed for user: {}", claims.sub);

    Ok(HttpResponse::Ok().json(TokenPair::new(access_token, refresh_token)))
}

/// POST /v1/auth/logout
/// Logs out the current user by revoking their token
pub async fn logout(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
) -> Result<HttpResponse, ApiError> {
    // Extract the user from the access token
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok());

    if let Some(header) = auth_header {
        if header.starts_with("Bearer ") {
            let token = &header[7..];
            if let Ok(claims) = state.jwt.validate_token(token) {
                log::info!("User logged out: {}", claims.sub);

                // Add token to revocation list in Redis
                if let Err(e) = state.redis.revoke_token(&claims.jti, claims.exp).await {
                    log::warn!("Failed to add token to revocation list: {}", e);
                    // Don't fail the logout - client should still discard token
                }
            }
        }
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Logged out successfully"
    })))
}

/// POST /v1/auth/logout-all
/// Logs out the user from all devices by incrementing their token generation
pub async fn logout_all(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
) -> Result<HttpResponse, ApiError> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok());

    if let Some(header) = auth_header {
        if header.starts_with("Bearer ") {
            let token = &header[7..];
            if let Ok(claims) = state.jwt.validate_token(token) {
                log::info!("User {} logging out from all devices", claims.sub);

                // Increment user's token generation to invalidate all existing tokens
                if let Err(e) = state.redis.revoke_all_user_tokens(&claims.sub).await {
                    log::error!("Failed to revoke all tokens: {}", e);
                    return Err(ApiError::internal("Failed to logout from all devices"));
                }

                return Ok(HttpResponse::Ok().json(serde_json::json!({
                    "message": "Logged out from all devices successfully"
                })));
            }
        }
    }

    Err(ApiError::unauthorized("Invalid or missing token"))
}

/// Authenticated user with role from JWT token
#[derive(Debug, Clone)]
pub struct AuthenticatedUserWithRole {
    pub wallet_address: String,
    pub role: UserRole,
}

/// Extract authenticated user from JWT Bearer token
/// Used for endpoints that require JWT auth (login-based)
pub fn extract_jwt_user(
    req: &HttpRequest,
    state: &web::Data<Arc<AppState>>,
) -> Result<AuthenticatedUserWithRole, ApiError> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| ApiError::unauthorized("Missing Authorization header"))?;

    if !auth_header.starts_with("Bearer ") {
        return Err(ApiError::unauthorized("Invalid Authorization header format"));
    }

    let token = &auth_header[7..];
    let claims = state.jwt.validate_token(token)?;

    Ok(AuthenticatedUserWithRole {
        wallet_address: claims.sub,
        role: claims.role,
    })
}

/// Determine user role based on wallet address
/// In production, this would query a database or check against known addresses
async fn determine_user_role(wallet: &str, _state: &web::Data<Arc<AppState>>) -> UserRole {
    // Check if this is a known keeper address
    // In production, this would be stored in database or config
    let keeper_addresses: Vec<&str> = vec![
        // Add keeper wallet addresses here
    ];

    let admin_addresses: Vec<&str> = vec![
        // Add admin wallet addresses here
    ];

    if admin_addresses.contains(&wallet) {
        UserRole::Admin
    } else if keeper_addresses.contains(&wallet) {
        UserRole::Keeper
    } else {
        UserRole::User
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_message_parse() {
        let msg = "polyguard:4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU:1705680000:abc123";
        let parsed = AuthMessage::parse(msg).unwrap();

        assert_eq!(parsed.wallet, "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU");
        assert_eq!(parsed.timestamp, 1705680000);
        assert_eq!(parsed.nonce, "abc123");
    }

    #[test]
    fn test_auth_message_invalid_format() {
        let msg = "invalid:message";
        assert!(AuthMessage::parse(msg).is_err());
    }

    #[test]
    fn test_auth_message_invalid_prefix() {
        let msg = "wrong:wallet:1234567890:nonce";
        assert!(AuthMessage::parse(msg).is_err());
    }

    #[test]
    fn test_validate_solana_address_valid() {
        let valid = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";
        assert!(validate_solana_address(valid).is_ok());
    }

    #[test]
    fn test_validate_solana_address_too_short() {
        let short = "4zMMC9srt5Ri5X14GAgX";
        assert!(validate_solana_address(short).is_err());
    }

    #[test]
    fn test_validate_solana_address_invalid_chars() {
        // 'O' and 'l' are not valid base58 characters
        let invalid = "OzMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";
        assert!(validate_solana_address(invalid).is_err());
    }

    // Note: Nonce replay protection tests moved to integration tests
    // because they require Redis

    #[test]
    fn test_generate_nonce_uniqueness() {
        let nonce1 = generate_nonce();
        let nonce2 = generate_nonce();

        assert_ne!(nonce1, nonce2);
        assert!(nonce1.len() >= 32); // Should be sufficiently long
    }
}
