use crate::api::{
    check_auth_rate_limit,
    jwt::{TokenPair, UserRole},
    ApiError,
};
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use http::uri::Authority;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use siwe::VerificationOpts;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use time::OffsetDateTime;

const MESSAGE_EXPIRATION_SECS: u64 = 300;
const NONCE_CLEANUP_AGE_SECS: u64 = 600;
const EVM_ADDRESS_LEN: usize = 42;

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub wallet_address: String,
}

pub async fn extract_authenticated_user(
    req: &HttpRequest,
    state: &web::Data<Arc<AppState>>,
) -> Result<AuthenticatedUser, ApiError> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| ApiError::unauthorized("Missing Authorization header"))?;

    if !auth_header.starts_with("Bearer ") {
        return Err(ApiError::unauthorized(
            "Invalid Authorization header format",
        ));
    }

    let token = &auth_header[7..];
    let claims = state.jwt.validate_token(token)?;

    let revoked = state
        .redis
        .is_token_revoked(&claims.jti)
        .await
        .map_err(|e| {
            log::error!("Token revocation check failed: {}", e);
            ApiError::internal("Authentication validation failed")
        })?;

    if revoked {
        return Err(ApiError::unauthorized("Token revoked"));
    }

    Ok(AuthenticatedUser {
        wallet_address: claims.sub,
    })
}

async fn check_and_record_siwe_nonce_redis(
    redis: &crate::services::RedisService,
    nonce: &str,
) -> Result<(), ApiError> {
    let siwe_nonce = format!("siwe:{}", nonce);
    let was_new = redis
        .check_and_record_nonce(&siwe_nonce, NONCE_CLEANUP_AGE_SECS)
        .await
        .map_err(|e| {
            log::error!("Redis SIWE nonce check failed: {}", e);
            ApiError::internal("Nonce verification failed")
        })?;

    if !was_new {
        log::warn!("Replay attack detected: SIWE nonce {} already used", nonce);
        return Err(ApiError::unauthorized(
            "Nonce already used (possible replay attack)",
        ));
    }

    Ok(())
}

pub fn generate_nonce() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    let random_part: u64 = rand::random();

    format!("{:x}{:016x}", timestamp, random_part)
}

#[macro_export]
macro_rules! require_auth {
    ($req:expr, $state:expr) => {
        $crate::api::auth::extract_authenticated_user($req, $state).await?
    };
}

#[derive(Serialize)]
pub struct NonceResponse {
    pub nonce: String,
    pub expires_at: u64,
}

#[derive(Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Deserialize)]
pub struct SiweLoginRequest {
    pub wallet: String,
    pub message: String,
    pub signature: String,
}

pub async fn get_nonce() -> Result<HttpResponse, ApiError> {
    let nonce = generate_nonce();
    let expires_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ApiError::internal("System time error"))?
        .as_secs()
        + MESSAGE_EXPIRATION_SECS;

    Ok(HttpResponse::Ok().json(NonceResponse { nonce, expires_at }))
}

pub async fn get_siwe_nonce() -> Result<HttpResponse, ApiError> {
    get_nonce().await
}

pub async fn login(
    _http_req: HttpRequest,
    _state: web::Data<Arc<AppState>>,
    _body: web::Json<serde_json::Value>,
) -> Result<HttpResponse, ApiError> {
    Err(ApiError::bad_request(
        "LEGACY_AUTH_REMOVED",
        "Use /v1/auth/siwe/login",
    ))
}

