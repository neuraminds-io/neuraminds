use crate::api::ApiError;
use crate::AppState;
use actix_web::HttpRequest;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use uuid::Uuid;

const ERC20_TRANSFER_SELECTOR: &str = "a9059cbb";

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum X402Resource {
    OrderBook,
    Trades,
    McpToolCall,
}

impl X402Resource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OrderBook => "orderbook",
            Self::Trades => "trades",
            Self::McpToolCall => "mcp_tool_call",
        }
    }
}

impl fmt::Display for X402Resource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct X402Quote {
    pub scheme: &'static str,
    pub version: u8,
    pub resource: String,
    pub currency: &'static str,
    pub amount_microusdc: u64,
    pub receiver: String,
    pub nonce: String,
    pub expires_at: u64,
    pub challenge: String,
    pub header_template: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct X402PaymentProof {
    pub resource: String,
    pub amount_microusdc: u64,
    pub nonce: String,
    pub expires_at: u64,
    pub tx_hash: String,
    pub signature: String,
}

impl X402PaymentProof {
    pub fn from_header_value(value: &str) -> Result<Self, ApiError> {
        let mut resource = None;
        let mut amount = None;
        let mut nonce = None;
        let mut expires_at = None;
        let mut tx_hash = None;
        let mut signature = None;

        for part in value.split(';') {
            let segment = part.trim();
            if segment.is_empty() {
                continue;
            }
            let mut pieces = segment.splitn(2, '=');
            let key = pieces.next().unwrap_or("").trim();
            let raw = pieces.next().unwrap_or("").trim().trim_matches('"');

            match key {
                "resource" => resource = Some(raw.to_string()),
                "amount_microusdc" => amount = raw.parse::<u64>().ok(),
                "nonce" => nonce = Some(raw.to_string()),
                "expires_at" => expires_at = raw.parse::<u64>().ok(),
                "tx_hash" => tx_hash = Some(raw.to_string()),
                "signature" => signature = Some(raw.to_string()),
                _ => {}
            }
        }

        let payload = Self {
            resource: resource.ok_or_else(|| {
                ApiError::bad_request("INVALID_X402_HEADER", "x-payment is missing resource")
            })?,
            amount_microusdc: amount.ok_or_else(|| {
                ApiError::bad_request(
                    "INVALID_X402_HEADER",
                    "x-payment is missing amount_microusdc",
                )
            })?,
            nonce: nonce.ok_or_else(|| {
                ApiError::bad_request("INVALID_X402_HEADER", "x-payment is missing nonce")
            })?,
            expires_at: expires_at.ok_or_else(|| {
                ApiError::bad_request("INVALID_X402_HEADER", "x-payment is missing expires_at")
            })?,
            tx_hash: tx_hash.ok_or_else(|| {
                ApiError::bad_request("INVALID_X402_HEADER", "x-payment is missing tx_hash")
            })?,
            signature: signature.ok_or_else(|| {
                ApiError::bad_request("INVALID_X402_HEADER", "x-payment is missing signature")
            })?,
        };

        Ok(payload)
    }

