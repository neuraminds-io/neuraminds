use actix_web::{web, HttpRequest, HttpResponse, Responder};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::Row;
use std::collections::BTreeMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::api::auth::{extract_authenticated_user, extract_jwt_user};
use crate::api::jwt::{check_role, UserRole};
use crate::api::ApiError;
use crate::config::ExternalExecutionMode;
use crate::services::external;
use crate::services::external::credentials::{decrypt_json, encrypt_json, mask_secret};
use crate::services::external::paper::{realized_pnl, simulate_fill, unrealized_pnl};
use crate::services::external::types::{ExternalMarketId, ExternalProvider};
use crate::services::provider_rails::{evaluate_provider_access, ProviderRailAction, RailProvider};
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerTickRequest {
    pub limit: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerTickResponse {
    pub executed: bool,
    pub agents_scanned: u64,
    pub agents_executed: u64,
    pub skips_by_reason: BTreeMap<String, u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalAgentPerformanceQuery {
    pub owner: Option<String>,
    pub scope: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalAgentPerformanceResponse {
    pub scope: String,
    pub owner: Option<String>,
    pub totals: ExternalAgentPerformanceTotals,
    pub strategies: Vec<ExternalAgentStrategyPerformance>,
    pub timeline: Vec<ExternalAgentPerformancePoint>,
    pub updated_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalAgentPerformanceTotals {
    pub agents: u64,
    pub active_agents: u64,
    pub open_positions: u64,
    pub closed_positions: u64,
    pub fills: u64,
    pub volume_usdc: f64,
    pub fees_usdc: f64,
    pub realized_pnl_usdc: f64,
    pub unrealized_pnl_usdc: f64,
    pub net_pnl_usdc: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalAgentStrategyPerformance {
    pub strategy: String,
    pub agents: u64,
    pub active_agents: u64,
    pub open_positions: u64,
    pub closed_positions: u64,
    pub fills: u64,
    pub volume_usdc: f64,
    pub fees_usdc: f64,
    pub realized_pnl_usdc: f64,
    pub unrealized_pnl_usdc: f64,
    pub net_pnl_usdc: f64,
    pub win_rate: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalAgentPerformancePoint {
    pub bucket: String,
    pub volume_usdc: f64,
    pub realized_pnl_usdc: f64,
    pub unrealized_pnl_usdc: f64,
    pub net_pnl_usdc: f64,
}

#[derive(Debug, Clone)]
struct StoredCredential {
    id: String,
    payload: Value,
}

#[derive(Debug, Clone)]
struct ExternalAgentRecord {
    id: String,
    owner: String,
    name: String,
    provider: ExternalProvider,
    market_id: String,
    outcome: String,
    side: String,
    price: f64,
    quantity: f64,
    cadence_seconds: i64,
    strategy: String,
    credential_id: Option<String>,
    active: bool,
    next_execution_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct PaperPositionRecord {
    id: String,
    entry_price: f64,
    filled_quantity: f64,
    fees_paid_usdc: f64,
    hold_until: chrono::DateTime<Utc>,
    opened_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct AgentExecutionOutcome {
    executed: bool,
    skip_reason: Option<String>,
    run_status: String,
    run_id: String,
    external_order_id: Option<String>,
    provider_order_id: Option<String>,
    next_execution_at: chrono::DateTime<Utc>,
    response: Value,
}

fn normalize_provider(raw: &str) -> Result<ExternalProvider, ApiError> {
    ExternalProvider::from_str(raw).ok_or_else(|| {
        ApiError::bad_request(
            "INVALID_PROVIDER",
            "provider must be one of: limitless, polymarket",
        )
    })
}

fn to_rail_provider(provider: ExternalProvider) -> RailProvider {
    match provider {
        ExternalProvider::Limitless => RailProvider::Limitless,
        ExternalProvider::Polymarket => RailProvider::Polymarket,
    }
}

fn ensure_provider_action_allowed(
    req: &HttpRequest,
    provider: ExternalProvider,
    action: ProviderRailAction,
) -> Result<(), ApiError> {
    let rail_provider = to_rail_provider(provider);
    let decision = evaluate_provider_access(req, rail_provider, action);
    if decision.allowed {
        return Ok(());
    }

    Err(ApiError::legal_restricted(
        "REGION_PROVIDER_RESTRICTED",
        "provider unavailable in your region for this action",
        Some(json!({
            "provider": rail_provider.as_str(),
            "action": action.as_str(),
            "country": decision.country,
            "regionClass": decision.region_class.as_str(),
            "routingMode": decision.mode.as_str(),
            "legacyCloseOnly": decision.legacy_close_only,
            "safeFallbackRestriction": decision.safe_fallback_restriction,
            "detail": decision.reason
        })),
    ))
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

fn execution_mode(state: &AppState) -> ExternalExecutionMode {
    state.config.external_execution_mode
}

fn requires_live_credentials(state: &AppState) -> bool {
    execution_mode(state) == ExternalExecutionMode::Live
}

fn ensure_live_write_mode(state: &AppState) -> Result<(), ApiError> {
    if execution_mode(state).is_paper() {
        return Err(ApiError::conflict(
            "EXTERNAL_PAPER_MODE_ONLY",
            "live external venue writes are disabled while EXTERNAL_EXECUTION_MODE=paper",
        ));
    }
    Ok(())
}

fn increment_skip_reason(skips: &mut BTreeMap<String, u64>, reason: &str) {
    let entry = skips.entry(reason.to_string()).or_insert(0);
    *entry += 1;
}

fn provider_order_id_from_payload(payload: &Value) -> String {
    payload
        .get("orderId")
        .or_else(|| payload.get("id"))
        .or_else(|| payload.get("order_id"))
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string()
}

fn skip_reason_from_error(err: &ApiError) -> String {
    err.code.trim().to_ascii_lowercase()
}

fn parse_external_agent_record(
    row: sqlx::postgres::PgRow,
) -> Result<ExternalAgentRecord, ApiError> {
    let provider_raw: String = row
        .try_get("provider")
        .map_err(|err| ApiError::internal(&err.to_string()))?;

    Ok(ExternalAgentRecord {
        id: row
            .try_get("id")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
        owner: row
            .try_get("owner")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
        name: row
            .try_get("name")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
        provider: normalize_provider(provider_raw.as_str())?,
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
            .try_get("cadence_seconds")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
        strategy: row
            .try_get("strategy")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
        credential_id: row.try_get("credential_id").ok(),
        active: row
            .try_get("active")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
        next_execution_at: row
            .try_get("next_execution_at")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
    })
}

fn parse_paper_position(row: sqlx::postgres::PgRow) -> Result<PaperPositionRecord, ApiError> {
    Ok(PaperPositionRecord {
        id: row
            .try_get("id")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
        entry_price: row
            .try_get("entry_price")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
        filled_quantity: row
            .try_get("filled_quantity")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
        fees_paid_usdc: row
            .try_get("fees_paid_usdc")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
        hold_until: row
            .try_get("hold_until")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
        opened_at: row
            .try_get("opened_at")
            .map_err(|err| ApiError::internal(&err.to_string()))?,
    })
}

async fn load_external_agent_for_owner(
    state: &AppState,
    agent_id: &str,
    owner: &str,
) -> Result<ExternalAgentRecord, ApiError> {
    let row = sqlx::query(
        "SELECT id, owner, name, provider, market_id, outcome, side, price, quantity,
                cadence_seconds, strategy, credential_id, active, last_executed_at, next_execution_at
         FROM external_agents
         WHERE id = $1 AND owner = $2",
    )
    .bind(agent_id)
    .bind(owner)
    .fetch_optional(state.db.pool())
    .await
    .map_err(|err| ApiError::internal(&err.to_string()))?
    .ok_or_else(|| ApiError::not_found("External agent"))?;

    parse_external_agent_record(row)
}

async fn load_due_external_agents(
    state: &AppState,
    limit: i64,
) -> Result<Vec<ExternalAgentRecord>, ApiError> {
    let rows = sqlx::query(
        "SELECT id, owner, name, provider, market_id, outcome, side, price, quantity,
                cadence_seconds, strategy, credential_id, active, last_executed_at, next_execution_at
         FROM external_agents
         WHERE active = TRUE
         ORDER BY next_execution_at ASC, id ASC
         LIMIT $1",
    )
    .bind(limit)
    .fetch_all(state.db.pool())
    .await
    .map_err(|err| ApiError::internal(&err.to_string()))?;

    rows.into_iter().map(parse_external_agent_record).collect()
}

async fn load_open_paper_position(
    state: &AppState,
    agent_id: &str,
) -> Result<Option<PaperPositionRecord>, ApiError> {
    let row = sqlx::query(
        "SELECT id, entry_price, filled_quantity, fees_paid_usdc, hold_until, opened_at
         FROM paper_positions
         WHERE agent_id = $1 AND status = 'open'
         LIMIT 1",
    )
    .bind(agent_id)
    .fetch_optional(state.db.pool())
    .await
    .map_err(|err| ApiError::internal(&err.to_string()))?;

    row.map(parse_paper_position).transpose()
}

async fn insert_external_agent_run(
    state: &AppState,
    run_id: &str,
    agent: &ExternalAgentRecord,
    status: &str,
    external_order_id: Option<&str>,
    error_message: Option<&str>,
    metadata: &Value,
) -> Result<(), ApiError> {
    sqlx::query(
        "INSERT INTO external_agent_runs (
            id, agent_id, owner, status, intent_id, external_order_id, error_message, metadata, created_at
        ) VALUES ($1,$2,$3,$4,NULL,$5,$6,$7,NOW())",
    )
    .bind(run_id)
    .bind(agent.id.as_str())
    .bind(agent.owner.as_str())
    .bind(status)
    .bind(external_order_id)
    .bind(error_message)
    .bind(metadata)
    .execute(state.db.pool())
    .await
    .map_err(|err| ApiError::internal(&err.to_string()))?;

    Ok(())
}

async fn update_external_agent_schedule(
    state: &AppState,
    agent_id: &str,
    executed_at: chrono::DateTime<Utc>,
    next_execution_at: chrono::DateTime<Utc>,
) -> Result<(), ApiError> {
    sqlx::query(
        "UPDATE external_agents
         SET last_executed_at = $2, next_execution_at = $3, updated_at = NOW()
         WHERE id = $1",
    )
    .bind(agent_id)
    .bind(executed_at)
    .bind(next_execution_at)
    .execute(state.db.pool())
    .await
    .map_err(|err| ApiError::internal(&err.to_string()))?;

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

async fn record_paper_mark(
    state: &AppState,
    position_id: &str,
    agent: &ExternalAgentRecord,
    mark_price: f64,
    unrealized_pnl_usdc: f64,
    notional_usdc: f64,
    metadata: &Value,
) -> Result<(), ApiError> {
    sqlx::query(
        "INSERT INTO paper_marks (
            id, position_id, agent_id, owner, market_id, outcome, mark_price,
            unrealized_pnl_usdc, notional_usdc, metadata, created_at
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,NOW())",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(position_id)
    .bind(agent.id.as_str())
    .bind(agent.owner.as_str())
    .bind(agent.market_id.as_str())
    .bind(agent.outcome.as_str())
    .bind(mark_price)
    .bind(unrealized_pnl_usdc)
    .bind(notional_usdc)
    .bind(metadata)
    .execute(state.db.pool())
    .await
    .map_err(|err| ApiError::internal(&err.to_string()))?;

    Ok(())
}

async fn close_due_paper_position(
    state: &AppState,
    agent: &ExternalAgentRecord,
    position: &PaperPositionRecord,
    now: chrono::DateTime<Utc>,
    market: &external::types::ExternalMarketSnapshot,
    orderbook: &external::types::ExternalOrderBookSnapshot,
) -> Result<(bool, Value), ApiError> {
    let exit_side = if agent.side == "buy" { "sell" } else { "buy" };
    let fill = simulate_fill(
        market,
        orderbook,
        agent.outcome.as_str(),
        exit_side,
        position.filled_quantity,
        state.config.paper_fee_bps,
    );

    if fill.filled_quantity <= 0.0 {
        let unrealized = unrealized_pnl(
            agent.side.as_str(),
            position.entry_price,
            fill.mark_price,
            position.filled_quantity,
        ) - position.fees_paid_usdc;
        sqlx::query(
            "UPDATE paper_positions
             SET mark_price = $2,
                 unrealized_pnl_usdc = $3,
                 last_marked_at = $4,
                 updated_at = NOW()
             WHERE id = $1",
        )
        .bind(position.id.as_str())
        .bind(fill.mark_price)
        .bind(unrealized)
        .bind(now)
        .execute(state.db.pool())
        .await
        .map_err(|err| ApiError::internal(&err.to_string()))?;

        record_paper_mark(
            state,
            position.id.as_str(),
            agent,
            fill.mark_price,
            unrealized,
            position.filled_quantity * fill.mark_price,
            &json!({
                "reason": "no_exit_liquidity",
                "holdExpired": true
            }),
        )
        .await?;

        return Ok((
            false,
            json!({
                "status": "holding",
                "reason": "no_exit_liquidity",
                "positionId": position.id,
                "markPrice": fill.mark_price,
                "unrealizedPnlUsdc": unrealized
            }),
        ));
    }

    let original_quantity = position.filled_quantity.max(fill.filled_quantity);
    let closed_quantity = fill.filled_quantity;
    let remaining_quantity = (position.filled_quantity - closed_quantity).max(0.0);
    let allocated_open_fees = if original_quantity > 0.0 {
        position.fees_paid_usdc * (closed_quantity / original_quantity)
    } else {
        0.0
    };
    let remaining_open_fees = (position.fees_paid_usdc - allocated_open_fees).max(0.0);
    let realized = realized_pnl(
        agent.side.as_str(),
        position.entry_price,
        fill.average_price,
        closed_quantity,
        allocated_open_fees + fill.fee_usdc,
    );
    let gross = unrealized_pnl(
        agent.side.as_str(),
        position.entry_price,
        fill.average_price,
        closed_quantity,
    );

    sqlx::query(
        "INSERT INTO paper_fills (
            id, run_id, position_id, agent_id, owner, provider, market_id, outcome, side, fill_type,
            requested_quantity, filled_quantity, price, mark_price, notional_usdc, fee_usdc, metadata, created_at
        ) VALUES ($1,NULL,$2,$3,$4,$5,$6,$7,$8,'close',$9,$10,$11,$12,$13,$14,$15,NOW())",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(position.id.as_str())
    .bind(agent.id.as_str())
    .bind(agent.owner.as_str())
    .bind(agent.provider.as_str())
    .bind(agent.market_id.as_str())
    .bind(agent.outcome.as_str())
    .bind(exit_side)
    .bind(position.filled_quantity)
    .bind(closed_quantity)
    .bind(fill.average_price)
    .bind(fill.mark_price)
    .bind(fill.notional_usdc)
    .bind(fill.fee_usdc)
    .bind(json!({
        "partialFill": fill.partial_fill,
        "slippageBps": fill.slippage_bps,
        "usedOrderbookDepth": fill.used_orderbook_depth
    }))
    .execute(state.db.pool())
    .await
    .map_err(|err| ApiError::internal(&err.to_string()))?;

    sqlx::query(
        "INSERT INTO paper_outcomes (
            id, position_id, agent_id, owner, provider, market_id, outcome, side, strategy,
            entry_price, exit_price, quantity, gross_pnl_usdc, fee_usdc, realized_pnl_usdc,
            hold_seconds, metadata, created_at, closed_at
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,NOW(),$18)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(position.id.as_str())
    .bind(agent.id.as_str())
    .bind(agent.owner.as_str())
    .bind(agent.provider.as_str())
    .bind(agent.market_id.as_str())
    .bind(agent.outcome.as_str())
    .bind(agent.side.as_str())
    .bind(agent.strategy.as_str())
    .bind(position.entry_price)
    .bind(fill.average_price)
    .bind(closed_quantity)
    .bind(gross)
    .bind(allocated_open_fees + fill.fee_usdc)
    .bind(realized)
    .bind((now - position.opened_at).num_seconds().max(0))
    .bind(json!({
        "partialClose": remaining_quantity > 0.0,
        "markPrice": fill.mark_price
    }))
    .bind(now)
    .execute(state.db.pool())
    .await
    .map_err(|err| ApiError::internal(&err.to_string()))?;

    if remaining_quantity > 0.0 {
        let unrealized = unrealized_pnl(
            agent.side.as_str(),
            position.entry_price,
            fill.mark_price,
            remaining_quantity,
        ) - remaining_open_fees;
        sqlx::query(
            "UPDATE paper_positions
             SET filled_quantity = $2,
                 mark_price = $3,
                 notional_usdc = $4,
                 fees_paid_usdc = $5,
                 realized_pnl_usdc = realized_pnl_usdc + $6,
                 unrealized_pnl_usdc = $7,
                 hold_until = $8,
                 last_marked_at = $9,
                 updated_at = NOW()
             WHERE id = $1",
        )
        .bind(position.id.as_str())
        .bind(remaining_quantity)
        .bind(fill.mark_price)
        .bind(remaining_quantity * position.entry_price)
        .bind(remaining_open_fees)
        .bind(realized)
        .bind(unrealized)
        .bind(now)
        .bind(now)
        .execute(state.db.pool())
        .await
        .map_err(|err| ApiError::internal(&err.to_string()))?;

        record_paper_mark(
            state,
            position.id.as_str(),
            agent,
            fill.mark_price,
            unrealized,
            remaining_quantity * fill.mark_price,
            &json!({
                "reason": "partial_close",
                "realizedPnlUsdc": realized
            }),
        )
        .await?;

        return Ok((
            false,
            json!({
                "status": "holding",
                "reason": "partial_close",
                "positionId": position.id,
                "closedQuantity": closed_quantity,
                "remainingQuantity": remaining_quantity,
                "exitPrice": fill.average_price,
                "markPrice": fill.mark_price,
                "realizedPnlUsdc": realized
            }),
        ));
    }

    sqlx::query(
        "UPDATE paper_positions
         SET status = 'closed',
             mark_price = $2,
             fees_paid_usdc = $3,
             realized_pnl_usdc = $4,
             unrealized_pnl_usdc = 0,
             closed_at = $5,
             last_marked_at = $5,
             updated_at = NOW()
         WHERE id = $1",
    )
    .bind(position.id.as_str())
    .bind(fill.mark_price)
    .bind(position.fees_paid_usdc + fill.fee_usdc)
    .bind(realized)
    .bind(now)
    .execute(state.db.pool())
    .await
    .map_err(|err| ApiError::internal(&err.to_string()))?;

    Ok((
        true,
        json!({
            "status": "closed",
            "positionId": position.id,
            "closedQuantity": closed_quantity,
            "exitPrice": fill.average_price,
            "markPrice": fill.mark_price,
            "realizedPnlUsdc": realized
        }),
    ))
}

async fn open_paper_position(
    state: &AppState,
    agent: &ExternalAgentRecord,
    now: chrono::DateTime<Utc>,
    market: &external::types::ExternalMarketSnapshot,
    orderbook: &external::types::ExternalOrderBookSnapshot,
) -> Result<AgentExecutionOutcome, ApiError> {
    let fill = simulate_fill(
        market,
        orderbook,
        agent.outcome.as_str(),
        agent.side.as_str(),
        agent.quantity,
        state.config.paper_fee_bps,
    );

    if fill.filled_quantity <= 0.0 {
        let run_id = Uuid::new_v4().to_string();
        let next_execution_at = now + Duration::seconds(agent.cadence_seconds.max(1));
        update_external_agent_schedule(state, agent.id.as_str(), now, next_execution_at).await?;
        insert_external_agent_run(
            state,
            run_id.as_str(),
            agent,
            "paper_skipped",
            None,
            Some("no_fill_liquidity"),
            &json!({
                "mode": "paper",
                "reason": "no_fill_liquidity",
                "marketQuestion": market.question,
                "markPrice": fill.mark_price
            }),
        )
        .await?;

        return Ok(AgentExecutionOutcome {
            executed: false,
            skip_reason: Some("no_fill_liquidity".to_string()),
            run_status: "paper_skipped".to_string(),
            run_id,
            external_order_id: None,
            provider_order_id: None,
            next_execution_at,
            response: json!({
                "mode": "paper",
                "status": "skipped",
                "reason": "no_fill_liquidity",
                "markPrice": fill.mark_price
            }),
        });
    }

    let order_id = Uuid::new_v4().to_string();
    let position_id = Uuid::new_v4().to_string();
    let run_id = Uuid::new_v4().to_string();
    let hold_until = now + Duration::seconds(state.config.paper_hold_duration_seconds as i64);
    let next_execution_at = now + Duration::seconds(agent.cadence_seconds.max(1));
    let unrealized = unrealized_pnl(
        agent.side.as_str(),
        fill.average_price,
        fill.mark_price,
        fill.filled_quantity,
    ) - fill.fee_usdc;

    sqlx::query(
        "INSERT INTO external_orders (
            id, owner, provider, intent_id, market_id, provider_order_id, status,
            request_payload, response_payload, error_message, created_at, updated_at
        ) VALUES ($1,$2,$3,NULL,$4,'','paper_filled',$5,$6,NULL,$7,$7)",
    )
    .bind(order_id.as_str())
    .bind(agent.owner.as_str())
    .bind(agent.provider.as_str())
    .bind(agent.market_id.as_str())
    .bind(json!({
        "mode": "paper",
        "side": agent.side,
        "outcome": agent.outcome,
        "quantity": agent.quantity,
        "price": agent.price
    }))
    .bind(json!({
        "mode": "paper",
        "positionId": position_id,
        "fill": fill
    }))
    .bind(now)
    .execute(state.db.pool())
    .await
    .map_err(|err| ApiError::internal(&err.to_string()))?;

    sqlx::query(
        "INSERT INTO paper_positions (
            id, agent_id, owner, provider, market_id, outcome, side, strategy, status,
            entry_price, mark_price, requested_quantity, filled_quantity, notional_usdc,
            fees_paid_usdc, realized_pnl_usdc, unrealized_pnl_usdc, hold_until, opened_at,
            last_marked_at, metadata, created_at, updated_at
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,'open',$9,$10,$11,$12,$13,$14,0,$15,$16,$17,$17,$18,$17,$17)",
    )
    .bind(position_id.as_str())
    .bind(agent.id.as_str())
    .bind(agent.owner.as_str())
    .bind(agent.provider.as_str())
    .bind(agent.market_id.as_str())
    .bind(agent.outcome.as_str())
    .bind(agent.side.as_str())
    .bind(agent.strategy.as_str())
    .bind(fill.average_price)
    .bind(fill.mark_price)
    .bind(fill.requested_quantity)
    .bind(fill.filled_quantity)
    .bind(fill.notional_usdc)
    .bind(fill.fee_usdc)
    .bind(unrealized)
    .bind(hold_until)
    .bind(now)
    .bind(json!({
        "agentName": agent.name,
        "marketQuestion": market.question,
        "partialFill": fill.partial_fill
    }))
    .execute(state.db.pool())
    .await
    .map_err(|err| ApiError::internal(&err.to_string()))?;

    insert_external_agent_run(
        state,
        run_id.as_str(),
        agent,
        "paper_opened",
        Some(order_id.as_str()),
        None,
        &json!({
            "mode": "paper",
            "positionId": position_id,
            "holdUntil": hold_until.to_rfc3339(),
            "fill": fill
        }),
    )
    .await?;

    sqlx::query(
        "INSERT INTO paper_fills (
            id, run_id, position_id, agent_id, owner, provider, market_id, outcome, side, fill_type,
            requested_quantity, filled_quantity, price, mark_price, notional_usdc, fee_usdc, metadata, created_at
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,'open',$10,$11,$12,$13,$14,$15,$16,$17)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(run_id.as_str())
    .bind(position_id.as_str())
    .bind(agent.id.as_str())
    .bind(agent.owner.as_str())
    .bind(agent.provider.as_str())
    .bind(agent.market_id.as_str())
    .bind(agent.outcome.as_str())
    .bind(agent.side.as_str())
    .bind(fill.requested_quantity)
    .bind(fill.filled_quantity)
    .bind(fill.average_price)
    .bind(fill.mark_price)
    .bind(fill.notional_usdc)
    .bind(fill.fee_usdc)
    .bind(json!({
        "partialFill": fill.partial_fill,
        "slippageBps": fill.slippage_bps,
        "usedOrderbookDepth": fill.used_orderbook_depth
    }))
    .bind(now)
    .execute(state.db.pool())
    .await
    .map_err(|err| ApiError::internal(&err.to_string()))?;

    record_paper_mark(
        state,
        position_id.as_str(),
        agent,
        fill.mark_price,
        unrealized,
        fill.filled_quantity * fill.mark_price,
        &json!({
            "reason": "opened",
            "holdUntil": hold_until.to_rfc3339()
        }),
    )
    .await?;

    update_external_agent_schedule(state, agent.id.as_str(), now, next_execution_at).await?;

    Ok(AgentExecutionOutcome {
        executed: true,
        skip_reason: None,
        run_status: "paper_opened".to_string(),
        run_id,
        external_order_id: Some(order_id),
        provider_order_id: None,
        next_execution_at,
        response: json!({
            "mode": "paper",
            "status": "opened",
            "positionId": position_id,
            "fill": fill,
            "holdUntil": hold_until.to_rfc3339()
        }),
    })
}

async fn execute_paper_agent(
    state: &AppState,
    agent: &ExternalAgentRecord,
) -> Result<AgentExecutionOutcome, ApiError> {
    let now = Utc::now();
    let market_id = ExternalMarketId::parse(agent.market_id.as_str())?;
    let market = external::fetch_market_by_id(&state.config, &market_id).await?;
    let orderbook = external::fetch_orderbook(
        &state.config,
        &state.redis,
        &market_id,
        agent.outcome.as_str(),
        20,
    )
    .await?;

    if let Some(position) = load_open_paper_position(state, agent.id.as_str()).await? {
        if now < position.hold_until {
            let fill = simulate_fill(
                &market,
                &orderbook,
                agent.outcome.as_str(),
                agent.side.as_str(),
                position.filled_quantity,
                state.config.paper_fee_bps,
            );
            let unrealized = unrealized_pnl(
                agent.side.as_str(),
                position.entry_price,
                fill.mark_price,
                position.filled_quantity,
            ) - position.fees_paid_usdc;
            let next_execution_at = now + Duration::seconds(agent.cadence_seconds.max(1));

            sqlx::query(
                "UPDATE paper_positions
                 SET mark_price = $2,
                     unrealized_pnl_usdc = $3,
                     last_marked_at = $4,
                     updated_at = NOW()
                 WHERE id = $1",
            )
            .bind(position.id.as_str())
            .bind(fill.mark_price)
            .bind(unrealized)
            .bind(now)
            .execute(state.db.pool())
            .await
            .map_err(|err| ApiError::internal(&err.to_string()))?;

            record_paper_mark(
                state,
                position.id.as_str(),
                agent,
                fill.mark_price,
                unrealized,
                position.filled_quantity * fill.mark_price,
                &json!({
                    "reason": "holding_open_position",
                    "holdUntil": position.hold_until.to_rfc3339()
                }),
            )
            .await?;

            update_external_agent_schedule(state, agent.id.as_str(), now, next_execution_at)
                .await?;
            let run_id = Uuid::new_v4().to_string();
            insert_external_agent_run(
                state,
                run_id.as_str(),
                agent,
                "paper_skipped",
                None,
                Some("holding_open_position"),
                &json!({
                    "mode": "paper",
                    "positionId": position.id,
                    "markPrice": fill.mark_price,
                    "unrealizedPnlUsdc": unrealized,
                    "holdUntil": position.hold_until.to_rfc3339()
                }),
            )
            .await?;

            return Ok(AgentExecutionOutcome {
                executed: false,
                skip_reason: Some("holding_open_position".to_string()),
                run_status: "paper_skipped".to_string(),
                run_id,
                external_order_id: None,
                provider_order_id: None,
                next_execution_at,
                response: json!({
                    "mode": "paper",
                    "status": "holding",
                    "positionId": position.id,
                    "markPrice": fill.mark_price,
                    "unrealizedPnlUsdc": unrealized
                }),
            });
        }

        let (fully_closed, close_response) =
            close_due_paper_position(state, agent, &position, now, &market, &orderbook).await?;
        if !fully_closed {
            let next_execution_at = now + Duration::seconds(agent.cadence_seconds.max(1));
            update_external_agent_schedule(state, agent.id.as_str(), now, next_execution_at)
                .await?;
            let run_id = Uuid::new_v4().to_string();
            insert_external_agent_run(
                state,
                run_id.as_str(),
                agent,
                "paper_partial_close",
                None,
                Some(
                    close_response
                        .get("reason")
                        .and_then(|value| value.as_str())
                        .unwrap_or("partial_close"),
                ),
                &json!({
                    "mode": "paper",
                    "close": close_response
                }),
            )
            .await?;

            return Ok(AgentExecutionOutcome {
                executed: true,
                skip_reason: None,
                run_status: "paper_partial_close".to_string(),
                run_id,
                external_order_id: None,
                provider_order_id: None,
                next_execution_at,
                response: json!({
                    "mode": "paper",
                    "status": "partial_close",
                    "close": close_response
                }),
            });
        }
    }

    open_paper_position(state, agent, now, &market, &orderbook).await
}

async fn execute_live_agent(
    state: &AppState,
    agent: &ExternalAgentRecord,
    signed_order_override: Option<Value>,
) -> Result<AgentExecutionOutcome, ApiError> {
    let credential = load_credential(
        state,
        agent.owner.as_str(),
        agent.provider,
        agent.credential_id.as_deref(),
    )
    .await?;

    let signed_order = if let Some(order) = signed_order_override {
        order
    } else if let Some(default_order) = credential.payload.get("defaultSignedOrder") {
        default_order.clone()
    } else {
        return Err(ApiError::bad_request(
            "SIGNED_ORDER_REQUIRED",
            "external agent execution requires signedOrder in request or credential.defaultSignedOrder",
        ));
    };

    let submit_payload =
        submit_to_provider(state, agent.provider, &credential, &signed_order).await?;
    let provider_order_id = provider_order_id_from_payload(&submit_payload);
    let now = Utc::now();
    let next_execution_at = now + Duration::seconds(agent.cadence_seconds.max(1));
    let order_id = Uuid::new_v4().to_string();
    let run_id = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO external_orders (
            id, owner, provider, intent_id, market_id, provider_order_id, status,
            request_payload, response_payload, error_message, created_at, updated_at
        ) VALUES ($1,$2,$3,NULL,$4,$5,'submitted',$6,$7,NULL,$8,$8)",
    )
    .bind(order_id.as_str())
    .bind(agent.owner.as_str())
    .bind(agent.provider.as_str())
    .bind(agent.market_id.as_str())
    .bind(provider_order_id.as_str())
    .bind(&signed_order)
    .bind(&submit_payload)
    .bind(now)
    .execute(state.db.pool())
    .await
    .map_err(|err| ApiError::internal(&err.to_string()))?;

    update_external_agent_schedule(state, agent.id.as_str(), now, next_execution_at).await?;
    insert_external_agent_run(
        state,
        run_id.as_str(),
        agent,
        "submitted",
        Some(order_id.as_str()),
        None,
        &json!({
            "mode": "live",
            "providerOrderId": provider_order_id,
            "response": submit_payload
        }),
    )
    .await?;

    Ok(AgentExecutionOutcome {
        executed: true,
        skip_reason: None,
        run_status: "submitted".to_string(),
        run_id,
        external_order_id: Some(order_id),
        provider_order_id: Some(provider_order_id),
        next_execution_at,
        response: json!({
            "mode": "live",
            "response": submit_payload
        }),
    })
}

async fn execute_agent_record(
    state: &AppState,
    agent: &ExternalAgentRecord,
    signed_order_override: Option<Value>,
) -> Result<AgentExecutionOutcome, ApiError> {
    match execution_mode(state) {
        ExternalExecutionMode::Paper => execute_paper_agent(state, agent).await,
        ExternalExecutionMode::Live => {
            execute_live_agent(state, agent, signed_order_override).await
        }
    }
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
    ensure_live_write_mode(&state)?;

    let user = extract_authenticated_user(&req, &state).await?;
    let provider = normalize_provider(body.provider.as_str())?;
    ensure_provider_action_allowed(&req, provider, ProviderRailAction::TradeOpen)?;
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
    ensure_live_write_mode(&state)?;

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
    ensure_provider_action_allowed(&req, provider, ProviderRailAction::TradeOpen)?;

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
    ensure_live_write_mode(&state)?;

    let user = extract_authenticated_user(&req, &state).await?;
    let provider = normalize_provider(body.provider.as_str())?;
    ensure_provider_action_allowed(&req, provider, ProviderRailAction::TradeClose)?;
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
    ensure_provider_action_allowed(&req, provider, ProviderRailAction::TradeOpen)?;
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

    let credential_id = if requires_live_credentials(&state) || body.credential_id.is_some() {
        Some(
            load_credential(
                &state,
                user.wallet_address.as_str(),
                provider,
                body.credential_id.as_deref(),
            )
            .await?
            .id,
        )
    } else {
        None
    };

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
    .bind(credential_id.as_deref())
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
    let agent =
        load_external_agent_for_owner(&state, agent_id.as_str(), user.wallet_address.as_str())
            .await?;

    if !agent.active {
        return Err(ApiError::bad_request(
            "EXTERNAL_AGENT_INACTIVE",
            "external agent is inactive",
        ));
    }

    if !body.force.unwrap_or(false) && Utc::now() < agent.next_execution_at {
        return Err(ApiError::bad_request(
            "EXTERNAL_AGENT_COOLDOWN",
            "agent cannot execute yet",
        ));
    }

    ensure_provider_action_allowed(&req, agent.provider, ProviderRailAction::TradeOpen)?;
    let outcome = execute_agent_record(&state, &agent, body.signed_order.clone()).await?;

    Ok(HttpResponse::Ok().json(json!({
        "ok": outcome.executed,
        "mode": execution_mode(&state).as_str(),
        "agentId": agent_id,
        "runId": outcome.run_id,
        "externalOrderId": outcome.external_order_id,
        "providerOrderId": outcome.provider_order_id,
        "nextExecutionAt": outcome.next_execution_at.to_rfc3339(),
        "status": outcome.run_status,
        "skipReason": outcome.skip_reason,
        "response": outcome.response,
    })))
}

pub async fn run_external_agents_tick(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
    body: web::Json<RunnerTickRequest>,
) -> Result<impl Responder, ApiError> {
    ensure_external_features_enabled(&state)?;
    if !state.config.external_agents_enabled || !state.config.external_trading_enabled {
        return Err(ApiError::bad_request(
            "EXTERNAL_AGENT_EXECUTION_DISABLED",
            "external agent execution is disabled",
        ));
    }

    let user = extract_jwt_user(&req, &state)?;
    check_role(user.role, UserRole::Admin)?;

    let max_limit = state.config.paper_runner_scan_limit.max(1) as i64;
    let limit = body.limit.unwrap_or(max_limit).clamp(1, max_limit);
    let now = Utc::now();
    let agents = load_due_external_agents(&state, limit).await?;
    let mut agents_executed = 0_u64;
    let mut skips_by_reason = BTreeMap::new();

    for agent in &agents {
        if now < agent.next_execution_at {
            increment_skip_reason(&mut skips_by_reason, "not_due");
            continue;
        }

        if let Err(err) =
            ensure_provider_action_allowed(&req, agent.provider, ProviderRailAction::TradeOpen)
        {
            let reason = skip_reason_from_error(&err);
            increment_skip_reason(&mut skips_by_reason, reason.as_str());
            let run_id = Uuid::new_v4().to_string();
            insert_external_agent_run(
                &state,
                run_id.as_str(),
                agent,
                "paper_skipped",
                None,
                Some(reason.as_str()),
                &json!({
                    "mode": execution_mode(&state).as_str(),
                    "error": {
                        "code": err.code,
                        "message": err.message,
                        "details": err.details
                    }
                }),
            )
            .await?;
            continue;
        }

        match execute_agent_record(&state, agent, None).await {
            Ok(outcome) => {
                if outcome.executed {
                    agents_executed += 1;
                } else if let Some(reason) = outcome.skip_reason.as_deref() {
                    increment_skip_reason(&mut skips_by_reason, reason);
                }
            }
            Err(err) => {
                let reason = skip_reason_from_error(&err);
                log::error!(
                    "external runner failed agent_id={} provider={} market_id={} strategy={} side={} outcome={} code={} message={} details={}",
                    agent.id,
                    agent.provider.as_str(),
                    agent.market_id,
                    agent.strategy,
                    agent.side,
                    agent.outcome,
                    err.code,
                    err.message,
                    err.details
                        .as_ref()
                        .map(Value::to_string)
                        .unwrap_or_else(|| "null".to_string()),
                );
                increment_skip_reason(&mut skips_by_reason, reason.as_str());
                let run_id = Uuid::new_v4().to_string();
                insert_external_agent_run(
                    &state,
                    run_id.as_str(),
                    agent,
                    "failed",
                    None,
                    Some(reason.as_str()),
                    &json!({
                        "mode": execution_mode(&state).as_str(),
                        "error": {
                            "code": err.code,
                            "message": err.message,
                            "details": err.details
                        }
                    }),
                )
                .await?;
            }
        }
    }

    Ok(HttpResponse::Ok().json(RunnerTickResponse {
        executed: agents_executed > 0,
        agents_scanned: agents.len() as u64,
        agents_executed,
        skips_by_reason,
    }))
}

pub async fn get_external_agents_performance(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
    query: web::Query<ExternalAgentPerformanceQuery>,
) -> Result<impl Responder, ApiError> {
    ensure_external_features_enabled(&state)?;
    let user = extract_jwt_user(&req, &state)?;
    let is_admin = matches!(user.role, UserRole::Admin);
    let requested_owner = query
        .owner
        .as_ref()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty());
    let scope = query
        .scope
        .as_deref()
        .unwrap_or(if is_admin { "all" } else { "self" })
        .trim()
        .to_ascii_lowercase();

    let owner_filter = match scope.as_str() {
        "all" if is_admin => None,
        "owner" if is_admin => requested_owner.clone(),
        _ => Some(requested_owner.unwrap_or_else(|| user.wallet_address.to_ascii_lowercase())),
    };

    if !is_admin && query.owner.is_some() {
        let requested = query
            .owner
            .as_ref()
            .map(|value| value.trim().to_ascii_lowercase())
            .unwrap_or_default();
        if requested != user.wallet_address.to_ascii_lowercase() {
            return Err(ApiError::forbidden("Insufficient permissions"));
        }
    }

    let agents_row = if let Some(owner) = owner_filter.as_ref() {
        sqlx::query(
            "SELECT COUNT(*) AS agents,
                    COUNT(*) FILTER (WHERE active) AS active_agents
             FROM external_agents
             WHERE owner = $1",
        )
        .bind(owner.as_str())
        .fetch_one(state.db.pool())
        .await
    } else {
        sqlx::query(
            "SELECT COUNT(*) AS agents,
                    COUNT(*) FILTER (WHERE active) AS active_agents
             FROM external_agents",
        )
        .fetch_one(state.db.pool())
        .await
    }
    .map_err(|err| ApiError::internal(&err.to_string()))?;

    let positions_row = if let Some(owner) = owner_filter.as_ref() {
        sqlx::query(
            "SELECT COUNT(*) FILTER (WHERE status = 'open') AS open_positions,
                    COUNT(*) FILTER (WHERE status = 'closed') AS closed_positions,
                    COALESCE(SUM(CASE WHEN status = 'open' THEN unrealized_pnl_usdc ELSE 0 END), 0) AS unrealized_pnl_usdc
             FROM paper_positions
             WHERE owner = $1",
        )
        .bind(owner.as_str())
        .fetch_one(state.db.pool())
        .await
    } else {
        sqlx::query(
            "SELECT COUNT(*) FILTER (WHERE status = 'open') AS open_positions,
                    COUNT(*) FILTER (WHERE status = 'closed') AS closed_positions,
                    COALESCE(SUM(CASE WHEN status = 'open' THEN unrealized_pnl_usdc ELSE 0 END), 0) AS unrealized_pnl_usdc
             FROM paper_positions",
        )
        .fetch_one(state.db.pool())
        .await
    }
    .map_err(|err| ApiError::internal(&err.to_string()))?;

    let fills_row = if let Some(owner) = owner_filter.as_ref() {
        sqlx::query(
            "SELECT COUNT(*) AS fills,
                    COALESCE(SUM(notional_usdc), 0) AS volume_usdc,
                    COALESCE(SUM(fee_usdc), 0) AS fees_usdc
             FROM paper_fills
             WHERE owner = $1",
        )
        .bind(owner.as_str())
        .fetch_one(state.db.pool())
        .await
    } else {
        sqlx::query(
            "SELECT COUNT(*) AS fills,
                    COALESCE(SUM(notional_usdc), 0) AS volume_usdc,
                    COALESCE(SUM(fee_usdc), 0) AS fees_usdc
             FROM paper_fills",
        )
        .fetch_one(state.db.pool())
        .await
    }
    .map_err(|err| ApiError::internal(&err.to_string()))?;

    let outcomes_row = if let Some(owner) = owner_filter.as_ref() {
        sqlx::query(
            "SELECT COALESCE(SUM(realized_pnl_usdc), 0) AS realized_pnl_usdc
             FROM paper_outcomes
             WHERE owner = $1",
        )
        .bind(owner.as_str())
        .fetch_one(state.db.pool())
        .await
    } else {
        sqlx::query(
            "SELECT COALESCE(SUM(realized_pnl_usdc), 0) AS realized_pnl_usdc
             FROM paper_outcomes",
        )
        .fetch_one(state.db.pool())
        .await
    }
    .map_err(|err| ApiError::internal(&err.to_string()))?;

    let agents = agents_row.try_get::<i64, _>("agents").unwrap_or(0).max(0) as u64;
    let active_agents = agents_row
        .try_get::<i64, _>("active_agents")
        .unwrap_or(0)
        .max(0) as u64;
    let open_positions = positions_row
        .try_get::<i64, _>("open_positions")
        .unwrap_or(0)
        .max(0) as u64;
    let closed_positions = positions_row
        .try_get::<i64, _>("closed_positions")
        .unwrap_or(0)
        .max(0) as u64;
    let fills = fills_row.try_get::<i64, _>("fills").unwrap_or(0).max(0) as u64;
    let volume_usdc = fills_row.try_get::<f64, _>("volume_usdc").unwrap_or(0.0);
    let fees_usdc = fills_row.try_get::<f64, _>("fees_usdc").unwrap_or(0.0);
    let realized_pnl_usdc = outcomes_row
        .try_get::<f64, _>("realized_pnl_usdc")
        .unwrap_or(0.0);
    let unrealized_pnl_usdc = positions_row
        .try_get::<f64, _>("unrealized_pnl_usdc")
        .unwrap_or(0.0);

    let strategy_rows = if let Some(owner) = owner_filter.as_ref() {
        sqlx::query(
            "SELECT strategy,
                    COUNT(*) AS agents,
                    COUNT(*) FILTER (WHERE active) AS active_agents
             FROM external_agents
             WHERE owner = $1
             GROUP BY strategy
             ORDER BY strategy ASC",
        )
        .bind(owner.as_str())
        .fetch_all(state.db.pool())
        .await
    } else {
        sqlx::query(
            "SELECT strategy,
                    COUNT(*) AS agents,
                    COUNT(*) FILTER (WHERE active) AS active_agents
             FROM external_agents
             GROUP BY strategy
             ORDER BY strategy ASC",
        )
        .fetch_all(state.db.pool())
        .await
    }
    .map_err(|err| ApiError::internal(&err.to_string()))?;

    let position_strategy_rows = if let Some(owner) = owner_filter.as_ref() {
        sqlx::query(
            "SELECT strategy,
                    COUNT(*) FILTER (WHERE status = 'open') AS open_positions,
                    COUNT(*) FILTER (WHERE status = 'closed') AS closed_positions,
                    COALESCE(SUM(CASE WHEN status = 'open' THEN unrealized_pnl_usdc ELSE 0 END), 0) AS unrealized_pnl_usdc
             FROM paper_positions
             WHERE owner = $1
             GROUP BY strategy",
        )
        .bind(owner.as_str())
        .fetch_all(state.db.pool())
        .await
    } else {
        sqlx::query(
            "SELECT strategy,
                    COUNT(*) FILTER (WHERE status = 'open') AS open_positions,
                    COUNT(*) FILTER (WHERE status = 'closed') AS closed_positions,
                    COALESCE(SUM(CASE WHEN status = 'open' THEN unrealized_pnl_usdc ELSE 0 END), 0) AS unrealized_pnl_usdc
             FROM paper_positions
             GROUP BY strategy",
        )
        .fetch_all(state.db.pool())
        .await
    }
    .map_err(|err| ApiError::internal(&err.to_string()))?;

    let fill_strategy_rows = if let Some(owner) = owner_filter.as_ref() {
        sqlx::query(
            "SELECT strategy,
                    COUNT(*) AS fills,
                    COALESCE(SUM(notional_usdc), 0) AS volume_usdc,
                    COALESCE(SUM(fee_usdc), 0) AS fees_usdc
             FROM (
                SELECT pf.*, ea.strategy
                FROM paper_fills pf
                JOIN external_agents ea ON ea.id = pf.agent_id
                WHERE pf.owner = $1
             ) AS scoped
             GROUP BY strategy",
        )
        .bind(owner.as_str())
        .fetch_all(state.db.pool())
        .await
    } else {
        sqlx::query(
            "SELECT strategy,
                    COUNT(*) AS fills,
                    COALESCE(SUM(notional_usdc), 0) AS volume_usdc,
                    COALESCE(SUM(fee_usdc), 0) AS fees_usdc
             FROM (
                SELECT pf.*, ea.strategy
                FROM paper_fills pf
                JOIN external_agents ea ON ea.id = pf.agent_id
             ) AS scoped
             GROUP BY strategy",
        )
        .fetch_all(state.db.pool())
        .await
    }
    .map_err(|err| ApiError::internal(&err.to_string()))?;

    let outcome_strategy_rows = if let Some(owner) = owner_filter.as_ref() {
        sqlx::query(
            "SELECT strategy,
                    COALESCE(SUM(realized_pnl_usdc), 0) AS realized_pnl_usdc,
                    COALESCE(AVG(CASE WHEN realized_pnl_usdc > 0 THEN 1.0 ELSE 0.0 END), 0) AS win_rate
             FROM paper_outcomes
             WHERE owner = $1
             GROUP BY strategy",
        )
        .bind(owner.as_str())
        .fetch_all(state.db.pool())
        .await
    } else {
        sqlx::query(
            "SELECT strategy,
                    COALESCE(SUM(realized_pnl_usdc), 0) AS realized_pnl_usdc,
                    COALESCE(AVG(CASE WHEN realized_pnl_usdc > 0 THEN 1.0 ELSE 0.0 END), 0) AS win_rate
             FROM paper_outcomes
             GROUP BY strategy",
        )
        .fetch_all(state.db.pool())
        .await
    }
    .map_err(|err| ApiError::internal(&err.to_string()))?;

    let mut strategy_map = BTreeMap::new();
    for row in strategy_rows {
        let strategy = row
            .try_get::<String, _>("strategy")
            .unwrap_or_else(|_| "unclassified".to_string());
        strategy_map.insert(
            strategy.clone(),
            ExternalAgentStrategyPerformance {
                strategy,
                agents: row.try_get::<i64, _>("agents").unwrap_or(0).max(0) as u64,
                active_agents: row.try_get::<i64, _>("active_agents").unwrap_or(0).max(0) as u64,
                open_positions: 0,
                closed_positions: 0,
                fills: 0,
                volume_usdc: 0.0,
                fees_usdc: 0.0,
                realized_pnl_usdc: 0.0,
                unrealized_pnl_usdc: 0.0,
                net_pnl_usdc: 0.0,
                win_rate: 0.0,
            },
        );
    }

    for row in position_strategy_rows {
        let strategy = row
            .try_get::<String, _>("strategy")
            .unwrap_or_else(|_| "unclassified".to_string());
        let entry =
            strategy_map
                .entry(strategy.clone())
                .or_insert(ExternalAgentStrategyPerformance {
                    strategy,
                    agents: 0,
                    active_agents: 0,
                    open_positions: 0,
                    closed_positions: 0,
                    fills: 0,
                    volume_usdc: 0.0,
                    fees_usdc: 0.0,
                    realized_pnl_usdc: 0.0,
                    unrealized_pnl_usdc: 0.0,
                    net_pnl_usdc: 0.0,
                    win_rate: 0.0,
                });
        entry.open_positions = row.try_get::<i64, _>("open_positions").unwrap_or(0).max(0) as u64;
        entry.closed_positions = row
            .try_get::<i64, _>("closed_positions")
            .unwrap_or(0)
            .max(0) as u64;
        entry.unrealized_pnl_usdc = row.try_get::<f64, _>("unrealized_pnl_usdc").unwrap_or(0.0);
    }

    for row in fill_strategy_rows {
        let strategy = row
            .try_get::<String, _>("strategy")
            .unwrap_or_else(|_| "unclassified".to_string());
        let entry =
            strategy_map
                .entry(strategy.clone())
                .or_insert(ExternalAgentStrategyPerformance {
                    strategy,
                    agents: 0,
                    active_agents: 0,
                    open_positions: 0,
                    closed_positions: 0,
                    fills: 0,
                    volume_usdc: 0.0,
                    fees_usdc: 0.0,
                    realized_pnl_usdc: 0.0,
                    unrealized_pnl_usdc: 0.0,
                    net_pnl_usdc: 0.0,
                    win_rate: 0.0,
                });
        entry.fills = row.try_get::<i64, _>("fills").unwrap_or(0).max(0) as u64;
        entry.volume_usdc = row.try_get::<f64, _>("volume_usdc").unwrap_or(0.0);
        entry.fees_usdc = row.try_get::<f64, _>("fees_usdc").unwrap_or(0.0);
    }

    for row in outcome_strategy_rows {
        let strategy = row
            .try_get::<String, _>("strategy")
            .unwrap_or_else(|_| "unclassified".to_string());
        let entry =
            strategy_map
                .entry(strategy.clone())
                .or_insert(ExternalAgentStrategyPerformance {
                    strategy,
                    agents: 0,
                    active_agents: 0,
                    open_positions: 0,
                    closed_positions: 0,
                    fills: 0,
                    volume_usdc: 0.0,
                    fees_usdc: 0.0,
                    realized_pnl_usdc: 0.0,
                    unrealized_pnl_usdc: 0.0,
                    net_pnl_usdc: 0.0,
                    win_rate: 0.0,
                });
        entry.realized_pnl_usdc = row.try_get::<f64, _>("realized_pnl_usdc").unwrap_or(0.0);
        entry.win_rate = row.try_get::<f64, _>("win_rate").unwrap_or(0.0);
    }

    let mut strategies = strategy_map.into_values().collect::<Vec<_>>();
    for entry in &mut strategies {
        entry.net_pnl_usdc = entry.realized_pnl_usdc + entry.unrealized_pnl_usdc;
    }

    let volume_timeline_rows = if let Some(owner) = owner_filter.as_ref() {
        sqlx::query(
            "SELECT date_trunc('hour', created_at) AS bucket,
                    COALESCE(SUM(notional_usdc), 0) AS volume_usdc
             FROM paper_fills
             WHERE owner = $1
               AND created_at >= NOW() - INTERVAL '24 hours'
             GROUP BY bucket
             ORDER BY bucket ASC",
        )
        .bind(owner.as_str())
        .fetch_all(state.db.pool())
        .await
    } else {
        sqlx::query(
            "SELECT date_trunc('hour', created_at) AS bucket,
                    COALESCE(SUM(notional_usdc), 0) AS volume_usdc
             FROM paper_fills
             WHERE created_at >= NOW() - INTERVAL '24 hours'
             GROUP BY bucket
             ORDER BY bucket ASC",
        )
        .fetch_all(state.db.pool())
        .await
    }
    .map_err(|err| ApiError::internal(&err.to_string()))?;

    let realized_timeline_rows = if let Some(owner) = owner_filter.as_ref() {
        sqlx::query(
            "SELECT date_trunc('hour', closed_at) AS bucket,
                    COALESCE(SUM(realized_pnl_usdc), 0) AS realized_pnl_usdc
             FROM paper_outcomes
             WHERE owner = $1
               AND closed_at >= NOW() - INTERVAL '24 hours'
             GROUP BY bucket
             ORDER BY bucket ASC",
        )
        .bind(owner.as_str())
        .fetch_all(state.db.pool())
        .await
    } else {
        sqlx::query(
            "SELECT date_trunc('hour', closed_at) AS bucket,
                    COALESCE(SUM(realized_pnl_usdc), 0) AS realized_pnl_usdc
             FROM paper_outcomes
             WHERE closed_at >= NOW() - INTERVAL '24 hours'
             GROUP BY bucket
             ORDER BY bucket ASC",
        )
        .fetch_all(state.db.pool())
        .await
    }
    .map_err(|err| ApiError::internal(&err.to_string()))?;

    let unrealized_timeline_rows = if let Some(owner) = owner_filter.as_ref() {
        sqlx::query(
            "SELECT bucket, COALESCE(SUM(unrealized_pnl_usdc), 0) AS unrealized_pnl_usdc
             FROM (
                 SELECT DISTINCT ON (position_id, bucket)
                        position_id,
                        date_trunc('hour', created_at) AS bucket,
                        unrealized_pnl_usdc
                 FROM paper_marks
                 WHERE owner = $1
                   AND created_at >= NOW() - INTERVAL '24 hours'
                 ORDER BY position_id, bucket, created_at DESC
             ) AS scoped
             GROUP BY bucket
             ORDER BY bucket ASC",
        )
        .bind(owner.as_str())
        .fetch_all(state.db.pool())
        .await
    } else {
        sqlx::query(
            "SELECT bucket, COALESCE(SUM(unrealized_pnl_usdc), 0) AS unrealized_pnl_usdc
             FROM (
                 SELECT DISTINCT ON (position_id, bucket)
                        position_id,
                        date_trunc('hour', created_at) AS bucket,
                        unrealized_pnl_usdc
                 FROM paper_marks
                 WHERE created_at >= NOW() - INTERVAL '24 hours'
                 ORDER BY position_id, bucket, created_at DESC
             ) AS scoped
             GROUP BY bucket
             ORDER BY bucket ASC",
        )
        .fetch_all(state.db.pool())
        .await
    }
    .map_err(|err| ApiError::internal(&err.to_string()))?;

    let mut timeline_map: BTreeMap<String, ExternalAgentPerformancePoint> = BTreeMap::new();
    for row in volume_timeline_rows {
        let bucket: chrono::DateTime<Utc> = row
            .try_get("bucket")
            .map_err(|err| ApiError::internal(&err.to_string()))?;
        let key = bucket.to_rfc3339();
        timeline_map.insert(
            key.clone(),
            ExternalAgentPerformancePoint {
                bucket: key,
                volume_usdc: row.try_get::<f64, _>("volume_usdc").unwrap_or(0.0),
                realized_pnl_usdc: 0.0,
                unrealized_pnl_usdc: 0.0,
                net_pnl_usdc: 0.0,
            },
        );
    }

    for row in realized_timeline_rows {
        let bucket: chrono::DateTime<Utc> = row
            .try_get("bucket")
            .map_err(|err| ApiError::internal(&err.to_string()))?;
        let key = bucket.to_rfc3339();
        let entry = timeline_map
            .entry(key.clone())
            .or_insert(ExternalAgentPerformancePoint {
                bucket: key,
                volume_usdc: 0.0,
                realized_pnl_usdc: 0.0,
                unrealized_pnl_usdc: 0.0,
                net_pnl_usdc: 0.0,
            });
        entry.realized_pnl_usdc = row.try_get::<f64, _>("realized_pnl_usdc").unwrap_or(0.0);
    }

    for row in unrealized_timeline_rows {
        let bucket: chrono::DateTime<Utc> = row
            .try_get("bucket")
            .map_err(|err| ApiError::internal(&err.to_string()))?;
        let key = bucket.to_rfc3339();
        let entry = timeline_map
            .entry(key.clone())
            .or_insert(ExternalAgentPerformancePoint {
                bucket: key,
                volume_usdc: 0.0,
                realized_pnl_usdc: 0.0,
                unrealized_pnl_usdc: 0.0,
                net_pnl_usdc: 0.0,
            });
        entry.unrealized_pnl_usdc = row.try_get::<f64, _>("unrealized_pnl_usdc").unwrap_or(0.0);
    }

    let mut cumulative_realized = 0.0;
    let mut timeline = timeline_map.into_values().collect::<Vec<_>>();
    for point in &mut timeline {
        cumulative_realized += point.realized_pnl_usdc;
        point.realized_pnl_usdc = cumulative_realized;
        point.net_pnl_usdc = point.realized_pnl_usdc + point.unrealized_pnl_usdc;
    }

    Ok(HttpResponse::Ok().json(ExternalAgentPerformanceResponse {
        scope: if owner_filter.is_none() {
            "all".to_string()
        } else {
            "owner".to_string()
        },
        owner: owner_filter,
        totals: ExternalAgentPerformanceTotals {
            agents,
            active_agents,
            open_positions,
            closed_positions,
            fills,
            volume_usdc,
            fees_usdc,
            realized_pnl_usdc,
            unrealized_pnl_usdc,
            net_pnl_usdc: realized_pnl_usdc + unrealized_pnl_usdc,
        },
        strategies,
        timeline,
        updated_at: Utc::now().to_rfc3339(),
    }))
}
