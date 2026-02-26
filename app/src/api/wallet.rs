use actix_web::{web, HttpRequest, HttpResponse, Responder};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use super::ApiError;
use crate::models::TransactionType;
use crate::require_auth;
use crate::AppState;

fn ensure_legacy_wallet_mode(state: &web::Data<Arc<AppState>>) -> Result<(), ApiError> {
    if !state.config.legacy_reads_enabled {
        return Err(ApiError::bad_request(
            "LEGACY_READ_PATH_DISABLED",
            "Legacy wallet read path is disabled",
        ));
    }
    Ok(())
}

fn ensure_legacy_wallet_write_mode(state: &web::Data<Arc<AppState>>) -> Result<(), ApiError> {
    if !state.config.legacy_writes_enabled {
        return Err(ApiError::bad_request(
            "LEGACY_WRITE_PATH_DISABLED",
            "Legacy wallet write path is disabled",
        ));
    }
    Ok(())
}

/// User wallet balance
#[derive(Debug, Serialize)]
pub struct WalletBalance {
    /// Available USDC balance (in smallest units, 6 decimals)
    pub available: u64,
    /// Locked in open orders
    pub locked: u64,
    /// Total balance
    pub total: u64,
    /// Pending deposits
    pub pending_deposits: u64,
    /// Pending withdrawals
    pub pending_withdrawals: u64,
}

