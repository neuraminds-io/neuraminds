use crate::api::ApiError;
use crate::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct SwarmSendRequest {
    pub swarm_id: String,
    pub sender: String,
    pub message: String,
    pub signature: String,
    pub metadata: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct SwarmListQuery {
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmMessage {
    pub id: String,
    pub swarm_id: String,
    pub topic: String,
    pub sender: String,
    pub message: String,
    pub signature: String,
    pub metadata: Option<Value>,
    pub created_at: String,
    pub unix_ms: i64,
}

#[derive(Debug, Serialize)]
pub struct SwarmMessagesResponse {
    pub data: Vec<SwarmMessage>,
    pub total_returned: usize,
    pub limit: u64,
    pub offset: u64,
    pub topic: String,
}

fn sign_payload(signing_key: &str, payload: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(signing_key.as_bytes());
    hasher.update(b":");
    hasher.update(payload.as_bytes());
    hex::encode(hasher.finalize())
}

fn build_payload(request: &SwarmSendRequest) -> String {
    format!(
        "swarm_id={};sender={};message={}",
        request.swarm_id.trim(),
        request.sender.trim().to_ascii_lowercase(),
        request.message
    )
}

fn validate_swarm_id(value: &str) -> Result<String, ApiError> {
    let trimmed = value.trim();
    if trimmed.len() < 3 || trimmed.len() > 128 {
        return Err(ApiError::bad_request(
            "INVALID_SWARM_ID",
            "swarm_id length must be between 3 and 128 characters",
        ));
    }
    if !trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err(ApiError::bad_request(
            "INVALID_SWARM_ID",
            "swarm_id contains invalid characters",
        ));
    }
    Ok(trimmed.to_string())
}

fn validate_sender(value: &str) -> Result<String, ApiError> {
    let trimmed = value.trim().to_ascii_lowercase();
    let is_valid_hex = trimmed.starts_with("0x")
        && trimmed.len() == 42
        && trimmed[2..].chars().all(|c| c.is_ascii_hexdigit());
    if !is_valid_hex {
        return Err(ApiError::bad_request(
            "INVALID_SENDER",
            "sender must be a valid 0x EVM wallet address",
        ));
    }
    Ok(trimmed)
}

fn message_key(swarm_id: &str) -> String {
    format!("xmtp:swarm:{swarm_id}:messages")
}

fn topic(state: &AppState, swarm_id: &str) -> String {
    format!(
        "{}/{}",
        state.config.xmtp_swarm_topic_prefix.trim_end_matches('/'),
        swarm_id
    )
}

pub async fn send_message(
    state: &AppState,
    request: SwarmSendRequest,
) -> Result<SwarmMessage, ApiError> {
    if !state.config.xmtp_swarm_enabled {
        return Err(ApiError::bad_request(
            "XMTP_SWARM_DISABLED",
            "XMTP swarm messaging is disabled",
        ));
    }

    let swarm_id = validate_swarm_id(request.swarm_id.as_str())?;
    let sender = validate_sender(request.sender.as_str())?;
    let message = request.message.trim().to_string();
    if message.is_empty() {
        return Err(ApiError::bad_request(
            "INVALID_MESSAGE",
            "message must not be empty",
        ));
    }
    if message.as_bytes().len() > state.config.xmtp_swarm_max_message_bytes as usize {
        return Err(ApiError::bad_request(
            "MESSAGE_TOO_LARGE",
            "message exceeds XMTP swarm max payload size",
        ));
    }

    let canonical = build_payload(&SwarmSendRequest {
        swarm_id: swarm_id.clone(),
        sender: sender.clone(),
        message: message.clone(),
        signature: request.signature.clone(),
        metadata: request.metadata.clone(),
    });
    let expected_signature = sign_payload(
        state.config.xmtp_swarm_signing_key.as_str(),
        canonical.as_str(),
    );
    if !expected_signature.eq_ignore_ascii_case(request.signature.trim()) {
        return Err(ApiError::unauthorized("Invalid XMTP swarm signature"));
    }

    let unix_ms = Utc::now().timestamp_millis();
    let created_at = Utc::now().to_rfc3339();
    let envelope = SwarmMessage {
        id: format!("xmtp_{}", Uuid::new_v4()),
        swarm_id: swarm_id.clone(),
        topic: topic(state, swarm_id.as_str()),
        sender,
        message,
        signature: request.signature,
        metadata: request.metadata,
        created_at,
        unix_ms,
    };

    let raw = serde_json::to_string(&envelope)
        .map_err(|_| ApiError::internal("Failed to serialize XMTP envelope"))?;
    state
        .redis
        .list_push_with_trim(
            message_key(swarm_id.as_str()).as_str(),
            raw.as_str(),
            state.config.xmtp_swarm_max_messages.max(10),
        )
        .await
        .map_err(|_| ApiError::internal("Failed to persist XMTP swarm message"))?;
    state
        .redis
        .publish(envelope.topic.as_str(), raw.as_str())
        .await
        .map_err(|_| ApiError::internal("Failed to publish XMTP swarm message"))?;

    Ok(envelope)
}

pub async fn list_messages(
    state: &AppState,
    swarm_id: &str,
    query: SwarmListQuery,
) -> Result<SwarmMessagesResponse, ApiError> {
    if !state.config.xmtp_swarm_enabled {
        return Err(ApiError::bad_request(
            "XMTP_SWARM_DISABLED",
            "XMTP swarm messaging is disabled",
        ));
    }

    let validated_swarm_id = validate_swarm_id(swarm_id)?;
    let limit = query.limit.unwrap_or(50).min(200);
    let offset = query.offset.unwrap_or(0);
    let end = offset.saturating_add(limit).saturating_sub(1);
    let raw_values = state
        .redis
        .list_range_raw(
            message_key(validated_swarm_id.as_str()).as_str(),
            offset,
            end,
        )
        .await
        .map_err(|_| ApiError::internal("Failed to load XMTP swarm messages"))?;

    let mut data = Vec::with_capacity(raw_values.len());
    for raw in raw_values {
        if let Ok(entry) = serde_json::from_str::<SwarmMessage>(raw.as_str()) {
            data.push(entry);
        }
    }

    Ok(SwarmMessagesResponse {
        total_returned: data.len(),
        limit,
        offset,
        topic: topic(state, validated_swarm_id.as_str()),
        data,
    })
}

pub fn health(state: &AppState) -> Value {
    serde_json::json!({
        "enabled": state.config.xmtp_swarm_enabled,
        "topic_prefix": state.config.xmtp_swarm_topic_prefix,
        "max_messages": state.config.xmtp_swarm_max_messages,
        "max_message_bytes": state.config.xmtp_swarm_max_message_bytes
    })
}
