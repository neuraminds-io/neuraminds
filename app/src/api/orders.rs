use actix_web::{web, HttpRequest, HttpResponse, Responder};
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use crate::models::{
    Order, OrderSide, OrderType, OrderStatus, Outcome,
    PlaceOrderRequest, ListOrdersQuery, OrderListResponse,
    PlaceOrderResponse, CancelOrderResponse,
};
use crate::AppState;
use crate::require_auth;
use super::ApiError;

/// List orders for authenticated user
pub async fn list_orders(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
    query: web::Query<ListOrdersQuery>,
) -> Result<impl Responder, ApiError> {
    // SECURITY: Extract authenticated user from request
    let user = require_auth!(&req, &state);
    let owner = &user.wallet_address;

    let status = query.status.as_ref().map(|s| match s.as_str() {
        "open" => OrderStatus::Open,
        "filled" => OrderStatus::Filled,
        "cancelled" => OrderStatus::Cancelled,
        _ => OrderStatus::Open,
    });

    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);

    let (orders, total) = state.db
        .get_orders(owner, query.market_id.as_deref(), status, limit, offset)
        .await
        .map_err(ApiError::from)?;

    Ok(HttpResponse::Ok().json(OrderListResponse { orders, total }))
}

/// Get a single order
pub async fn get_order(
    state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> Result<impl Responder, ApiError> {
    let order_id = path.into_inner();

    let order = state.db
        .get_order(&order_id)
        .await
        .map_err(ApiError::from)?;

    match order {
        Some(o) => Ok(HttpResponse::Ok().json(o)),
        None => Err(ApiError::not_found("Order")),
    }
}

/// Place a new order
pub async fn place_order(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
    body: web::Json<PlaceOrderRequest>,
) -> Result<impl Responder, ApiError> {
    // SECURITY: Extract authenticated user from request
    let user = require_auth!(&req, &state);

    // Validate inputs
    if body.price <= 0.0 || body.price >= 1.0 {
        return Err(ApiError::bad_request(
            "INVALID_PRICE",
            "Price must be between 0 and 1",
        ));
    }
    if body.quantity == 0 {
        return Err(ApiError::bad_request(
            "INVALID_QUANTITY",
            "Quantity must be greater than 0",
        ));
    }
    // SECURITY: Validate quantity limits
    if body.quantity > 1_000_000_000 {
        return Err(ApiError::bad_request(
            "INVALID_QUANTITY",
            "Quantity exceeds maximum allowed",
        ));
    }

    let owner = user.wallet_address;

    let now = Utc::now();
    let order_id = Uuid::new_v4().to_string();
    let price_bps = (body.price * 10000.0) as u16;

    let order = Order {
        id: order_id.clone(),
        order_id: 0, // Would be assigned by on-chain program
        market_id: body.market_id.clone(),
        owner: owner.clone(),
        side: body.side,
        outcome: body.outcome,
        order_type: body.order_type,
        price: body.price,
        price_bps,
        quantity: body.quantity,
        filled_quantity: 0,
        remaining_quantity: body.quantity,
        status: OrderStatus::Open,
        is_private: body.private,
        tx_signature: None,
        created_at: now,
        updated_at: now,
        expires_at: body.expires_at,
    };

    // Add to order book and attempt matching
    let matches = state.orderbook.add_order(&order);

    // Process matches
    for matched_trade in &matches {
        // In production: submit settlement transaction
        // let tx_sig = state.solana.settle_trade(matched_trade).await?;

        // Publish trade event via Redis
        let outcome_str = match order.outcome {
            Outcome::Yes => "yes",
            Outcome::No => "no",
        };
        state.redis
            .publish_trade(
                &order.market_id,
                outcome_str,
                matched_trade.fill_price_bps as f64 / 10000.0,
                matched_trade.fill_quantity,
            )
            .await
            .ok();
    }

    // Calculate filled amount
    let total_filled: u64 = matches.iter().map(|m| m.fill_quantity).sum();
    let remaining = body.quantity.saturating_sub(total_filled);

    let final_status = if remaining == 0 {
        OrderStatus::Filled
    } else if total_filled > 0 {
        OrderStatus::PartiallyFilled
    } else {
        OrderStatus::Open
    };

    // Update order with fill info
    let mut final_order = order.clone();
    final_order.filled_quantity = total_filled;
    final_order.remaining_quantity = remaining;
    final_order.status = final_status;

    // Save to database
    state.db.create_order(&final_order).await.map_err(ApiError::from)?;

    // Publish order book update
    let outcome_str = match body.outcome {
        Outcome::Yes => "yes",
        Outcome::No => "no",
    };
    let side_str = match body.side {
        OrderSide::Buy => "bid",
        OrderSide::Sell => "ask",
    };
    state.redis
        .publish_orderbook_update(&body.market_id, outcome_str, side_str, body.price, remaining)
        .await
        .ok();

    Ok(HttpResponse::Created().json(PlaceOrderResponse {
        order_id,
        market_id: body.market_id.clone(),
        side: body.side,
        outcome: body.outcome,
        order_type: body.order_type,
        price: body.price,
        quantity: body.quantity,
        filled_quantity: total_filled,
        status: final_status,
        created_at: now,
        expires_at: body.expires_at,
        tx_signature: None,
    }))
}

/// Cancel an open order
pub async fn cancel_order(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> Result<impl Responder, ApiError> {
    // SECURITY: Extract authenticated user from request
    let user = require_auth!(&req, &state);

    let order_id = path.into_inner();

    // Get the order
    let order = state.db
        .get_order(&order_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::not_found("Order"))?;

    // SECURITY: Verify ownership - only order owner can cancel
    if order.owner != user.wallet_address {
        return Err(ApiError::forbidden("You can only cancel your own orders"));
    }

    // Check if order can be cancelled
    if order.status == OrderStatus::Filled {
        return Err(ApiError::bad_request(
            "ORDER_FILLED",
            "Cannot cancel a filled order",
        ));
    }
    if order.status == OrderStatus::Cancelled {
        return Err(ApiError::bad_request(
            "ORDER_CANCELLED",
            "Order is already cancelled",
        ));
    }

    // Remove from order book
    state.orderbook.remove_order(&order.market_id, order.outcome, order.side, &order_id);

    // Update database
    state.db
        .update_order_status(&order_id, OrderStatus::Cancelled, order.filled_quantity, 0)
        .await
        .map_err(ApiError::from)?;

    // In production: submit cancellation transaction
    // let tx_sig = state.solana.cancel_order(...).await?;

    let now = Utc::now();

    Ok(HttpResponse::Ok().json(CancelOrderResponse {
        order_id,
        status: OrderStatus::Cancelled,
        cancelled_at: now,
        tx_signature: None,
    }))
}
