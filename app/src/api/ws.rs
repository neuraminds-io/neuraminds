//! WebSocket endpoint for real-time updates

use actix_web::{web, HttpRequest, HttpResponse, Error};
use actix_web_actors::ws;
use actix::{Actor, StreamHandler, AsyncContext, ActorContext};
use std::sync::Arc;
use std::time::{Duration, Instant};
use log::{info, warn, error};

use crate::AppState;
use crate::services::websocket::SubscribeRequest;
use super::jwt::UserRole;

/// WebSocket connection timeout
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(60);

/// WebSocket session actor
pub struct WsSession {
    /// Unique session id
    id: usize,
    /// Client heartbeat
    hb: Instant,
    /// App state
    #[allow(dead_code)]
    state: Arc<AppState>,
    /// Authenticated user wallet address
    user_wallet: String,
    /// User role
    #[allow(dead_code)]
    user_role: UserRole,
    /// Subscribed markets
    subscribed_markets: Vec<String>,
}

impl WsSession {
    pub fn new(state: Arc<AppState>, user_wallet: String, user_role: UserRole) -> Self {
        Self {
            id: rand::random(),
            hb: Instant::now(),
            state,
            user_wallet,
            user_role,
            subscribed_markets: Vec::new(),
        }
    }

    /// Heartbeat to keep connection alive
    fn hb(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                warn!("WebSocket client timeout, disconnecting");
                ctx.stop();
                return;
            }
            ctx.ping(b"");
        });
    }

    /// Handle incoming text message
    fn handle_message(&mut self, text: &str, ctx: &mut <Self as Actor>::Context) {
        // Try to parse as subscription request
        if let Ok(req) = serde_json::from_str::<SubscribeRequest>(text) {
            match req.channel.as_str() {
                "orderbook" | "trades" | "market" => {
                    if let Some(market_id) = req.market_id {
                        if !self.subscribed_markets.contains(&market_id) {
                            self.subscribed_markets.push(market_id.clone());
                            info!("Client {} subscribed to market: {}", self.id, market_id);

                            // Send confirmation
                            let response = serde_json::json!({
                                "type": "subscribed",
                                "channel": req.channel,
                                "market_id": market_id
                            });
                            ctx.text(response.to_string());
                        }
                    }
                }
                "unsubscribe" => {
                    if let Some(market_id) = req.market_id {
                        self.subscribed_markets.retain(|m| m != &market_id);
                        info!("Client {} unsubscribed from market: {}", self.id, market_id);
                    }
                }
                _ => {
                    let response = serde_json::json!({
                        "type": "error",
                        "message": "Unknown channel"
                    });
                    ctx.text(response.to_string());
                }
            }
        }
    }
}

impl Actor for WsSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("WebSocket session started: {} for user: {}", self.id, self.user_wallet);
        self.hb(ctx);
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        info!("WebSocket session stopped: {}", self.id);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Text(text)) => {
                self.handle_message(&text, ctx);
            }
            Ok(ws::Message::Binary(_)) => {
                warn!("Binary messages not supported");
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            Err(e) => {
                error!("WebSocket error: {:?}", e);
                ctx.stop();
            }
            _ => {}
        }
    }
}

/// WebSocket upgrade handler
/// Requires JWT authentication via query parameter: ?token=<jwt>
pub async fn ws_handler(
    req: HttpRequest,
    stream: web::Payload,
    state: web::Data<Arc<AppState>>,
    query: web::Query<WsAuthQuery>,
) -> Result<HttpResponse, Error> {
    // Validate JWT token from query parameter
    let claims = match state.jwt.validate_token(&query.token) {
        Ok(claims) => claims,
        Err(e) => {
            warn!("WebSocket authentication failed: {:?}", e);
            return Ok(HttpResponse::Unauthorized().body("Invalid or missing token"));
        }
    };

    let session = WsSession::new(state.get_ref().clone(), claims.sub, claims.role);
    ws::start(session, &req, stream)
}

/// Query parameters for WebSocket authentication
#[derive(serde::Deserialize)]
pub struct WsAuthQuery {
    /// JWT token for authentication
    pub token: String,
}
