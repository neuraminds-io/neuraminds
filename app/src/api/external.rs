use actix_web::{web, HttpRequest, HttpResponse, Responder};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

use crate::api::auth::extract_authenticated_user;
use crate::api::ApiError;
use crate::services::external;
use crate::services::external::credentials::{decrypt_json, encrypt_json, mask_secret};
use crate::services::external::types::{ExternalMarketId, ExternalProvider};
use crate::AppState;

const MAX_PAGE_SIZE: i64 = 200;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListExternalCredentialsQuery {
    pub provider: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertExternalCredentialRequest {
    pub provider: String,
    pub label: Option<String>,
    pub credentials: Value,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalCredentialResponse {
    pub id: String,
    pub provider: String,
    pub label: String,
    pub key_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub credentials: Value,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalCredentialsListResponse {
    pub credentials: Vec<ExternalCredentialResponse>,
    pub total: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateExternalOrderIntentRequest {
    pub provider: String,
    pub market_id: String,
    pub outcome: String,
    pub side: String,
    pub price: f64,
    pub quantity: f64,
    pub credential_id: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalOrderIntentResponse {
    pub id: String,
    pub provider: String,
    pub market_id: String,
    pub preflight: Value,
    pub typed_data: Value,
    pub status: String,
    pub expires_at: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitExternalOrderRequest {
    pub intent_id: String,
    pub signed_order: Value,
    pub credential_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelExternalOrderRequest {
    pub provider: String,
    pub provider_order_id: String,
    pub credential_id: Option<String>,
    pub payload: Option<Value>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalOrderResponse {
    pub id: String,
    pub provider: String,
    pub market_id: String,
    pub provider_order_id: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub response_payload: Value,
    pub error_message: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListExternalOrdersQuery {
    pub provider: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalOrdersListResponse {
    pub orders: Vec<ExternalOrderResponse>,
    pub total: u64,
    pub limit: u64,
    pub offset: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateExternalAgentRequest {
    pub name: String,
    pub provider: String,
    pub market_id: String,
    pub outcome: String,
    pub side: String,
    pub price: f64,
    pub quantity: f64,
    pub cadence_seconds: u64,
    pub strategy: String,
    pub credential_id: Option<String>,
    pub active: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateExternalAgentRequest {
    pub name: Option<String>,
    pub outcome: Option<String>,
    pub side: Option<String>,
    pub price: Option<f64>,
    pub quantity: Option<f64>,
    pub cadence_seconds: Option<u64>,
    pub strategy: Option<String>,
    pub credential_id: Option<String>,
    pub active: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteExternalAgentRequest {
    pub force: Option<bool>,
    pub signed_order: Option<Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListExternalAgentsQuery {
    pub provider: Option<String>,
    pub active: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalAgentResponse {
    pub id: String,
    pub owner: String,
    pub name: String,
    pub provider: String,
    pub market_id: String,
    pub outcome: String,
    pub side: String,
    pub price: f64,
    pub quantity: f64,
    pub cadence_seconds: u64,
    pub strategy: String,
    pub credential_id: Option<String>,
    pub active: bool,
    pub last_executed_at: Option<String>,
    pub next_execution_at: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalAgentsListResponse {
    pub agents: Vec<ExternalAgentResponse>,
    pub total: u64,
    pub limit: u64,
    pub offset: u64,
}

#[derive(Debug, Clone)]
struct StoredCredential {
    id: String,
    payload: Value,
}

fn normalize_provider(raw: &str) -> Result<ExternalProvider, ApiError> {
    ExternalProvider::from_str(raw).ok_or_else(|| {
        ApiError::bad_request(
            "INVALID_PROVIDER",
            "provider must be one of: limitless, polymarket",
        )
    })
}

fn normalize_side(raw: &str) -> Result<String, ApiError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "buy" | "sell" => Ok(raw.trim().to_ascii_lowercase()),
        _ => Err(ApiError::bad_request(
            "INVALID_SIDE",
            "side must be one of: buy, sell",
        )),
    }
}

fn normalize_outcome(raw: &str) -> Result<String, ApiError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "yes" | "no" => Ok(raw.trim().to_ascii_lowercase()),
        _ => Err(ApiError::bad_request(
            "INVALID_OUTCOME",
            "outcome must be one of: yes, no",
        )),
    }
}

fn normalize_namespaced_market_id(provider: ExternalProvider, market_id: &str) -> String {
    if market_id.contains(':') {
        return market_id.trim().to_string();
    }
    format!("{}:{}", provider.as_str(), market_id.trim())
}

fn mask_credentials(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut next = serde_json::Map::new();
            for (key, raw) in map {
                if raw.is_string() {
                    let masked = mask_secret(raw.as_str().unwrap_or_default());
                    next.insert(key.clone(), Value::String(masked));
                } else {
                    next.insert(key.clone(), mask_credentials(raw));
                }
            }
            Value::Object(next)
        }
        Value::Array(entries) => Value::Array(entries.iter().map(mask_credentials).collect()),
        _ => value.clone(),
    }
}

fn ensure_external_features_enabled(state: &AppState) -> Result<(), ApiError> {
    if !state.config.external_markets_enabled {
        return Err(ApiError::bad_request(
            "EXTERNAL_MARKETS_DISABLED",
            "external market integration is disabled",
        ));
    }
    Ok(())
}

async fn load_credential(
    state: &AppState,
    owner: &str,
    provider: ExternalProvider,
    credential_id: Option<&str>,
) -> Result<StoredCredential, ApiError> {
    let row = if let Some(id) = credential_id {
        sqlx::query(
            "SELECT id, provider, label, encrypted_payload, key_id
             FROM external_credentials
             WHERE id = $1 AND owner = $2 AND revoked_at IS NULL",
        )
        .bind(id)
        .bind(owner)
        .fetch_optional(state.db.pool())
        .await
        .map_err(|err| ApiError::internal(&err.to_string()))?
    } else {
        sqlx::query(
            "SELECT id, provider, label, encrypted_payload, key_id
             FROM external_credentials
             WHERE owner = $1 AND provider = $2 AND revoked_at IS NULL
             ORDER BY updated_at DESC
             LIMIT 1",
        )
        .bind(owner)
        .bind(provider.as_str())
        .fetch_optional(state.db.pool())
        .await
        .map_err(|err| ApiError::internal(&err.to_string()))?
    };

    let row = row.ok_or_else(|| {
        ApiError::bad_request(
            "CREDENTIAL_NOT_FOUND",
            "no active credential found for provider",
        )
    })?;

    let encrypted_payload: String = row
        .try_get("encrypted_payload")
        .map_err(|err| ApiError::internal(&err.to_string()))?;
    let key_id: String = row
        .try_get("key_id")
        .map_err(|err| ApiError::internal(&err.to_string()))?;

    let payload = decrypt_json(
        state.config.external_credentials_master_key.as_str(),
        key_id.as_str(),
        encrypted_payload.as_str(),
    )?;

    Ok(StoredCredential {
        id: row
            .try_get("id")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
        payload,
    })
}

fn api_key_from_payload(payload: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(value) = payload.get(*key).and_then(|entry| entry.as_str()) {
            if !value.trim().is_empty() {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

fn build_preflight(provider: ExternalProvider, market: &Value) -> Value {
    match provider {
        ExternalProvider::Limitless => {
            let venue_exchange = market
                .get("venue")
                .and_then(|value| value.get("exchange"))
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            json!({
                "chainId": 8453,
                "mode": "manual",
                "checks": [
                    {
                        "type": "funding",
                        "token": "USDC",
                        "chainId": 8453,
                        "required": true,
                        "message": "Ensure the trading wallet is funded with Base USDC"
                    },
                    {
                        "type": "approval",
                        "token": "USDC",
                        "spender": venue_exchange,
                        "required": true,
                        "message": "Approve venue exchange to spend USDC"
                    }
                ]
            })
        }
        ExternalProvider::Polymarket => json!({
            "chainId": 137,
            "mode": "manual",
            "checks": [
                {
                    "type": "funding",
                    "token": "USDC",
                    "chainId": 137,
                    "required": true,
                    "message": "Fund Polygon wallet for Polymarket execution"
                },
                {
                    "type": "approval",
                    "token": "USDC",
                    "required": true,
                    "message": "Set required CLOB allowance(s) before trading"
                }
            ]
        }),
    }
}

fn build_typed_data(
    owner: &str,
    provider: ExternalProvider,
    request: &CreateExternalOrderIntentRequest,
    market_ref: &str,
) -> Value {
    json!({
        "provider": provider.as_str(),
        "primaryType": "Order",
        "domain": {
            "name": match provider {
                ExternalProvider::Limitless => "Limitless Exchange",
                ExternalProvider::Polymarket => "Polymarket CLOB",
            },
            "chainId": provider.chain_id(),
            "verifyingContract": market_ref,
        },
        "message": {
            "maker": owner,
            "marketId": request.market_id,
            "outcome": request.outcome,
            "side": request.side,
            "price": request.price,
            "quantity": request.quantity,
            "nonce": Uuid::new_v4().to_string(),
            "expiration": (Utc::now() + Duration::hours(2)).to_rfc3339(),
        }
    })
}

async fn submit_to_provider(
    state: &AppState,
    provider: ExternalProvider,
    credential: &StoredCredential,
    signed_order: &Value,
) -> Result<Value, ApiError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|err| ApiError::internal(&err.to_string()))?;

    match provider {
        ExternalProvider::Limitless => {
            let api_key = api_key_from_payload(&credential.payload, &["apiKey", "api_key"])
                .ok_or_else(|| {
                    ApiError::bad_request(
                        "INVALID_CREDENTIALS",
                        "limitless credential must include apiKey",
                    )
                })?;

            let response = client
                .post(format!(
                    "{}/orders",
                    state.config.limitless_api_base.trim_end_matches('/')
                ))
                .header("X-API-Key", api_key)
                .json(signed_order)
                .send()
                .await
                .map_err(|err| ApiError::internal(&format!("limitless submit failed: {}", err)))?;

            let status = response.status();
            let payload = response
                .json::<Value>()
                .await
                .unwrap_or_else(|_| json!({ "ok": status.is_success() }));

            if !status.is_success() {
                return Err(ApiError::bad_request(
                    "LIMITLESS_SUBMIT_FAILED",
                    payload
                        .get("message")
                        .and_then(|value| value.as_str())
                        .unwrap_or("limitless order submission failed"),
                ));
            }

            Ok(payload)
        }
        ExternalProvider::Polymarket => {
            let api_key = api_key_from_payload(&credential.payload, &["apiKey", "api_key"])
                .ok_or_else(|| {
                    ApiError::bad_request(
                        "INVALID_CREDENTIALS",
                        "polymarket credential must include apiKey",
                    )
                })?;
            let api_secret =
                api_key_from_payload(&credential.payload, &["apiSecret", "api_secret"])
                    .ok_or_else(|| {
                        ApiError::bad_request(
                            "INVALID_CREDENTIALS",
                            "polymarket credential must include apiSecret",
                        )
                    })?;
            let api_passphrase =
                api_key_from_payload(&credential.payload, &["apiPassphrase", "api_passphrase"])
                    .ok_or_else(|| {
                        ApiError::bad_request(
                            "INVALID_CREDENTIALS",
                            "polymarket credential must include apiPassphrase",
                        )
                    })?;

            let response = client
                .post(format!(
                    "{}/order",
                    state.config.polymarket_clob_api_base.trim_end_matches('/')
                ))
                .header("POLY_API_KEY", api_key)
                .header("POLY_API_SECRET", api_secret)
                .header("POLY_PASSPHRASE", api_passphrase)
                .json(signed_order)
                .send()
                .await
                .map_err(|err| ApiError::internal(&format!("polymarket submit failed: {}", err)))?;

            let status = response.status();
            let payload = response
                .json::<Value>()
                .await
                .unwrap_or_else(|_| json!({ "ok": status.is_success() }));

            if !status.is_success() {
                return Err(ApiError::bad_request(
                    "POLYMARKET_SUBMIT_FAILED",
                    payload
                        .get("error")
                        .and_then(|value| value.as_str())
                        .unwrap_or("polymarket order submission failed"),
                ));
            }

            Ok(payload)
        }
    }
}

async fn cancel_on_provider(
    state: &AppState,
    provider: ExternalProvider,
    credential: &StoredCredential,
    provider_order_id: &str,
    payload: Option<Value>,
) -> Result<Value, ApiError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|err| ApiError::internal(&err.to_string()))?;

    match provider {
        ExternalProvider::Limitless => {
            let api_key = api_key_from_payload(&credential.payload, &["apiKey", "api_key"])
                .ok_or_else(|| {
                    ApiError::bad_request(
                        "INVALID_CREDENTIALS",
                        "limitless credential must include apiKey",
                    )
                })?;

            let response = client
                .delete(format!(
                    "{}/orders/{}",
                    state.config.limitless_api_base.trim_end_matches('/'),
                    provider_order_id
                ))
                .header("X-API-Key", api_key)
                .send()
                .await
                .map_err(|err| ApiError::internal(&format!("limitless cancel failed: {}", err)))?;

            let status = response.status();
            let body = response
                .json::<Value>()
                .await
                .unwrap_or_else(|_| json!({ "ok": status.is_success() }));

            if !status.is_success() {
                return Err(ApiError::bad_request(
                    "LIMITLESS_CANCEL_FAILED",
                    body.get("message")
                        .and_then(|value| value.as_str())
                        .unwrap_or("limitless cancel failed"),
                ));
            }
            Ok(body)
        }
        ExternalProvider::Polymarket => {
            let api_key = api_key_from_payload(&credential.payload, &["apiKey", "api_key"])
                .ok_or_else(|| {
                    ApiError::bad_request(
                        "INVALID_CREDENTIALS",
                        "polymarket credential must include apiKey",
                    )
                })?;
            let api_secret =
                api_key_from_payload(&credential.payload, &["apiSecret", "api_secret"])
                    .ok_or_else(|| {
                        ApiError::bad_request(
                            "INVALID_CREDENTIALS",
                            "polymarket credential must include apiSecret",
                        )
                    })?;
            let api_passphrase =
                api_key_from_payload(&credential.payload, &["apiPassphrase", "api_passphrase"])
                    .ok_or_else(|| {
                        ApiError::bad_request(
                            "INVALID_CREDENTIALS",
                            "polymarket credential must include apiPassphrase",
                        )
                    })?;

            let body = payload.unwrap_or_else(|| json!({ "orderID": provider_order_id }));
            let response = client
                .delete(format!(
                    "{}/order",
                    state.config.polymarket_clob_api_base.trim_end_matches('/')
                ))
                .header("POLY_API_KEY", api_key)
                .header("POLY_API_SECRET", api_secret)
                .header("POLY_PASSPHRASE", api_passphrase)
                .json(&body)
                .send()
                .await
                .map_err(|err| ApiError::internal(&format!("polymarket cancel failed: {}", err)))?;

            let status = response.status();
            let payload = response
                .json::<Value>()
                .await
                .unwrap_or_else(|_| json!({ "ok": status.is_success() }));

            if !status.is_success() {
                return Err(ApiError::bad_request(
                    "POLYMARKET_CANCEL_FAILED",
                    payload
                        .get("error")
                        .and_then(|value| value.as_str())
                        .unwrap_or("polymarket cancel failed"),
                ));
            }

            Ok(payload)
        }
    }
}

fn build_external_order_response(
    row: sqlx::postgres::PgRow,
) -> Result<ExternalOrderResponse, ApiError> {
    let created_at: chrono::DateTime<Utc> = row
        .try_get("created_at")
        .map_err(|err| ApiError::internal(&err.to_string()))?;
    let updated_at: chrono::DateTime<Utc> = row
        .try_get("updated_at")
        .map_err(|err| ApiError::internal(&err.to_string()))?;

    Ok(ExternalOrderResponse {
        id: row
            .try_get("id")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
        provider: row
            .try_get("provider")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
        market_id: row
            .try_get("market_id")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
        provider_order_id: row
            .try_get("provider_order_id")
            .unwrap_or_else(|_| String::new()),
        status: row
            .try_get("status")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
        created_at: created_at.to_rfc3339(),
        updated_at: updated_at.to_rfc3339(),
        response_payload: row
            .try_get("response_payload")
            .unwrap_or_else(|_| json!({})),
        error_message: row.try_get("error_message").ok(),
    })
}

fn parse_external_agent(row: sqlx::postgres::PgRow) -> Result<ExternalAgentResponse, ApiError> {
    let created_at: chrono::DateTime<Utc> = row
        .try_get("created_at")
        .map_err(|err| ApiError::internal(&err.to_string()))?;
    let updated_at: chrono::DateTime<Utc> = row
        .try_get("updated_at")
        .map_err(|err| ApiError::internal(&err.to_string()))?;
    let next_execution_at: chrono::DateTime<Utc> = row
        .try_get("next_execution_at")
        .map_err(|err| ApiError::internal(&err.to_string()))?;
    let last_executed_at: Option<chrono::DateTime<Utc>> = row.try_get("last_executed_at").ok();

    Ok(ExternalAgentResponse {
        id: row
            .try_get("id")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
        owner: row
            .try_get("owner")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
        name: row
            .try_get("name")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
        provider: row
            .try_get("provider")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
        market_id: row
            .try_get("market_id")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
        outcome: row
            .try_get("outcome")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
        side: row
            .try_get("side")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
        price: row
            .try_get("price")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
        quantity: row
            .try_get("quantity")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
        cadence_seconds: row
            .try_get::<i64, _>("cadence_seconds")
            .map_err(|err| ApiError::internal(&err.to_string()))?
            .max(1) as u64,
        strategy: row
            .try_get("strategy")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
        credential_id: row.try_get("credential_id").ok(),
        active: row
            .try_get("active")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
        last_executed_at: last_executed_at.map(|entry| entry.to_rfc3339()),
        next_execution_at: next_execution_at.to_rfc3339(),
        created_at: created_at.to_rfc3339(),
        updated_at: updated_at.to_rfc3339(),
    })
}

pub async fn list_external_credentials(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
    query: web::Query<ListExternalCredentialsQuery>,
) -> Result<impl Responder, ApiError> {
    ensure_external_features_enabled(&state)?;
    let user = extract_authenticated_user(&req, &state).await?;

    let provider_filter = query
        .provider
        .as_ref()
        .map(|value| normalize_provider(value))
        .transpose()?;

    let rows = if let Some(provider) = provider_filter {
        sqlx::query(
            "SELECT id, provider, label, encrypted_payload, key_id, created_at, updated_at
             FROM external_credentials
             WHERE owner = $1 AND provider = $2 AND revoked_at IS NULL
             ORDER BY updated_at DESC",
        )
        .bind(user.wallet_address.as_str())
        .bind(provider.as_str())
        .fetch_all(state.db.pool())
        .await
        .map_err(|err| ApiError::internal(&err.to_string()))?
    } else {
        sqlx::query(
            "SELECT id, provider, label, encrypted_payload, key_id, created_at, updated_at
             FROM external_credentials
             WHERE owner = $1 AND revoked_at IS NULL
             ORDER BY updated_at DESC",
        )
        .bind(user.wallet_address.as_str())
        .fetch_all(state.db.pool())
        .await
        .map_err(|err| ApiError::internal(&err.to_string()))?
    };

    let mut credentials = Vec::new();
    for row in rows {
        let encrypted_payload: String = row
            .try_get("encrypted_payload")
            .map_err(|err| ApiError::internal(&err.to_string()))?;
        let key_id: String = row
            .try_get("key_id")
            .map_err(|err| ApiError::internal(&err.to_string()))?;
        let payload = decrypt_json(
            state.config.external_credentials_master_key.as_str(),
            key_id.as_str(),
            encrypted_payload.as_str(),
        )
        .unwrap_or_else(|_| json!({}));

        let created_at: chrono::DateTime<Utc> = row
            .try_get("created_at")
            .map_err(|err| ApiError::internal(&err.to_string()))?;
        let updated_at: chrono::DateTime<Utc> = row
            .try_get("updated_at")
            .map_err(|err| ApiError::internal(&err.to_string()))?;

        credentials.push(ExternalCredentialResponse {
            id: row
                .try_get("id")
                .map_err(|err| ApiError::internal(&err.to_string()))?,
            provider: row
                .try_get("provider")
                .map_err(|err| ApiError::internal(&err.to_string()))?,
            label: row
                .try_get("label")
                .map_err(|err| ApiError::internal(&err.to_string()))?,
            key_id,
            created_at: created_at.to_rfc3339(),
            updated_at: updated_at.to_rfc3339(),
            credentials: mask_credentials(&payload),
        });
    }

    Ok(HttpResponse::Ok().json(ExternalCredentialsListResponse {
        total: credentials.len() as u64,
        credentials,
    }))
}

pub async fn upsert_external_credentials(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
    body: web::Json<UpsertExternalCredentialRequest>,
) -> Result<impl Responder, ApiError> {
    ensure_external_features_enabled(&state)?;
    let user = extract_authenticated_user(&req, &state).await?;

    let provider = normalize_provider(body.provider.as_str())?;
    let label = body
        .label
        .as_deref()
        .unwrap_or("default")
        .trim()
        .to_ascii_lowercase();
    if label.is_empty() {
        return Err(ApiError::bad_request(
            "INVALID_LABEL",
            "label must not be empty",
        ));
    }

    if !body.credentials.is_object() {
        return Err(ApiError::bad_request(
            "INVALID_CREDENTIALS",
            "credentials must be an object",
        ));
    }

    let encrypted_payload = encrypt_json(
        state.config.external_credentials_master_key.as_str(),
        state.config.external_credentials_key_id.as_str(),
        &body.credentials,
    )?;

    let row = sqlx::query(
        "INSERT INTO external_credentials (
            id, owner, provider, label, encrypted_payload, key_id, created_at, updated_at, revoked_at
        ) VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW(), NULL)
        ON CONFLICT (owner, provider, label)
        DO UPDATE SET encrypted_payload = EXCLUDED.encrypted_payload,
                      key_id = EXCLUDED.key_id,
                      updated_at = NOW(),
                      revoked_at = NULL
        RETURNING id, provider, label, key_id, created_at, updated_at",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(user.wallet_address.as_str())
    .bind(provider.as_str())
    .bind(label)
    .bind(encrypted_payload)
    .bind(state.config.external_credentials_key_id.as_str())
    .fetch_one(state.db.pool())
    .await
    .map_err(|err| ApiError::internal(&err.to_string()))?;

    let created_at: chrono::DateTime<Utc> = row
        .try_get("created_at")
        .map_err(|err| ApiError::internal(&err.to_string()))?;
    let updated_at: chrono::DateTime<Utc> = row
        .try_get("updated_at")
        .map_err(|err| ApiError::internal(&err.to_string()))?;

    Ok(HttpResponse::Ok().json(ExternalCredentialResponse {
        id: row
            .try_get("id")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
        provider: row
            .try_get("provider")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
        label: row
            .try_get("label")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
        key_id: row
            .try_get("key_id")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
        created_at: created_at.to_rfc3339(),
        updated_at: updated_at.to_rfc3339(),
        credentials: mask_credentials(&body.credentials),
    }))
}

pub async fn delete_external_credentials(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
    credential_id: web::Path<String>,
) -> Result<impl Responder, ApiError> {
    ensure_external_features_enabled(&state)?;
    let user = extract_authenticated_user(&req, &state).await?;

    let result = sqlx::query(
        "UPDATE external_credentials
         SET revoked_at = NOW(), updated_at = NOW()
         WHERE id = $1 AND owner = $2 AND revoked_at IS NULL",
    )
    .bind(credential_id.as_str())
    .bind(user.wallet_address.as_str())
    .execute(state.db.pool())
    .await
    .map_err(|err| ApiError::internal(&err.to_string()))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("External credential"));
    }

    Ok(HttpResponse::Ok().json(json!({ "ok": true })))
}

pub async fn create_external_order_intent(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
    body: web::Json<CreateExternalOrderIntentRequest>,
) -> Result<impl Responder, ApiError> {
    ensure_external_features_enabled(&state)?;
    if !state.config.external_trading_enabled {
        return Err(ApiError::bad_request(
            "EXTERNAL_TRADING_DISABLED",
            "external trading is disabled",
        ));
    }

    let user = extract_authenticated_user(&req, &state).await?;
    let provider = normalize_provider(body.provider.as_str())?;
    let outcome = normalize_outcome(body.outcome.as_str())?;
    let side = normalize_side(body.side.as_str())?;

    if body.price <= 0.0 || body.price >= 1.0 {
        return Err(ApiError::bad_request(
            "INVALID_PRICE",
            "price must be between 0 and 1",
        ));
    }
    if body.quantity <= 0.0 {
        return Err(ApiError::bad_request(
            "INVALID_QUANTITY",
            "quantity must be greater than zero",
        ));
    }

    let namespaced_market_id = normalize_namespaced_market_id(provider, body.market_id.as_str());
    let parsed_market_id = ExternalMarketId::parse(namespaced_market_id.as_str())?;
    let market = external::fetch_market_by_id(&state.config, &parsed_market_id).await?;

    if !market.execution_users {
        return Err(ApiError::bad_request(
            "MARKET_NOT_EXECUTABLE",
            "market is not executable for users under current launch policy",
        ));
    }

    let credential = load_credential(
        &state,
        user.wallet_address.as_str(),
        provider,
        body.credential_id.as_deref(),
    )
    .await?;

    let market_ref = if !market.provider_market_ref.is_empty() {
        market.provider_market_ref.clone()
    } else {
        parsed_market_id.value.clone()
    };

    let provider_market_payload = match provider {
        ExternalProvider::Limitless => {
            let client = reqwest::Client::new();
            match client
                .get(format!(
                    "{}/markets/{}",
                    state.config.limitless_api_base.trim_end_matches('/'),
                    parsed_market_id.value
                ))
                .send()
                .await
            {
                Ok(response) => response.json::<Value>().await.unwrap_or_else(|_| json!({})),
                Err(_) => json!({}),
            }
        }
        ExternalProvider::Polymarket => {
            let client = reqwest::Client::new();
            match client
                .get(format!(
                    "{}/markets/{}",
                    state.config.polymarket_gamma_api_base.trim_end_matches('/'),
                    parsed_market_id.value
                ))
                .send()
                .await
            {
                Ok(response) => response.json::<Value>().await.unwrap_or_else(|_| json!({})),
                Err(_) => json!({}),
            }
        }
    };

    let preflight = build_preflight(provider, &provider_market_payload);
    let intent_for_signing = CreateExternalOrderIntentRequest {
        provider: provider.as_str().to_string(),
        market_id: namespaced_market_id.clone(),
        outcome,
        side,
        price: body.price,
        quantity: body.quantity,
        credential_id: body.credential_id.clone(),
    };
    let typed_data = build_typed_data(
        user.wallet_address.as_str(),
        provider,
        &intent_for_signing,
        market_ref.as_str(),
    );

    let intent_id = Uuid::new_v4().to_string();
    let expires_at = (Utc::now() + Duration::hours(2)).to_rfc3339();

    sqlx::query(
        "INSERT INTO external_order_intents (
            id, owner, provider, market_id, provider_market_ref, outcome, side,
            price, quantity, preflight, typed_data, status, credential_id, created_at, updated_at
         ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,'prepared',$12,NOW(),NOW())",
    )
    .bind(intent_id.as_str())
    .bind(user.wallet_address.as_str())
    .bind(provider.as_str())
    .bind(namespaced_market_id.as_str())
    .bind(market_ref.as_str())
    .bind(intent_for_signing.outcome.as_str())
    .bind(intent_for_signing.side.as_str())
    .bind(intent_for_signing.price)
    .bind(intent_for_signing.quantity)
    .bind(&preflight)
    .bind(&typed_data)
    .bind(credential.id.as_str())
    .execute(state.db.pool())
    .await
    .map_err(|err| ApiError::internal(&err.to_string()))?;

    Ok(HttpResponse::Ok().json(ExternalOrderIntentResponse {
        id: intent_id,
        provider: provider.as_str().to_string(),
        market_id: namespaced_market_id,
        preflight,
        typed_data,
        status: "prepared".to_string(),
        expires_at,
    }))
}

pub async fn submit_external_order(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
    body: web::Json<SubmitExternalOrderRequest>,
) -> Result<impl Responder, ApiError> {
    ensure_external_features_enabled(&state)?;
    if !state.config.external_trading_enabled {
        return Err(ApiError::bad_request(
            "EXTERNAL_TRADING_DISABLED",
            "external trading is disabled",
        ));
    }

    let user = extract_authenticated_user(&req, &state).await?;

    let row = sqlx::query(
        "SELECT id, provider, market_id, credential_id, status
         FROM external_order_intents
         WHERE id = $1 AND owner = $2",
    )
    .bind(body.intent_id.as_str())
    .bind(user.wallet_address.as_str())
    .fetch_optional(state.db.pool())
    .await
    .map_err(|err| ApiError::internal(&err.to_string()))?
    .ok_or_else(|| ApiError::not_found("External order intent"))?;

    let provider_raw: String = row
        .try_get("provider")
        .map_err(|err| ApiError::internal(&err.to_string()))?;
    let provider = normalize_provider(provider_raw.as_str())?;

    let credential_id = body
        .credential_id
        .as_deref()
        .map(ToOwned::to_owned)
        .or_else(|| row.try_get::<String, _>("credential_id").ok());

    let credential = load_credential(
        &state,
        user.wallet_address.as_str(),
        provider,
        credential_id.as_deref(),
    )
    .await?;

    let provider_response =
        submit_to_provider(&state, provider, &credential, &body.signed_order).await;
    let now = Utc::now();
    let order_id = Uuid::new_v4().to_string();

    let (status, payload, error_message, provider_order_id) = match provider_response {
        Ok(payload) => {
            let provider_order_id = payload
                .get("orderId")
                .or_else(|| payload.get("id"))
                .or_else(|| payload.get("order_id"))
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string();
            ("submitted".to_string(), payload, None, provider_order_id)
        }
        Err(err) => (
            "failed".to_string(),
            json!({ "error": err.message }),
            Some(err.message),
            String::new(),
        ),
    };

    sqlx::query(
        "INSERT INTO external_orders (
            id, owner, provider, intent_id, market_id, provider_order_id, status,
            request_payload, response_payload, error_message, created_at, updated_at
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)",
    )
    .bind(order_id.as_str())
    .bind(user.wallet_address.as_str())
    .bind(provider.as_str())
    .bind(body.intent_id.as_str())
    .bind(
        row.try_get::<String, _>("market_id")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
    )
    .bind(provider_order_id.as_str())
    .bind(status.as_str())
    .bind(&body.signed_order)
    .bind(&payload)
    .bind(error_message.as_deref())
    .bind(now)
    .bind(now)
    .execute(state.db.pool())
    .await
    .map_err(|err| ApiError::internal(&err.to_string()))?;

    let next_intent_status = if status == "submitted" {
        "submitted"
    } else {
        "failed"
    };
    sqlx::query("UPDATE external_order_intents SET status = $2, updated_at = NOW() WHERE id = $1")
        .bind(body.intent_id.as_str())
        .bind(next_intent_status)
        .execute(state.db.pool())
        .await
        .map_err(|err| ApiError::internal(&err.to_string()))?;

    if status != "submitted" {
        return Err(ApiError::bad_request(
            "EXTERNAL_ORDER_SUBMIT_FAILED",
            error_message
                .as_deref()
                .unwrap_or("external order submission failed"),
        ));
    }

    Ok(HttpResponse::Ok().json(ExternalOrderResponse {
        id: order_id,
        provider: provider.as_str().to_string(),
        market_id: row
            .try_get("market_id")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
        provider_order_id,
        status,
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
        response_payload: payload,
        error_message: None,
    }))
}

pub async fn cancel_external_order(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
    body: web::Json<CancelExternalOrderRequest>,
) -> Result<impl Responder, ApiError> {
    ensure_external_features_enabled(&state)?;
    if !state.config.external_trading_enabled {
        return Err(ApiError::bad_request(
            "EXTERNAL_TRADING_DISABLED",
            "external trading is disabled",
        ));
    }

    let user = extract_authenticated_user(&req, &state).await?;
    let provider = normalize_provider(body.provider.as_str())?;
    let credential = load_credential(
        &state,
        user.wallet_address.as_str(),
        provider,
        body.credential_id.as_deref(),
    )
    .await?;

    let response_payload = cancel_on_provider(
        &state,
        provider,
        &credential,
        body.provider_order_id.as_str(),
        body.payload.clone(),
    )
    .await?;

    sqlx::query(
        "UPDATE external_orders
         SET status = 'cancelled', response_payload = $1, updated_at = NOW()
         WHERE owner = $2 AND provider = $3 AND provider_order_id = $4",
    )
    .bind(&response_payload)
    .bind(user.wallet_address.as_str())
    .bind(provider.as_str())
    .bind(body.provider_order_id.as_str())
    .execute(state.db.pool())
    .await
    .map_err(|err| ApiError::internal(&err.to_string()))?;

    Ok(HttpResponse::Ok().json(json!({
        "ok": true,
        "provider": provider.as_str(),
        "providerOrderId": body.provider_order_id,
        "response": response_payload,
    })))
}

pub async fn list_external_orders(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
    query: web::Query<ListExternalOrdersQuery>,
) -> Result<impl Responder, ApiError> {
    ensure_external_features_enabled(&state)?;
    let user = extract_authenticated_user(&req, &state).await?;

    let limit = query.limit.unwrap_or(50).clamp(1, MAX_PAGE_SIZE);
    let offset = query.offset.unwrap_or(0).max(0);

    let rows = if let Some(provider_raw) = query.provider.as_ref() {
        let provider = normalize_provider(provider_raw.as_str())?;
        sqlx::query(
            "SELECT id, provider, market_id, provider_order_id, status, response_payload, error_message, created_at, updated_at
             FROM external_orders
             WHERE owner = $1 AND provider = $2
             ORDER BY created_at DESC
             LIMIT $3 OFFSET $4",
        )
        .bind(user.wallet_address.as_str())
        .bind(provider.as_str())
        .bind(limit)
        .bind(offset)
        .fetch_all(state.db.pool())
        .await
        .map_err(|err| ApiError::internal(&err.to_string()))?
    } else {
        sqlx::query(
            "SELECT id, provider, market_id, provider_order_id, status, response_payload, error_message, created_at, updated_at
             FROM external_orders
             WHERE owner = $1
             ORDER BY created_at DESC
             LIMIT $2 OFFSET $3",
        )
        .bind(user.wallet_address.as_str())
        .bind(limit)
        .bind(offset)
        .fetch_all(state.db.pool())
        .await
        .map_err(|err| ApiError::internal(&err.to_string()))?
    };

    let total_row = sqlx::query("SELECT COUNT(*) AS total FROM external_orders WHERE owner = $1")
        .bind(user.wallet_address.as_str())
        .fetch_one(state.db.pool())
        .await
        .map_err(|err| ApiError::internal(&err.to_string()))?;
    let total: i64 = total_row
        .try_get("total")
        .map_err(|err| ApiError::internal(&err.to_string()))?;

    let mut orders = Vec::new();
    for row in rows {
        orders.push(build_external_order_response(row)?);
    }

    Ok(HttpResponse::Ok().json(ExternalOrdersListResponse {
        orders,
        total: total.max(0) as u64,
        limit: limit as u64,
        offset: offset as u64,
    }))
}

pub async fn list_external_agents(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
    query: web::Query<ListExternalAgentsQuery>,
) -> Result<impl Responder, ApiError> {
    ensure_external_features_enabled(&state)?;
    let user = extract_authenticated_user(&req, &state).await?;

    let limit = query.limit.unwrap_or(50).clamp(1, MAX_PAGE_SIZE);
    let offset = query.offset.unwrap_or(0).max(0);

    let mut sql = String::from(
        "SELECT id, owner, name, provider, market_id, outcome, side, price, quantity, cadence_seconds,
                strategy, credential_id, active, last_executed_at, next_execution_at, created_at, updated_at
         FROM external_agents
         WHERE owner = $1",
    );

    let mut bind_provider: Option<String> = None;
    let mut bind_active: Option<bool> = None;

    if let Some(provider_raw) = query.provider.as_ref() {
        let provider = normalize_provider(provider_raw.as_str())?;
        bind_provider = Some(provider.as_str().to_string());
        sql.push_str(" AND provider = $2");
    }
    if let Some(active) = query.active {
        bind_active = Some(active);
        sql.push_str(if bind_provider.is_some() {
            " AND active = $3"
        } else {
            " AND active = $2"
        });
    }

    let pagination_index = if bind_provider.is_some() && bind_active.is_some() {
        4
    } else if bind_provider.is_some() || bind_active.is_some() {
        3
    } else {
        2
    };
    sql.push_str(&format!(
        " ORDER BY created_at DESC LIMIT ${} OFFSET ${}",
        pagination_index,
        pagination_index + 1
    ));

    let mut query_builder = sqlx::query(sql.as_str()).bind(user.wallet_address.as_str());
    if let Some(provider) = bind_provider.as_ref() {
        query_builder = query_builder.bind(provider);
    }
    if let Some(active) = bind_active {
        query_builder = query_builder.bind(active);
    }
    query_builder = query_builder.bind(limit).bind(offset);

    let rows = query_builder
        .fetch_all(state.db.pool())
        .await
        .map_err(|err| ApiError::internal(&err.to_string()))?;

    let count_row = sqlx::query("SELECT COUNT(*) AS total FROM external_agents WHERE owner = $1")
        .bind(user.wallet_address.as_str())
        .fetch_one(state.db.pool())
        .await
        .map_err(|err| ApiError::internal(&err.to_string()))?;
    let total: i64 = count_row
        .try_get("total")
        .map_err(|err| ApiError::internal(&err.to_string()))?;

    let mut agents = Vec::new();
    for row in rows {
        agents.push(parse_external_agent(row)?);
    }

    Ok(HttpResponse::Ok().json(ExternalAgentsListResponse {
        agents,
        total: total.max(0) as u64,
        limit: limit as u64,
        offset: offset as u64,
    }))
}

pub async fn create_external_agent(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
    body: web::Json<CreateExternalAgentRequest>,
) -> Result<impl Responder, ApiError> {
    ensure_external_features_enabled(&state)?;
    if !state.config.external_agents_enabled {
        return Err(ApiError::bad_request(
            "EXTERNAL_AGENTS_DISABLED",
            "external agents are disabled",
        ));
    }

    let user = extract_authenticated_user(&req, &state).await?;
    let provider = normalize_provider(body.provider.as_str())?;
    let outcome = normalize_outcome(body.outcome.as_str())?;
    let side = normalize_side(body.side.as_str())?;

    if body.name.trim().is_empty() {
        return Err(ApiError::bad_request("INVALID_NAME", "name is required"));
    }
    if body.cadence_seconds == 0 {
        return Err(ApiError::bad_request(
            "INVALID_CADENCE",
            "cadenceSeconds must be greater than zero",
        ));
    }
    if body.price <= 0.0 || body.price >= 1.0 {
        return Err(ApiError::bad_request(
            "INVALID_PRICE",
            "price must be between 0 and 1",
        ));
    }
    if body.quantity <= 0.0 {
        return Err(ApiError::bad_request(
            "INVALID_QUANTITY",
            "quantity must be greater than zero",
        ));
    }

    let namespaced_market_id = normalize_namespaced_market_id(provider, body.market_id.as_str());
    let parsed_market_id = ExternalMarketId::parse(namespaced_market_id.as_str())?;
    let market = external::fetch_market_by_id(&state.config, &parsed_market_id).await?;
    if !market.execution_agents {
        return Err(ApiError::bad_request(
            "MARKET_NOT_EXECUTABLE",
            "market is not executable for external agents",
        ));
    }

    let credential = load_credential(
        &state,
        user.wallet_address.as_str(),
        provider,
        body.credential_id.as_deref(),
    )
    .await?;

    let id = Uuid::new_v4().to_string();
    let row = sqlx::query(
        "INSERT INTO external_agents (
            id, owner, name, provider, market_id, provider_market_ref,
            outcome, side, price, quantity, cadence_seconds, strategy,
            credential_id, active, next_execution_at, created_at, updated_at
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,NOW(),NOW(),NOW())
        RETURNING id, owner, name, provider, market_id, outcome, side, price, quantity,
                  cadence_seconds, strategy, credential_id, active, last_executed_at,
                  next_execution_at, created_at, updated_at",
    )
    .bind(id.as_str())
    .bind(user.wallet_address.as_str())
    .bind(body.name.trim())
    .bind(provider.as_str())
    .bind(namespaced_market_id.as_str())
    .bind(market.provider_market_ref)
    .bind(outcome)
    .bind(side)
    .bind(body.price)
    .bind(body.quantity)
    .bind(body.cadence_seconds as i64)
    .bind(body.strategy.trim())
    .bind(credential.id.as_str())
    .bind(body.active.unwrap_or(true))
    .fetch_one(state.db.pool())
    .await
    .map_err(|err| ApiError::internal(&err.to_string()))?;

    Ok(HttpResponse::Ok().json(parse_external_agent(row)?))
}

pub async fn update_external_agent(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    body: web::Json<UpdateExternalAgentRequest>,
) -> Result<impl Responder, ApiError> {
    ensure_external_features_enabled(&state)?;
    let user = extract_authenticated_user(&req, &state).await?;

    let agent_id = path.into_inner();
    let current = sqlx::query(
        "SELECT id, provider, market_id, outcome, side, price, quantity, cadence_seconds, strategy, credential_id, active
         FROM external_agents
         WHERE id = $1 AND owner = $2",
    )
    .bind(agent_id.as_str())
    .bind(user.wallet_address.as_str())
    .fetch_optional(state.db.pool())
    .await
    .map_err(|err| ApiError::internal(&err.to_string()))?
    .ok_or_else(|| ApiError::not_found("External agent"))?;

    let provider_raw: String = current
        .try_get("provider")
        .map_err(|err| ApiError::internal(&err.to_string()))?;
    let provider = normalize_provider(provider_raw.as_str())?;

    let next_outcome = if let Some(outcome) = body.outcome.as_deref() {
        normalize_outcome(outcome)?
    } else {
        current
            .try_get("outcome")
            .map_err(|err| ApiError::internal(&err.to_string()))?
    };
    let next_side = if let Some(side) = body.side.as_deref() {
        normalize_side(side)?
    } else {
        current
            .try_get("side")
            .map_err(|err| ApiError::internal(&err.to_string()))?
    };
    let next_price = body
        .price
        .unwrap_or_else(|| current.try_get("price").unwrap_or(0.5));
    let next_quantity = body
        .quantity
        .unwrap_or_else(|| current.try_get("quantity").unwrap_or(0.0));
    let next_cadence = body
        .cadence_seconds
        .unwrap_or_else(|| current.try_get::<i64, _>("cadence_seconds").unwrap_or(60) as u64);
    let next_strategy = body
        .strategy
        .as_deref()
        .unwrap_or_else(|| current.try_get("strategy").unwrap_or("external"))
        .trim()
        .to_string();
    let next_name = body
        .name
        .as_deref()
        .unwrap_or("external-agent")
        .trim()
        .to_string();
    let next_active = body
        .active
        .unwrap_or_else(|| current.try_get("active").unwrap_or(true));

    let credential_id = if let Some(id) = body.credential_id.as_deref() {
        let credential =
            load_credential(&state, user.wallet_address.as_str(), provider, Some(id)).await?;
        Some(credential.id)
    } else {
        current.try_get::<String, _>("credential_id").ok()
    };

    let row = sqlx::query(
        "UPDATE external_agents
         SET name = COALESCE(NULLIF($3, ''), name),
             outcome = $4,
             side = $5,
             price = $6,
             quantity = $7,
             cadence_seconds = $8,
             strategy = $9,
             credential_id = $10,
             active = $11,
             updated_at = NOW()
         WHERE id = $1 AND owner = $2
         RETURNING id, owner, name, provider, market_id, outcome, side, price, quantity,
                   cadence_seconds, strategy, credential_id, active, last_executed_at,
                   next_execution_at, created_at, updated_at",
    )
    .bind(agent_id.as_str())
    .bind(user.wallet_address.as_str())
    .bind(next_name)
    .bind(next_outcome)
    .bind(next_side)
    .bind(next_price)
    .bind(next_quantity)
    .bind(next_cadence as i64)
    .bind(next_strategy)
    .bind(credential_id.as_deref())
    .bind(next_active)
    .fetch_one(state.db.pool())
    .await
    .map_err(|err| ApiError::internal(&err.to_string()))?;

    Ok(HttpResponse::Ok().json(parse_external_agent(row)?))
}

pub async fn execute_external_agent(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    body: web::Json<ExecuteExternalAgentRequest>,
) -> Result<impl Responder, ApiError> {
    ensure_external_features_enabled(&state)?;
    if !state.config.external_agents_enabled || !state.config.external_trading_enabled {
        return Err(ApiError::bad_request(
            "EXTERNAL_AGENT_EXECUTION_DISABLED",
            "external agent execution is disabled",
        ));
    }

    let user = extract_authenticated_user(&req, &state).await?;
    let agent_id = path.into_inner();

    let row = sqlx::query(
        "SELECT id, owner, provider, market_id, outcome, side, price, quantity,
                cadence_seconds, strategy, credential_id, active, next_execution_at
         FROM external_agents
         WHERE id = $1 AND owner = $2",
    )
    .bind(agent_id.as_str())
    .bind(user.wallet_address.as_str())
    .fetch_optional(state.db.pool())
    .await
    .map_err(|err| ApiError::internal(&err.to_string()))?
    .ok_or_else(|| ApiError::not_found("External agent"))?;

    let active: bool = row
        .try_get("active")
        .map_err(|err| ApiError::internal(&err.to_string()))?;
    if !active {
        return Err(ApiError::bad_request(
            "EXTERNAL_AGENT_INACTIVE",
            "external agent is inactive",
        ));
    }

    let next_execution_at: chrono::DateTime<Utc> = row
        .try_get("next_execution_at")
        .map_err(|err| ApiError::internal(&err.to_string()))?;
    if !body.force.unwrap_or(false) && Utc::now() < next_execution_at {
        return Err(ApiError::bad_request(
            "EXTERNAL_AGENT_COOLDOWN",
            "agent cannot execute yet",
        ));
    }

    let provider_raw: String = row
        .try_get("provider")
        .map_err(|err| ApiError::internal(&err.to_string()))?;
    let provider = normalize_provider(provider_raw.as_str())?;
    let credential_id: Option<String> = row.try_get("credential_id").ok();
    let credential = load_credential(
        &state,
        user.wallet_address.as_str(),
        provider,
        credential_id.as_deref(),
    )
    .await?;

    let signed_order = if let Some(order) = body.signed_order.clone() {
        order
    } else if let Some(default_order) = credential.payload.get("defaultSignedOrder") {
        default_order.clone()
    } else {
        return Err(ApiError::bad_request(
            "SIGNED_ORDER_REQUIRED",
            "external agent execution requires signedOrder in request or credential.defaultSignedOrder",
        ));
    };

    let submit_payload = submit_to_provider(&state, provider, &credential, &signed_order).await?;
    let provider_order_id = submit_payload
        .get("orderId")
        .or_else(|| submit_payload.get("id"))
        .or_else(|| submit_payload.get("order_id"))
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string();

    let now = Utc::now();
    let order_id = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO external_orders (
            id, owner, provider, intent_id, market_id, provider_order_id, status,
            request_payload, response_payload, error_message, created_at, updated_at
        ) VALUES ($1,$2,$3,NULL,$4,$5,'submitted',$6,$7,NULL,$8,$9)",
    )
    .bind(order_id.as_str())
    .bind(user.wallet_address.as_str())
    .bind(provider.as_str())
    .bind(
        row.try_get::<String, _>("market_id")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
    )
    .bind(provider_order_id.as_str())
    .bind(&signed_order)
    .bind(&submit_payload)
    .bind(now)
    .bind(now)
    .execute(state.db.pool())
    .await
    .map_err(|err| ApiError::internal(&err.to_string()))?;

    let cadence: i64 = row
        .try_get("cadence_seconds")
        .map_err(|err| ApiError::internal(&err.to_string()))?;
    let next = now + Duration::seconds(cadence.max(1));

    sqlx::query(
        "UPDATE external_agents
         SET last_executed_at = $2, next_execution_at = $3, updated_at = NOW()
         WHERE id = $1",
    )
    .bind(agent_id.as_str())
    .bind(now)
    .bind(next)
    .execute(state.db.pool())
    .await
    .map_err(|err| ApiError::internal(&err.to_string()))?;

    let run_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO external_agent_runs (
            id, agent_id, owner, status, intent_id, external_order_id, error_message, created_at
        ) VALUES ($1,$2,$3,'submitted',NULL,$4,NULL,NOW())",
    )
    .bind(run_id.as_str())
    .bind(agent_id.as_str())
    .bind(user.wallet_address.as_str())
    .bind(order_id.as_str())
    .execute(state.db.pool())
    .await
    .map_err(|err| ApiError::internal(&err.to_string()))?;

    Ok(HttpResponse::Ok().json(json!({
        "ok": true,
        "agentId": agent_id,
        "runId": run_id,
        "externalOrderId": order_id,
        "providerOrderId": provider_order_id,
        "nextExecutionAt": next.to_rfc3339(),
        "response": submit_payload,
    })))
}
