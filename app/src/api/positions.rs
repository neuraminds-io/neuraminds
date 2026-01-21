use actix_web::{web, HttpRequest, HttpResponse, Responder};
use std::sync::Arc;

use crate::models::{PositionListResponse, ClaimWinningsResponse, Outcome};
use crate::AppState;
use crate::require_auth;
use super::ApiError;

/// List all positions for authenticated user
pub async fn list_positions(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
) -> Result<impl Responder, ApiError> {
    // SECURITY: Extract authenticated user from request
    let user = require_auth!(&req, &state);
    let owner = &user.wallet_address;

    let positions = state.db
        .get_positions(owner)
        .await
        .map_err(ApiError::from)?;

    Ok(HttpResponse::Ok().json(PositionListResponse { positions }))
}

/// Get position for a specific market
pub async fn get_position(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> Result<impl Responder, ApiError> {
    // SECURITY: Extract authenticated user from request
    let user = require_auth!(&req, &state);
    let owner = &user.wallet_address;

    let market_id = path.into_inner();

    let position = state.db
        .get_position(owner, &market_id)
        .await
        .map_err(ApiError::from)?;

    match position {
        Some(p) => Ok(HttpResponse::Ok().json(p)),
        None => Err(ApiError::not_found("Position")),
    }
}

/// Claim winnings for a resolved market
pub async fn claim_winnings(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> Result<impl Responder, ApiError> {
    // SECURITY: Extract authenticated user from request
    let user = require_auth!(&req, &state);
    let owner = &user.wallet_address;

    let market_id = path.into_inner();

    // Get market to verify it's resolved
    let market = state.db
        .get_market(&market_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::not_found("Market"))?;

    if market.resolved_outcome.is_none() {
        return Err(ApiError::bad_request(
            "MARKET_NOT_RESOLVED",
            "Market has not been resolved yet",
        ));
    }

    // Get position - this will only return the authenticated user's position
    let position = state.db
        .get_position(owner, &market_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::not_found("Position"))?;

    // SECURITY: Double-check ownership (defense in depth)
    if position.owner != *owner {
        return Err(ApiError::forbidden("You can only claim your own winnings"));
    }

    let winning_outcome = market.resolved_outcome.unwrap();
    let winning_tokens = match winning_outcome {
        Outcome::Yes => position.yes_balance,
        Outcome::No => position.no_balance,
    };

    if winning_tokens == 0 {
        return Err(ApiError::bad_request(
            "NO_WINNINGS",
            "No winning tokens to claim",
        ));
    }

    // In production: submit claim transaction
    // let (tx_sig, claimed_amount) = state.solana.claim_winnings(&market_pda, &owner).await?;

    let claimed_amount = winning_tokens; // 1:1 redemption

    // SECURITY: Log claim for audit trail
    log::info!(
        "Claim processed: market={}, user={}, amount={}, outcome={:?}",
        market_id, owner, claimed_amount, winning_outcome
    );

    Ok(HttpResponse::Ok().json(ClaimWinningsResponse {
        market_id,
        claimed_amount,
        winning_outcome,
        winning_tokens_burned: winning_tokens,
        tx_signature: "placeholder_signature".to_string(),
    }))
}
