//! Input validation module for the Polyguard API
//!
//! Centralizes all validation logic to ensure consistent security across endpoints.

use regex::Regex;
use lazy_static::lazy_static;
use super::ApiError;

lazy_static! {
    // Solana base58 address pattern
    static ref SOLANA_ADDRESS_REGEX: Regex = Regex::new(
        r"^[1-9A-HJ-NP-Za-km-z]{32,44}$"
    ).unwrap();

    // Market ID pattern (alphanumeric with hyphens, max 64 chars)
    static ref MARKET_ID_REGEX: Regex = Regex::new(
        r"^[a-zA-Z0-9\-_]{1,64}$"
    ).unwrap();

    // UUID pattern
    static ref UUID_REGEX: Regex = Regex::new(
        r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$"
    ).unwrap();

    // Transaction signature pattern (base58, 87-88 chars)
    static ref TX_SIGNATURE_REGEX: Regex = Regex::new(
        r"^[1-9A-HJ-NP-Za-km-z]{87,88}$"
    ).unwrap();
}

/// Validation limits
pub mod limits {
    /// Maximum quantity per order (1 billion tokens)
    pub const MAX_ORDER_QUANTITY: u64 = 1_000_000_000;

    /// Minimum quantity per order
    pub const MIN_ORDER_QUANTITY: u64 = 1;

    /// Maximum question length for markets
    pub const MAX_QUESTION_LENGTH: usize = 500;

    /// Maximum market description length
    pub const MAX_DESCRIPTION_LENGTH: usize = 2000;

    /// Maximum resolution details length
    pub const MAX_RESOLUTION_DETAILS_LENGTH: usize = 1000;

    /// Maximum pagination limit
    pub const MAX_PAGE_LIMIT: i64 = 100;

    /// Default pagination limit
    pub const DEFAULT_PAGE_LIMIT: i64 = 50;

    /// Maximum fee in basis points (50%)
    pub const MAX_FEE_BPS: u16 = 5000;

    /// Minimum trading window (1 hour in seconds)
    pub const MIN_TRADING_WINDOW_SECS: i64 = 3600;

    /// Maximum trading window (1 year in seconds)
    pub const MAX_TRADING_WINDOW_SECS: i64 = 365 * 24 * 3600;
}

/// Validate a Solana wallet address
pub fn validate_wallet_address(address: &str) -> Result<(), ApiError> {
    if address.is_empty() {
        return Err(ApiError::bad_request(
            "INVALID_WALLET",
            "Wallet address cannot be empty",
        ));
    }

    if !SOLANA_ADDRESS_REGEX.is_match(address) {
        return Err(ApiError::bad_request(
            "INVALID_WALLET",
            "Invalid Solana wallet address format",
        ));
    }

    Ok(())
}

/// Validate a market ID
pub fn validate_market_id(market_id: &str) -> Result<(), ApiError> {
    if market_id.is_empty() {
        return Err(ApiError::bad_request(
            "INVALID_MARKET_ID",
            "Market ID cannot be empty",
        ));
    }

    if !MARKET_ID_REGEX.is_match(market_id) {
        return Err(ApiError::bad_request(
            "INVALID_MARKET_ID",
            "Market ID must be alphanumeric with optional hyphens/underscores, max 64 chars",
        ));
    }

    Ok(())
}

/// Validate a UUID
pub fn validate_uuid(id: &str, field_name: &str) -> Result<(), ApiError> {
    if !UUID_REGEX.is_match(id) {
        return Err(ApiError::bad_request(
            "INVALID_ID",
            &format!("Invalid {} format - must be a valid UUID", field_name),
        ));
    }

    Ok(())
}

/// Validate order price (must be between 0 and 1 exclusive)
pub fn validate_order_price(price: f64) -> Result<(), ApiError> {
    if price.is_nan() || price.is_infinite() {
        return Err(ApiError::bad_request(
            "INVALID_PRICE",
            "Price must be a valid number",
        ));
    }

    if price <= 0.0 || price >= 1.0 {
        return Err(ApiError::bad_request(
            "INVALID_PRICE",
            "Price must be between 0 and 1 (exclusive)",
        ));
    }

    // Check precision (max 4 decimal places / 1 basis point)
    let bps = (price * 10000.0).round() as u16;
    if (price - (bps as f64 / 10000.0)).abs() > 0.00001 {
        return Err(ApiError::bad_request(
            "INVALID_PRICE",
            "Price can have at most 4 decimal places (1 basis point precision)",
        ));
    }

    Ok(())
}