    pub fn to_header_value(&self) -> String {
        format!(
            "resource={};amount_microusdc={};nonce={};expires_at={};tx_hash={};signature={}",
            self.resource,
            self.amount_microusdc,
            self.nonce,
            self.expires_at,
            self.tx_hash,
            self.signature
        )
    }
}

fn required_amount(state: &AppState, resource: X402Resource) -> u64 {
    match resource {
        X402Resource::OrderBook => state.config.x402_orderbook_price_microusdc,
        X402Resource::Trades => state.config.x402_trades_price_microusdc,
        X402Resource::McpToolCall => state.config.x402_mcp_price_microusdc,
    }
}

fn build_quote_payload(
    resource: &str,
    amount_microusdc: u64,
    nonce: &str,
    expires_at: u64,
) -> String {
    format!(
        "resource={resource};amount_microusdc={amount_microusdc};nonce={nonce};expires_at={expires_at}"
    )
}

fn sign_payload(signing_key: &str, payload: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(signing_key.as_bytes());
    hasher.update(b":");
    hasher.update(payload.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn build_quote(state: &AppState, resource: X402Resource) -> X402Quote {
    let now = Utc::now().timestamp().max(0) as u64;
    let expires_at = now.saturating_add(state.config.x402_quote_ttl_seconds.max(30));
    let nonce = Uuid::new_v4().to_string();
    let amount = required_amount(state, resource);
    let payload = build_quote_payload(resource.as_str(), amount, &nonce, expires_at);
    let challenge = sign_payload(state.config.x402_signing_key.as_str(), &payload);

    X402Quote {
        scheme: "x402",
        version: 1,
        resource: resource.as_str().to_string(),
        currency: "USDC",
        amount_microusdc: amount,
        receiver: state.config.x402_receiver_address.clone(),
        nonce: nonce.clone(),
        expires_at,
        challenge,
        header_template: format!(
            "resource={};amount_microusdc={};nonce={};expires_at={};tx_hash=<tx>;signature=<quote.challenge>",
            resource.as_str(),
            amount,
            nonce,
            expires_at
        ),
    }
}

pub async fn ensure_payment_for_request(
    state: &AppState,
    req: &HttpRequest,
    resource: X402Resource,
) -> Result<(), ApiError> {
    if !state.config.x402_enabled {
        return Ok(());
    }

    let header = req
        .headers()
        .get("x-payment")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| {
            let quote = build_quote(state, resource);
            ApiError::payment_required("x402 payment required", Some(quote))
        })?;

    let proof = X402PaymentProof::from_header_value(header)?;
    ensure_payment_from_proof(state, &proof, resource).await
}

pub async fn ensure_payment_from_proof(
    state: &AppState,
    proof: &X402PaymentProof,
    resource: X402Resource,
) -> Result<(), ApiError> {
    if !state.config.x402_enabled {
        return Ok(());
    }

    if proof.resource != resource.as_str() {
        let quote = build_quote(state, resource);
        return Err(ApiError::payment_required(
            "x402 resource mismatch",
            Some(quote),
        ));
    }
    if proof.nonce.len() < 8 || proof.nonce.len() > 128 {
        return Err(ApiError::bad_request(
            "INVALID_X402_NONCE",
            "x402 nonce must be between 8 and 128 characters",
        ));
    }
    if !proof
        .nonce
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':')
    {
        return Err(ApiError::bad_request(
            "INVALID_X402_NONCE",
            "x402 nonce contains invalid characters",
        ));
    }
    if !proof.tx_hash.starts_with("0x")
        || proof.tx_hash.len() != 66
        || !proof.tx_hash[2..].chars().all(|c| c.is_ascii_hexdigit())
    {
        return Err(ApiError::bad_request(
            "INVALID_X402_TX_HASH",
            "x402 tx_hash must be a 0x-prefixed 32-byte hash",
        ));
    }

    let now = Utc::now().timestamp().max(0) as u64;
    if now > proof.expires_at {
        let quote = build_quote(state, resource);
        return Err(ApiError::payment_required(
            "x402 payment expired",
            Some(quote),
        ));
    }

    let expected_amount = required_amount(state, resource);
    if proof.amount_microusdc < expected_amount {
        let quote = build_quote(state, resource);
        return Err(ApiError::payment_required(
            "x402 amount is below required price",
            Some(quote),
        ));
    }

    let payload = build_quote_payload(
        proof.resource.as_str(),
        proof.amount_microusdc,
        proof.nonce.as_str(),
        proof.expires_at,
    );
    let expected_signature = sign_payload(state.config.x402_signing_key.as_str(), payload.as_str());
    if !expected_signature.eq_ignore_ascii_case(proof.signature.as_str()) {
        return Err(ApiError::unauthorized("Invalid x402 payment signature"));
    }

    verify_settlement_transaction(state, proof, expected_amount).await?;

    let ttl = proof.expires_at.saturating_sub(now).max(1);
    let nonce_key = format!("x402:nonce:{}", proof.nonce);
    let newly_recorded = state
        .redis
        .check_and_record_nonce(nonce_key.as_str(), ttl)
        .await
        .map_err(|_| ApiError::internal("Failed to validate x402 nonce"))?;
    if !newly_recorded {
        return Err(ApiError::conflict(
            "X402_NONCE_REPLAYED",
            "x402 nonce has already been used",
        ));
    }

    let tx_key = format!("x402:tx:{}", proof.tx_hash.to_ascii_lowercase());
    let tx_recorded = state
        .redis
        .check_and_record_nonce(tx_key.as_str(), ttl)
        .await
        .map_err(|_| ApiError::internal("Failed to validate x402 tx hash"))?;
    if !tx_recorded {
        return Err(ApiError::conflict(
            "X402_TX_REPLAYED",
            "x402 tx_hash has already been used",
        ));
    }

    Ok(())
}

async fn verify_settlement_transaction(
    state: &AppState,
    proof: &X402PaymentProof,
    required_amount: u64,
) -> Result<(), ApiError> {
    let receipt = state
        .evm_rpc
        .eth_get_transaction_receipt(proof.tx_hash.as_str())
        .await
        .map_err(|_| {
            ApiError::payment_required("x402 tx receipt lookup failed", None::<X402Quote>)
        })?
        .ok_or_else(|| {
            ApiError::payment_required("x402 tx receipt not found", None::<X402Quote>)
        })?;

    let status = receipt
        .status
        .as_deref()
        .and_then(parse_u64_hex)
        .ok_or_else(|| {
            ApiError::payment_required("x402 tx status unavailable", None::<X402Quote>)
        })?;
    if status != 1 {
        return Err(ApiError::payment_required(
            "x402 tx reverted onchain",
            None::<X402Quote>,
        ));
    }

    let tx = state
        .evm_rpc
        .eth_get_transaction_by_hash(proof.tx_hash.as_str())
        .await
        .map_err(|_| ApiError::payment_required("x402 tx lookup failed", None::<X402Quote>))?
        .ok_or_else(|| ApiError::payment_required("x402 tx not found", None::<X402Quote>))?;

    let token_address = normalize_hex_address(state.config.usdc_mint.as_str())
        .ok_or_else(|| ApiError::internal("USDC_MINT is not a valid address"))?;
    let tx_to = tx
        .to
        .as_deref()
        .and_then(normalize_hex_address)
        .ok_or_else(|| ApiError::payment_required("x402 tx target missing", None::<X402Quote>))?;
    if tx_to != token_address {
        return Err(ApiError::payment_required(
            "x402 tx must target configured USDC token",
            None::<X402Quote>,
        ));
    }

    let (receiver, amount) = decode_erc20_transfer(tx.input.as_str()).ok_or_else(|| {
        ApiError::payment_required("x402 tx is not a valid ERC20 transfer", None::<X402Quote>)
    })?;
    let expected_receiver = normalize_hex_address(state.config.x402_receiver_address.as_str())
        .ok_or_else(|| ApiError::internal("X402_RECEIVER_ADDRESS is not a valid address"))?;
    if receiver != expected_receiver {
        return Err(ApiError::payment_required(
            "x402 tx receiver mismatch",
            None::<X402Quote>,
        ));
    }

    let proof_amount = proof.amount_microusdc as u128;
    let minimum = (required_amount as u128).max(proof_amount);
    if amount < minimum {
        return Err(ApiError::payment_required(
            "x402 tx amount below required minimum",
            None::<X402Quote>,
        ));
    }

    Ok(())
}

fn parse_u64_hex(value: &str) -> Option<u64> {
    let trimmed = value.trim_start_matches("0x");
    if trimmed.is_empty() {
        return None;
    }

    let normalized = trimmed.trim_start_matches('0');
    if normalized.is_empty() {
        return Some(0);
    }
    if normalized.len() > 16 {
        return None;
    }

    u64::from_str_radix(normalized, 16).ok()
}

fn parse_u128_hex_word(word: &str) -> Option<u128> {
    let trimmed = word.trim_start_matches('0');
    if trimmed.is_empty() {
        return Some(0);
    }
    if trimmed.len() > 32 {
        return None;
    }
    u128::from_str_radix(trimmed, 16).ok()
}

fn normalize_hex_address(value: &str) -> Option<String> {
    let trimmed = value.trim().to_ascii_lowercase();
    if trimmed.len() != 42 || !trimmed.starts_with("0x") {
        return None;
    }
    if !trimmed[2..].chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(trimmed)
}

fn decode_erc20_transfer(input: &str) -> Option<(String, u128)> {
    let payload = input.trim().trim_start_matches("0x").to_ascii_lowercase();
    if payload.len() < 136 || &payload[0..8] != ERC20_TRANSFER_SELECTOR {
        return None;
    }
    if !payload.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }

    let receiver_word = &payload[8..72];
    let amount_word = &payload[72..136];

    let receiver = format!("0x{}", &receiver_word[24..64]);
    let amount = parse_u128_hex_word(amount_word)?;
    Some((receiver, amount))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_payment_header() {
        let header = "resource=orderbook;amount_microusdc=2500;nonce=test-nonce-1;expires_at=1700000000;tx_hash=0x1111111111111111111111111111111111111111111111111111111111111111;signature=sig";
        let parsed = X402PaymentProof::from_header_value(header).expect("valid header");
        assert_eq!(parsed.resource, "orderbook");
        assert_eq!(parsed.amount_microusdc, 2500);
        assert_eq!(parsed.nonce, "test-nonce-1");
    }

    #[test]
    fn test_decode_erc20_transfer_input() {
        let receiver = "0000000000000000000000001111111111111111111111111111111111111111";
        let amount = format!("{:064x}", 25_000u128);
        let input = format!("0x{}{}{}", ERC20_TRANSFER_SELECTOR, receiver, amount);
        let decoded = decode_erc20_transfer(input.as_str()).expect("transfer input should decode");
        assert_eq!(decoded.0, "0x1111111111111111111111111111111111111111");
        assert_eq!(decoded.1, 25_000);
    }
}
