//! Per-endpoint rate limiting
//!
//! Provides configurable rate limits per endpoint type:
//! - Auth endpoints: 10/min (prevent brute force)
//! - Write endpoints: 30/min (prevent spam)
//! - Read endpoints: 120/min (reasonable API usage)

#![allow(dead_code)]

use actix_web::HttpRequest;

use crate::services::RedisService;
use super::ApiError;

/// Rate limit tiers for different endpoint types
#[derive(Clone, Copy)]
pub enum RateLimitTier {
    /// Auth endpoints: 10 requests per minute
    Auth,
    /// Write operations: 30 requests per minute
    Write,
    /// Read operations: 120 requests per minute
    Read,
}

impl RateLimitTier {
    pub fn limit(&self) -> i64 {
        match self {
            RateLimitTier::Auth => 10,
            RateLimitTier::Write => 30,
            RateLimitTier::Read => 120,
        }
    }

    pub fn window_secs(&self) -> u64 {
        60 // 1 minute window for all tiers
    }
}

/// Check rate limit for a request. Call at the start of rate-limited handlers.
pub async fn check_rate_limit(
    req: &HttpRequest,
    redis: &RedisService,
    tier: RateLimitTier,
) -> Result<(), ApiError> {
    let client_ip = req
        .connection_info()
        .realip_remote_addr()
        .unwrap_or("unknown")
        .to_string();

    let path = req.path();
    let method = req.method().as_str();

    // Build rate limit key: tier:ip:path_prefix
    let path_prefix = path.split('/').take(3).collect::<Vec<_>>().join("/");
    let key = format!("{}:{}:{}", method, client_ip, path_prefix);

    let limit = tier.limit();
    let window = tier.window_secs();

    match redis.increment_rate_limit(&key, window).await {
        Ok((count, _ttl)) => {
            if count > limit {
                log::warn!(
                    "Rate limit exceeded for {} on {} (count: {}, limit: {})",
                    client_ip, path, count, limit
                );
                return Err(ApiError::rate_limited(window));
            }
            Ok(())
        }
        Err(e) => {
            // Fail open to avoid blocking legitimate requests on Redis errors
            log::error!("Rate limit check failed (allowing request): {}", e);
            Ok(())
        }
    }
}

/// Helper to check auth-tier rate limit
pub async fn check_auth_rate_limit(req: &HttpRequest, redis: &RedisService) -> Result<(), ApiError> {
    check_rate_limit(req, redis, RateLimitTier::Auth).await
}

/// Helper to check write-tier rate limit
pub async fn check_write_rate_limit(req: &HttpRequest, redis: &RedisService) -> Result<(), ApiError> {
    check_rate_limit(req, redis, RateLimitTier::Write).await
}
