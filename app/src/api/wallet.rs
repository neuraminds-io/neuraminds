use actix_web::{web, HttpRequest, HttpResponse, Responder};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use super::ApiError;
use crate::models::TransactionType;
use crate::require_auth;
use crate::AppState;

fn ensure_wallet_read_mode(state: &web::Data<Arc<AppState>>) -> Result<(), ApiError> {
    if !state.config.evm_enabled || !state.config.evm_reads_enabled {
        return Err(ApiError::bad_request(
            "EVM_READ_PATH_DISABLED",
            "EVM wallet read path is disabled",
        ));
    }
    Ok(())
}

fn ensure_wallet_write_mode(state: &web::Data<Arc<AppState>>) -> Result<(), ApiError> {
    if !state.config.evm_enabled || !state.config.evm_writes_enabled {
        return Err(ApiError::bad_request(
            "EVM_WRITE_PATH_DISABLED",
            "EVM wallet write path is disabled",
        ));
    }
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct WalletBalance {
    pub available: u64,
    pub locked: u64,
    pub total: u64,
    pub pending_deposits: u64,
    pub pending_withdrawals: u64,
}

#[derive(Debug, Deserialize)]
pub struct DepositRequest {
    pub amount: u64,
    pub tx_signature: Option<String>,
    pub source: DepositSource,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum DepositSource {
    Wallet,
    Blindfold,
    Jupiter,
}

#[derive(Debug, Serialize)]
pub struct DepositResponse {
    pub transaction_id: String,
    pub status: String,
    pub amount: u64,
    pub deposit_address: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WithdrawRequest {
    pub amount: u64,
    pub destination: String,
}

#[derive(Debug, Serialize)]
pub struct WithdrawResponse {
    pub transaction_id: String,
    pub status: String,
    pub amount: u64,
    pub fee: u64,
    pub net_amount: u64,
    pub estimated_completion: String,
}

pub async fn get_balance(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
) -> Result<impl Responder, ApiError> {
    ensure_wallet_read_mode(&state)?;

    let user = require_auth!(&req, &state);
    let wallet = &user.wallet_address;

    let settled_balance = get_settled_balance(&state, wallet).await?;
    let locked_balance = get_locked_balance(&state, wallet).await?;
    let (pending_deposits, pending_withdrawals) = get_pending_amounts(&state, wallet).await?;

    let available = settled_balance.saturating_sub(locked_balance);

    Ok(HttpResponse::Ok().json(WalletBalance {
        available,
        locked: locked_balance,
        total: settled_balance,
        pending_deposits,
        pending_withdrawals,
    }))
}

pub async fn get_deposit_address(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
) -> Result<impl Responder, ApiError> {
    ensure_wallet_read_mode(&state)?;

    let user = require_auth!(&req, &state);

    let deposit_info = serde_json::json!({
        "address": state.config.program_vault_address,
        "mint": state.config.usdc_mint,
        "memo_required": false,
        "memo_format": user.wallet_address,
        "network": "base",
        "minimum_amount": 1_000_000,
    });

    Ok(HttpResponse::Ok().json(deposit_info))
}

pub async fn deposit(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
    body: web::Json<DepositRequest>,
) -> Result<impl Responder, ApiError> {
    ensure_wallet_write_mode(&state)?;

    let user = require_auth!(&req, &state);
    let wallet = user.wallet_address.to_ascii_lowercase();

    if body.amount < 1_000_000 {
        return Err(ApiError::bad_request(
            "INVALID_AMOUNT",
            "Minimum deposit is 1 USDC",
        ));
    }

    if body.amount > 1_000_000_000_000 {
        return Err(ApiError::bad_request(
            "INVALID_AMOUNT",
            "Maximum deposit is 1M USDC",
        ));
    }

    let transaction_id = Uuid::new_v4().to_string();

    match body.source {
        DepositSource::Wallet => {
            let tx_sig = body.tx_signature.as_ref().ok_or_else(|| {
                ApiError::bad_request("MISSING_FIELD", "tx_signature required for wallet deposits")
            })?;

            if !is_valid_tx_hash(tx_sig) {
                return Err(ApiError::bad_request(
                    "INVALID_SIGNATURE",
                    "tx_signature must be a valid EVM transaction hash",
                ));
            }

            record_transaction(
                &state,
                &transaction_id,
                &wallet,
                TransactionType::Deposit,
                body.amount,
                Some(tx_sig.to_ascii_lowercase()),
                "confirmed",
            )
            .await?;

            Ok(HttpResponse::Ok().json(DepositResponse {
                transaction_id,
                status: "confirmed".into(),
                amount: body.amount,
                deposit_address: None,
            }))
        }
        DepositSource::Blindfold => {
            record_transaction(
                &state,
                &transaction_id,
                &wallet,
                TransactionType::Deposit,
                body.amount,
                None,
                "pending",
            )
            .await?;

            Ok(HttpResponse::Ok().json(DepositResponse {
                transaction_id,
                status: "pending".into(),
                amount: body.amount,
                deposit_address: Some(state.config.program_vault_address.clone()),
            }))
        }
        DepositSource::Jupiter => {
            let status = if body.tx_signature.is_some() {
                "confirmed"
            } else {
                "pending"
            };
            let tx_signature = body.tx_signature.as_ref().map(|sig| sig.to_ascii_lowercase());

            record_transaction(
                &state,
                &transaction_id,
                &wallet,
                TransactionType::Deposit,
                body.amount,
                tx_signature,
                status,
            )
            .await?;

            Ok(HttpResponse::Ok().json(DepositResponse {
                transaction_id,
                status: status.into(),
                amount: body.amount,
                deposit_address: None,
            }))
        }
    }
}

pub async fn withdraw(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
    body: web::Json<WithdrawRequest>,
) -> Result<impl Responder, ApiError> {
    ensure_wallet_write_mode(&state)?;

    let user = require_auth!(&req, &state);
    let wallet = user.wallet_address.to_ascii_lowercase();

    if body.amount < 1_000_000 {
        return Err(ApiError::bad_request(
            "INVALID_AMOUNT",
            "Minimum withdrawal is 1 USDC",
        ));
    }

    let settled_balance = get_settled_balance(&state, &wallet).await?;
    let locked = get_locked_balance(&state, &wallet).await?;
    let available = settled_balance.saturating_sub(locked);

    if body.amount > available {
        return Err(ApiError::bad_request(
            "INSUFFICIENT_BALANCE",
            &format!(
                "Insufficient balance. Available: {} USDC",
                available as f64 / 1_000_000.0
            ),
        ));
    }

    if !is_valid_evm_address(&body.destination) {
        return Err(ApiError::bad_request(
            "INVALID_ADDRESS",
            "Invalid destination address",
        ));
    }

    let fee = std::cmp::max(body.amount / 1000, 100_000);
    let net_amount = body.amount - fee;

    let transaction_id = Uuid::new_v4().to_string();

    record_transaction(
        &state,
        &transaction_id,
        &wallet,
        TransactionType::Withdraw,
        body.amount,
        None,
        "pending",
    )
    .await?;

    Ok(HttpResponse::Ok().json(WithdrawResponse {
        transaction_id,
        status: "pending".into(),
        amount: body.amount,
        fee,
        net_amount,
        estimated_completion: "Pending operator settlement".into(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct BlindpayWebhook {
    pub event: String,
    pub payment_id: String,
    pub amount: u64,
    pub wallet_address: String,
    pub signature: String,
}

pub async fn blindfold_webhook(
    state: web::Data<Arc<AppState>>,
    body: web::Json<BlindpayWebhook>,
) -> Result<impl Responder, ApiError> {
    ensure_wallet_write_mode(&state)?;

    let expected_sig = compute_blindfold_signature(&body, &state.config.blindfold_webhook_secret);
    if body.signature != expected_sig {
        return Err(ApiError::unauthorized("Invalid webhook signature"));
    }

    let wallet = body.wallet_address.to_ascii_lowercase();

    match body.event.as_str() {
        "payment.completed" => {
            let tx_id = Uuid::new_v4().to_string();
            record_transaction(
                &state,
                &tx_id,
                &wallet,
                TransactionType::Deposit,
                body.amount,
                Some(body.payment_id.clone()),
                "confirmed",
            )
            .await?;
        }
        "payment.failed" => {
            update_transaction_status(&state, &body.payment_id, "failed").await?;
        }
        _ => {}
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({"received": true})))
}

async fn get_locked_balance(state: &AppState, wallet: &str) -> Result<u64, ApiError> {
    let (orders, _) = state
        .db
        .get_orders(
            wallet,
            None,
            Some(crate::models::OrderStatus::Open),
            1000,
            0,
        )
        .await
        .map_err(|e| ApiError::internal(&e.to_string()))?;

    let locked: u64 = orders
        .iter()
        .map(|o| {
            let price = o.price_bps as u64;
            let quantity = o.remaining_quantity;
            (price * quantity) / 10000
        })
        .sum();

    Ok(locked)
}

async fn get_settled_balance(state: &AppState, wallet: &str) -> Result<u64, ApiError> {
    let (txs, _) = state
        .db
        .get_transactions(wallet, None, 1000, 0)
        .await
        .map_err(|e| ApiError::internal(&e.to_string()))?;

    let mut balance: i128 = 0;

    for tx in txs.iter().filter(|tx| tx.status == "confirmed") {
        let amount = tx.amount as i128;
        match tx.tx_type {
            TransactionType::Deposit
            | TransactionType::Mint
            | TransactionType::Claim
            | TransactionType::Sell => balance += amount,
            TransactionType::Withdraw | TransactionType::Buy | TransactionType::Redeem => {
                balance -= amount
            }
        }
    }

    if balance <= 0 {
        Ok(0)
    } else {
        Ok(balance as u64)
    }
}

async fn get_pending_amounts(state: &AppState, wallet: &str) -> Result<(u64, u64), ApiError> {
    let (txs, _) = state
        .db
        .get_transactions(wallet, None, 100, 0)
        .await
        .map_err(|e| ApiError::internal(&e.to_string()))?;

    let pending_deposits: u64 = txs
        .iter()
        .filter(|t| matches!(t.tx_type, TransactionType::Deposit) && t.status == "pending")
        .map(|t| t.amount)
        .sum();

    let pending_withdrawals: u64 = txs
        .iter()
        .filter(|t| matches!(t.tx_type, TransactionType::Withdraw) && t.status == "pending")
        .map(|t| t.amount)
        .sum();

    Ok((pending_deposits, pending_withdrawals))
}

async fn record_transaction(
    state: &AppState,
    id: &str,
    owner: &str,
    tx_type: TransactionType,
    amount: u64,
    tx_signature: Option<String>,
    status: &str,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        INSERT INTO transactions (id, owner, tx_type, amount, tx_signature, status, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(id)
    .bind(owner)
    .bind(tx_type as i16)
    .bind(amount as i64)
    .bind(tx_signature)
    .bind(status)
    .bind(Utc::now())
    .execute(state.db.pool())
    .await
    .map_err(|e| ApiError::internal(&e.to_string()))?;

    Ok(())
}

async fn update_transaction_status(
    state: &AppState,
    payment_id: &str,
    status: &str,
) -> Result<(), ApiError> {
    sqlx::query("UPDATE transactions SET status = $1 WHERE tx_signature = $2")
        .bind(status)
        .bind(payment_id)
        .execute(state.db.pool())
        .await
        .map_err(|e| ApiError::internal(&e.to_string()))?;

    Ok(())
}

fn is_valid_tx_hash(tx: &str) -> bool {
    let hash = tx.strip_prefix("0x").unwrap_or(tx);
    hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit())
}

fn is_valid_evm_address(address: &str) -> bool {
    if address.len() != 42 || !address.starts_with("0x") {
        return false;
    }

    address[2..].chars().all(|c| c.is_ascii_hexdigit())
}

fn compute_blindfold_signature(webhook: &BlindpayWebhook, secret: &str) -> String {
    use sha2::{Digest, Sha256};
    let payload = format!(
        "{}{}{}{}",
        webhook.event, webhook.payment_id, webhook.amount, secret
    );
    let mut hasher = Sha256::new();
    hasher.update(payload.as_bytes());
    hex::encode(hasher.finalize())
}