/// Validate order quantity
pub fn validate_order_quantity(quantity: u64) -> Result<(), ApiError> {
    if quantity < limits::MIN_ORDER_QUANTITY {
        return Err(ApiError::bad_request(
            "INVALID_QUANTITY",
            "Quantity must be at least 1",
        ));
    }

    if quantity > limits::MAX_ORDER_QUANTITY {
        return Err(ApiError::bad_request(
            "INVALID_QUANTITY",
            &format!("Quantity cannot exceed {}", limits::MAX_ORDER_QUANTITY),
        ));
    }

    Ok(())
}

/// Validate market question
pub fn validate_market_question(question: &str) -> Result<(), ApiError> {
    let trimmed = question.trim();

    if trimmed.is_empty() {
        return Err(ApiError::bad_request(
            "INVALID_QUESTION",
            "Market question cannot be empty",
        ));
    }

    if trimmed.len() > limits::MAX_QUESTION_LENGTH {
        return Err(ApiError::bad_request(
            "INVALID_QUESTION",
            &format!("Question cannot exceed {} characters", limits::MAX_QUESTION_LENGTH),
        ));
    }

    // Check for potential XSS/injection (basic check)
    if contains_dangerous_chars(trimmed) {
        return Err(ApiError::bad_request(
            "INVALID_QUESTION",
            "Question contains invalid characters",
        ));
    }

    Ok(())
}

/// Validate optional description
pub fn validate_description(description: Option<&str>) -> Result<(), ApiError> {
    if let Some(desc) = description {
        if desc.len() > limits::MAX_DESCRIPTION_LENGTH {
            return Err(ApiError::bad_request(
                "INVALID_DESCRIPTION",
                &format!("Description cannot exceed {} characters", limits::MAX_DESCRIPTION_LENGTH),
            ));
        }

        if contains_dangerous_chars(desc) {
            return Err(ApiError::bad_request(
                "INVALID_DESCRIPTION",
                "Description contains invalid characters",
            ));
        }
    }

    Ok(())
}

/// Validate fee in basis points
pub fn validate_fee_bps(fee_bps: u16) -> Result<(), ApiError> {
    if fee_bps > limits::MAX_FEE_BPS {
        return Err(ApiError::bad_request(
            "INVALID_FEE",
            &format!("Fee cannot exceed {} basis points ({}%)", limits::MAX_FEE_BPS, limits::MAX_FEE_BPS / 100),
        ));
    }

    Ok(())
}

/// Validate trading end timestamp
pub fn validate_trading_end(trading_end: i64) -> Result<(), ApiError> {
    let now = chrono::Utc::now().timestamp();

    if trading_end <= now {
        return Err(ApiError::bad_request(
            "INVALID_TRADING_END",
            "Trading end time must be in the future",
        ));
    }

    let duration = trading_end - now;

    if duration < limits::MIN_TRADING_WINDOW_SECS {
        return Err(ApiError::bad_request(
            "INVALID_TRADING_END",
            "Trading window must be at least 1 hour",
        ));
    }

    if duration > limits::MAX_TRADING_WINDOW_SECS {
        return Err(ApiError::bad_request(
            "INVALID_TRADING_END",
            "Trading window cannot exceed 1 year",
        ));
    }

    Ok(())
}

/// Validate pagination parameters
pub fn validate_pagination(limit: Option<i64>, offset: Option<i64>) -> Result<(i64, i64), ApiError> {
    let limit = limit.unwrap_or(limits::DEFAULT_PAGE_LIMIT);
    let offset = offset.unwrap_or(0);

    if limit <= 0 {
        return Err(ApiError::bad_request(
            "INVALID_LIMIT",
            "Limit must be a positive number",
        ));
    }

    if limit > limits::MAX_PAGE_LIMIT {
        return Err(ApiError::bad_request(
            "INVALID_LIMIT",
            &format!("Limit cannot exceed {}", limits::MAX_PAGE_LIMIT),
        ));
    }

    if offset < 0 {
        return Err(ApiError::bad_request(
            "INVALID_OFFSET",
            "Offset cannot be negative",
        ));
    }

    Ok((limit, offset))
}