pub async fn siwe_login(
    http_req: HttpRequest,
    state: web::Data<Arc<AppState>>,
    body: web::Json<SiweLoginRequest>,
) -> Result<HttpResponse, ApiError> {
    if !state.config.evm_enabled {
        return Err(ApiError::bad_request(
            "EVM_DISABLED",
            "EVM auth is disabled",
        ));
    }

    check_auth_rate_limit(&http_req, &state.redis).await?;

    let req = body.into_inner();
    validate_evm_address(&req.wallet)?;
    let wallet = normalize_evm_address(&req.wallet);

    let message: siwe::Message = req.message.parse().map_err(|e| {
        ApiError::bad_request(
            "INVALID_SIWE_MESSAGE",
            &format!("Invalid SIWE message: {}", e),
        )
    })?;

    let message_address = format!("0x{}", hex::encode(message.address));
    if wallet != message_address {
        return Err(ApiError::unauthorized(
            "Wallet address mismatch in SIWE message",
        ));
    }

    if message.chain_id != state.config.base_chain_id {
        return Err(ApiError::bad_request(
            "INVALID_CHAIN_ID",
            "SIWE message chain ID does not match configured Base chain",
        ));
    }

    let expected_domain: Authority = state
        .config
        .siwe_domain
        .parse()
        .map_err(|_| ApiError::internal("Invalid SIWE domain configuration"))?;
    if message.domain != expected_domain {
        return Err(ApiError::unauthorized("SIWE domain mismatch"));
    }

    check_and_record_siwe_nonce_redis(&state.redis, &message.nonce).await?;

    let signature = decode_hex_signature(&req.signature)?;

    let opts = VerificationOpts {
        domain: Some(expected_domain.clone()),
        nonce: Some(message.nonce.clone()),
        timestamp: Some(OffsetDateTime::now_utc()),
    };

    message
        .verify(&signature, &opts)
        .await
        .map_err(|e| ApiError::unauthorized(&format!("SIWE verification failed: {}", e)))?;

    let role = determine_user_role(&wallet, &state).await;
    let access_token = state.jwt.generate_access_token(&wallet, role)?;
    let refresh_token = state.jwt.generate_refresh_token(&wallet, role)?;

    log::info!("SIWE user logged in: {} with role {:?}", wallet, role);

    Ok(HttpResponse::Ok().json(TokenPair::new(access_token, refresh_token)))
}

pub async fn refresh_token(
    http_req: HttpRequest,
    state: web::Data<Arc<AppState>>,
    body: web::Json<RefreshRequest>,
) -> Result<HttpResponse, ApiError> {
    check_auth_rate_limit(&http_req, &state.redis).await?;

    let req = body.into_inner();
    let claims = state.jwt.validate_token(&req.refresh_token)?;

    let access_token = state.jwt.generate_access_token(&claims.sub, claims.role)?;
    let refresh_token = state.jwt.generate_refresh_token(&claims.sub, claims.role)?;

    log::info!("Tokens refreshed for user: {}", claims.sub);

    Ok(HttpResponse::Ok().json(TokenPair::new(access_token, refresh_token)))
}

pub async fn logout(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
) -> Result<HttpResponse, ApiError> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok());

    if let Some(header) = auth_header {
        if let Some(token) = header.strip_prefix("Bearer ") {
            if let Ok(claims) = state.jwt.validate_token(token) {
                log::info!("User logged out: {}", claims.sub);

                if let Err(e) = state.redis.revoke_token(&claims.jti, claims.exp).await {
                    log::warn!("Failed to add token to revocation list: {}", e);
                }
            }
        }
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Logged out successfully"
    })))
}

#[allow(dead_code)]
pub async fn logout_all(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
) -> Result<HttpResponse, ApiError> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok());

    if let Some(header) = auth_header {
        if let Some(token) = header.strip_prefix("Bearer ") {
            if let Ok(claims) = state.jwt.validate_token(token) {
                log::info!("User {} logging out from all devices", claims.sub);

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

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AuthenticatedUserWithRole {
    pub wallet_address: String,
    pub role: UserRole,
}

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
        return Err(ApiError::unauthorized(
            "Invalid Authorization header format",
        ));
    }

    let token = &auth_header[7..];
    let claims = state.jwt.validate_token(token)?;

    Ok(AuthenticatedUserWithRole {
        wallet_address: claims.sub,
        role: claims.role,
    })
}

async fn determine_user_role(wallet: &str, _state: &web::Data<Arc<AppState>>) -> UserRole {
    let keeper_addresses: Vec<&str> = vec![];
    let admin_addresses: Vec<&str> = vec![];

    if admin_addresses.contains(&wallet) {
        UserRole::Admin
    } else if keeper_addresses.contains(&wallet) {
        UserRole::Keeper
    } else {
        UserRole::User
    }
}

fn validate_evm_address(address: &str) -> Result<(), ApiError> {
    if address.len() != EVM_ADDRESS_LEN || !address.starts_with("0x") {
        return Err(ApiError::bad_request(
            "INVALID_WALLET",
            "Invalid EVM wallet address format",
        ));
    }

    if !address[2..].chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ApiError::bad_request(
            "INVALID_WALLET",
            "Invalid EVM wallet address format",
        ));
    }

    if !is_eip55_checksum(address) {
        return Err(ApiError::bad_request(
            "INVALID_WALLET",
            "EVM wallet address must use EIP-55 checksum casing",
        ));
    }

    Ok(())
}