/// Deposit request
#[derive(Debug, Deserialize)]
pub struct DepositRequest {
    /// Amount in USDC (6 decimals)
    pub amount: u64,
    /// Solana transaction signature (for on-chain deposits)
    pub tx_signature: Option<String>,
    /// Source type
    pub source: DepositSource,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum DepositSource {
    /// Direct USDC transfer from wallet
    Wallet,
    /// Blindfold Finance card payment
    Blindfold,
    /// Jupiter swap from another token
    Jupiter,
}

/// Deposit response
#[derive(Debug, Serialize)]
pub struct DepositResponse {
    pub transaction_id: String,
    pub status: String,
    pub amount: u64,
    pub deposit_address: Option<String>,
}

/// Withdrawal request
#[derive(Debug, Deserialize)]
pub struct WithdrawRequest {
    /// Amount in USDC (6 decimals)
    pub amount: u64,
    /// Destination wallet address
    pub destination: String,
}

/// Withdrawal response
#[derive(Debug, Serialize)]
pub struct WithdrawResponse {
    pub transaction_id: String,
    pub status: String,
    pub amount: u64,
    pub fee: u64,
    pub net_amount: u64,
    pub estimated_completion: String,
}

/// Get wallet balance
pub async fn get_balance(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
) -> Result<impl Responder, ApiError> {
    ensure_legacy_wallet_mode(&state)?;

    let user = require_auth!(&req, &state);
    let wallet = &user.wallet_address;

    // Get on-chain balance from program
    let on_chain_balance = state.solana.get_user_balance(wallet).await.unwrap_or(0);

    // Get locked balance from open orders
    let locked_balance = get_locked_balance(&state, wallet).await?;

    // Get pending transactions
    let (pending_deposits, pending_withdrawals) = get_pending_amounts(&state, wallet).await?;

    let balance = WalletBalance {
        available: on_chain_balance.saturating_sub(locked_balance),
        locked: locked_balance,
        total: on_chain_balance,
        pending_deposits,
        pending_withdrawals,
    };

    Ok(HttpResponse::Ok().json(balance))
}

/// Get deposit address for USDC transfers
pub async fn get_deposit_address(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
) -> Result<impl Responder, ApiError> {
    ensure_legacy_wallet_mode(&state)?;

    let user = require_auth!(&req, &state);

    // For Solana, the deposit address is the program vault PDA
    // Users send USDC to this address with a memo containing their wallet
    let deposit_info = serde_json::json!({
        "address": state.config.program_vault_address,
        "mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC mainnet
        "memo_required": true,
        "memo_format": user.wallet_address,
        "network": "solana",
        "minimum_amount": 1_000_000, // 1 USDC
    });

    Ok(HttpResponse::Ok().json(deposit_info))
}

/// Initiate deposit
pub async fn deposit(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
    body: web::Json<DepositRequest>,
) -> Result<impl Responder, ApiError> {
    ensure_legacy_wallet_write_mode(&state)?;

    let user = require_auth!(&req, &state);
    let wallet = &user.wallet_address;

    // Validate amount
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
            // For direct wallet deposits, verify the transaction signature
            let tx_sig = body.tx_signature.as_ref().ok_or_else(|| {
                ApiError::bad_request("MISSING_FIELD", "tx_signature required for wallet deposits")
            })?;

            // Verify transaction on-chain
            let verified = state
                .solana
                .verify_deposit_transaction(tx_sig, wallet, body.amount)
                .await
                .map_err(|e| {
                    ApiError::bad_request(
                        "VERIFICATION_FAILED",
                        &format!("Transaction verification failed: {}", e),
                    )
                })?;

            if !verified {
                return Err(ApiError::bad_request(
                    "VERIFICATION_FAILED",
                    "Transaction verification failed",
                ));
            }

            // Record deposit
            record_transaction(
                &state,
                &transaction_id,
                wallet,
                TransactionType::Deposit,
                body.amount,
                Some(tx_sig.clone()),
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
            // For Blindfold, we create a pending deposit and return session info
            record_transaction(
                &state,
                &transaction_id,
                wallet,
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
            // For Jupiter swaps, the swap widget handles the transaction
            // We just record the expected deposit
            record_transaction(
                &state,
                &transaction_id,
                wallet,
                TransactionType::Deposit,
                body.amount,
                body.tx_signature.clone(),
                if body.tx_signature.is_some() {
                    "confirmed"
                } else {
                    "pending"
                },
            )
            .await?;

            Ok(HttpResponse::Ok().json(DepositResponse {
                transaction_id,
                status: if body.tx_signature.is_some() {
                    "confirmed".into()
                } else {
                    "pending".into()
                },
                amount: body.amount,
                deposit_address: None,
            }))
        }
    }
}

/// Initiate withdrawal
pub async fn withdraw(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
    body: web::Json<WithdrawRequest>,
) -> Result<impl Responder, ApiError> {
    ensure_legacy_wallet_write_mode(&state)?;

    let user = require_auth!(&req, &state);
    let wallet = &user.wallet_address;

    // Validate amount
    if body.amount < 1_000_000 {
        return Err(ApiError::bad_request(
            "INVALID_AMOUNT",
            "Minimum withdrawal is 1 USDC",
        ));
    }

    // Check balance
    let balance = state.solana.get_user_balance(wallet).await.unwrap_or(0);

    let locked = get_locked_balance(&state, wallet).await?;
    let available = balance.saturating_sub(locked);

    if body.amount > available {
        return Err(ApiError::bad_request(
            "INSUFFICIENT_BALANCE",
            &format!(
                "Insufficient balance. Available: {} USDC",
                available as f64 / 1_000_000.0
            ),
        ));
    }

    // Validate destination address
    if !is_valid_solana_address(&body.destination) {
        return Err(ApiError::bad_request(
            "INVALID_ADDRESS",
            "Invalid destination address",
        ));
    }

    // Calculate fee (0.1% with 0.1 USDC minimum)
    let fee = std::cmp::max(body.amount / 1000, 100_000);
    let net_amount = body.amount - fee;

    let transaction_id = Uuid::new_v4().to_string();

    // Execute withdrawal on-chain
    let tx_signature = state
        .solana
        .execute_withdrawal(wallet, &body.destination, net_amount)
        .await
        .map_err(|e| ApiError::internal(&format!("Withdrawal failed: {}", e)))?;

    // Record withdrawal
    record_transaction(
        &state,
        &transaction_id,
        wallet,
        TransactionType::Withdraw,
        body.amount,
        Some(tx_signature.clone()),
        "confirmed",
    )
    .await?;

    Ok(HttpResponse::Ok().json(WithdrawResponse {
        transaction_id,
        status: "confirmed".into(),
        amount: body.amount,
        fee,
        net_amount,
        estimated_completion: "Immediate".into(),
    }))
}

/// Blindfold webhook handler
#[derive(Debug, Deserialize)]
pub struct BlindpayWebhook {
    pub event: String,
    pub payment_id: String,
    pub amount: u64,
    pub currency: String,
    pub wallet_address: String,
    pub status: String,
    pub signature: String,
}

pub async fn blindfold_webhook(
    state: web::Data<Arc<AppState>>,
    body: web::Json<BlindpayWebhook>,
) -> Result<impl Responder, ApiError> {
    ensure_legacy_wallet_write_mode(&state)?;

    // Verify webhook signature
    let expected_sig = compute_blindfold_signature(&body, &state.config.blindfold_webhook_secret);
    if body.signature != expected_sig {
        return Err(ApiError::unauthorized("Invalid webhook signature"));
    }

    match body.event.as_str() {
        "payment.completed" => {
            // Credit user's account
            let tx_id = Uuid::new_v4().to_string();
            record_transaction(
                &state,
                &tx_id,
                &body.wallet_address,
                TransactionType::Deposit,
                body.amount,
                Some(body.payment_id.clone()),
                "confirmed",
            )
            .await?;

            // Execute on-chain deposit
            state
                .solana
                .credit_user_balance(&body.wallet_address, body.amount)
                .await
                .map_err(|e| ApiError::internal(&format!("Failed to credit balance: {}", e)))?;
        }
        "payment.failed" => {
            // Update transaction status
            update_transaction_status(&state, &body.payment_id, "failed").await?;
        }
        _ => {}
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({"received": true})))
}

// Helper functions

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
            // Calculate collateral locked per order
            let price = o.price_bps as u64;
            let quantity = o.remaining_quantity;
            (price * quantity) / 10000
        })
        .sum();

    Ok(locked)
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

fn is_valid_solana_address(address: &str) -> bool {
    // Basic validation: 32-44 characters, base58
    if address.len() < 32 || address.len() > 44 {
        return false;
    }
    address
        .chars()
        .all(|c| c.is_ascii_alphanumeric() && c != '0' && c != 'O' && c != 'I' && c != 'l')
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