/// Validate transaction signature
pub fn validate_tx_signature(signature: &str) -> Result<(), ApiError> {
    if !TX_SIGNATURE_REGEX.is_match(signature) {
        return Err(ApiError::bad_request(
            "INVALID_SIGNATURE",
            "Invalid transaction signature format",
        ));
    }

    Ok(())
}

/// Check for potentially dangerous characters (basic XSS/injection prevention)
fn contains_dangerous_chars(input: &str) -> bool {
    // Check for script tags, SQL injection patterns, etc.
    let dangerous_patterns = [
        "<script", "</script", "javascript:", "onerror=", "onload=",
        "onclick=", "DROP TABLE", "DELETE FROM", "INSERT INTO",
        "UPDATE ", "UNION SELECT", "--", "/*", "*/",
    ];

    let lower = input.to_lowercase();
    dangerous_patterns.iter().any(|pattern| lower.contains(&pattern.to_lowercase()))
}

/// Sanitize string input (trim and limit length)
pub fn sanitize_string(input: &str, max_length: usize) -> String {
    input.trim().chars().take(max_length).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_wallet_address() {
        // Valid addresses
        assert!(validate_wallet_address("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU").is_ok());
        assert!(validate_wallet_address("11111111111111111111111111111111").is_ok());

        // Invalid addresses
        assert!(validate_wallet_address("").is_err());
        assert!(validate_wallet_address("short").is_err());
        assert!(validate_wallet_address("0xInvalidEthAddress").is_err());
        assert!(validate_wallet_address("contains spaces here").is_err());
    }

    #[test]
    fn test_validate_order_price() {
        // Valid prices
        assert!(validate_order_price(0.5).is_ok());
        assert!(validate_order_price(0.0001).is_ok());
        assert!(validate_order_price(0.9999).is_ok());

        // Invalid prices
        assert!(validate_order_price(0.0).is_err());
        assert!(validate_order_price(1.0).is_err());
        assert!(validate_order_price(-0.5).is_err());
        assert!(validate_order_price(1.5).is_err());
        assert!(validate_order_price(f64::NAN).is_err());
        assert!(validate_order_price(f64::INFINITY).is_err());
    }

    #[test]
    fn test_validate_order_quantity() {
        // Valid quantities
        assert!(validate_order_quantity(1).is_ok());
        assert!(validate_order_quantity(1000).is_ok());
        assert!(validate_order_quantity(1_000_000_000).is_ok());

        // Invalid quantities
        assert!(validate_order_quantity(0).is_err());
        assert!(validate_order_quantity(1_000_000_001).is_err());
    }

    #[test]
    fn test_validate_market_question() {
        // Valid questions
        assert!(validate_market_question("Will BTC reach $100k by 2025?").is_ok());
        assert!(validate_market_question("Simple question").is_ok());

        // Invalid questions
        assert!(validate_market_question("").is_err());
        assert!(validate_market_question("   ").is_err());
        assert!(validate_market_question("<script>alert('xss')</script>").is_err());
        assert!(validate_market_question("DROP TABLE markets;").is_err());
    }

    #[test]
    fn test_validate_pagination() {
        // Valid pagination
        assert!(validate_pagination(Some(50), Some(0)).is_ok());
        assert!(validate_pagination(None, None).is_ok());
        assert!(validate_pagination(Some(100), Some(100)).is_ok());

        // Invalid pagination
        assert!(validate_pagination(Some(0), None).is_err());
        assert!(validate_pagination(Some(101), None).is_err());
        assert!(validate_pagination(None, Some(-1)).is_err());
    }

    #[test]
    fn test_contains_dangerous_chars() {
        assert!(contains_dangerous_chars("<script>"));
        assert!(contains_dangerous_chars("DROP TABLE users"));
        assert!(contains_dangerous_chars("SELECT * FROM users--"));

        assert!(!contains_dangerous_chars("Normal question about markets"));
        assert!(!contains_dangerous_chars("Will ETH > $5000?"));
    }
}