fn normalize_evm_address(address: &str) -> String {
    address.to_ascii_lowercase()
}

fn is_eip55_checksum(address: &str) -> bool {
    if address.len() != EVM_ADDRESS_LEN || !address.starts_with("0x") {
        return false;
    }

    let hex_part = &address[2..];
    let lower = hex_part.to_ascii_lowercase();

    if hex_part == lower || hex_part == lower.to_ascii_uppercase() {
        return false;
    }

    let mut hasher = Keccak256::new();
    hasher.update(lower.as_bytes());
    let hash = hasher.finalize();

    for (idx, ch) in hex_part.chars().enumerate() {
        if ch.is_ascii_digit() {
            continue;
        }

        let hash_byte = hash[idx / 2];
        let nibble = if idx % 2 == 0 {
            hash_byte >> 4
        } else {
            hash_byte & 0x0f
        };

        if nibble >= 8 && !ch.is_ascii_uppercase() {
            return false;
        }

        if nibble < 8 && !ch.is_ascii_lowercase() {
            return false;
        }
    }

    true
}

fn decode_hex_signature(signature: &str) -> Result<Vec<u8>, ApiError> {
    let sig = signature.strip_prefix("0x").unwrap_or(signature);

    hex::decode(sig)
        .map_err(|_| ApiError::bad_request("INVALID_SIGNATURE", "Signature must be valid hex"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_evm_address_valid() {
        let valid = "0x71C7656EC7ab88b098defB751B7401B5f6d8976F";
        assert!(validate_evm_address(valid).is_ok());
    }

    #[test]
    fn test_validate_evm_address_invalid() {
        assert!(validate_evm_address("0x123").is_err());
        assert!(validate_evm_address("71C7656EC7ab88b098defB751B7401B5f6d8976F").is_err());
        assert!(validate_evm_address("0xZZC7656EC7ab88b098defB751B7401B5f6d8976F").is_err());
        assert!(validate_evm_address("0x71c7656ec7ab88b098defb751b7401b5f6d8976f").is_err());
    }

    #[test]
    fn test_decode_hex_signature() {
        let sig = format!("0x{}", "11".repeat(65));
        let decoded = decode_hex_signature(&sig).unwrap();
        assert_eq!(decoded.len(), 65);
    }

    #[test]
    fn test_generate_nonce_uniqueness() {
        let nonce1 = generate_nonce();
        let nonce2 = generate_nonce();

        assert_ne!(nonce1, nonce2);
        assert!(nonce1.len() >= 32);
    }

    #[test]
    fn test_generate_nonce_format() {
        let nonce = generate_nonce();
        assert!(nonce.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_authenticated_user_struct() {
        let user = AuthenticatedUser {
            wallet_address: "0xabc".to_string(),
        };
        assert_eq!(user.wallet_address, "0xabc");
    }

    #[test]
    fn test_message_expiration_constant() {
        assert_eq!(MESSAGE_EXPIRATION_SECS, 300);
    }

    #[test]
    fn test_nonce_cleanup_age_constant() {
        assert_eq!(NONCE_CLEANUP_AGE_SECS, 600);
    }

    #[test]
    fn test_generate_multiple_nonces_all_unique() {
        use std::collections::HashSet;
        let nonces: HashSet<String> = (0..100).map(|_| generate_nonce()).collect();
        assert_eq!(nonces.len(), 100, "All generated nonces should be unique");
    }
}
