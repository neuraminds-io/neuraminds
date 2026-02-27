use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::api::ApiError;
use crate::services::x402::{self, build_quote, X402PaymentProof, X402Resource};
use crate::services::xmtp_swarm::{self, SwarmListQuery, SwarmSendRequest};
use crate::AppState;

fn infer_api_base_url(state: &AppState) -> String {
    if let Ok(public_api_url) = std::env::var("PUBLIC_API_URL") {
        let value = public_api_url.trim().trim_end_matches('/');
        if !value.is_empty() {
            return value.to_string();
        }
    }

    if let Some(origin) = state
        .config
        .cors_origins
        .iter()
        .find(|origin| origin.starts_with("http://") || origin.starts_with("https://"))
    {
        return format!("{}/v1", origin.trim_end_matches('/'));
    }

    if state.config.is_development || state.config.siwe_domain.contains("localhost") {
        return format!("http://{}:{}/v1", state.config.host, state.config.port);
    }

    format!("https://{}/v1", state.config.siwe_domain)
}

fn internal_api_base_url(state: &AppState) -> String {
    let host = if state.config.host == "0.0.0.0" {
        "127.0.0.1"
    } else {
        state.config.host.as_str()
    };
    format!("http://{}:{}/v1", host, state.config.port)
}

#[derive(Debug, Deserialize)]
struct McpJsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct McpToolCallParams {
    name: String,
    #[serde(default)]
    arguments: Value,
}

#[derive(Debug, Deserialize)]
struct McpResourceReadParams {
    uri: String,
}

#[derive(Debug, Deserialize)]
struct McpPromptGetParams {
    name: String,
    #[serde(default)]
    arguments: Value,
}

#[derive(Debug, Serialize)]
struct McpToolContent {
    #[serde(rename = "type")]
    kind: &'static str,
    text: String,
}

fn mcp_response_result(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

fn mcp_response_error(id: Value, code: i64, message: &str, data: Option<Value>) -> Value {
    let mut payload = json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    });

    if let Some(extra) = data {
        payload["error"]["data"] = extra;
    }
    payload
}

fn tool_result_payload(payload: Value, is_error: bool) -> Value {
    let pretty = serde_json::to_string_pretty(&payload).unwrap_or_else(|_| payload.to_string());
    json!({
        "content": [McpToolContent {
            kind: "text",
            text: pretty,
        }],
        "structuredContent": payload,
        "isError": is_error
    })
}

fn mcp_tools() -> Vec<Value> {
    vec![
        json!({
            "name": "getMarkets",
            "description": "List Base markets with pagination.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "minimum": 1, "maximum": 200 },
                    "offset": { "type": "integer", "minimum": 0 },
                    "payment": { "type": "object" }
                }
            }
        }),
        json!({
            "name": "getOrderBook",
            "description": "Fetch order book for a market side (x402 payment required when enabled).",
            "inputSchema": {
                "type": "object",
                "required": ["market_id", "outcome"],
                "properties": {
                    "market_id": { "type": "integer", "minimum": 1 },
                    "outcome": { "type": "string", "enum": ["yes", "no"] },
                    "depth": { "type": "integer", "minimum": 1, "maximum": 100 },
                    "payment": { "type": "object" }
                }
            }
        }),
        json!({
            "name": "getTrades",
            "description": "Fetch recent market trades (x402 payment required when enabled).",
            "inputSchema": {
                "type": "object",
                "required": ["market_id"],
                "properties": {
                    "market_id": { "type": "integer", "minimum": 1 },
                    "outcome": { "type": "string", "enum": ["yes", "no"] },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 200 },
                    "offset": { "type": "integer", "minimum": 0 },
                    "payment": { "type": "object" }
                }
            }
        }),
        json!({
            "name": "getAgents",
            "description": "List active or historical autonomous agents in AgentRuntime.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "minimum": 1, "maximum": 200 },
                    "offset": { "type": "integer", "minimum": 0 },
                    "owner": { "type": "string" },
                    "market_id": { "type": "integer", "minimum": 1 },
                    "active": { "type": "boolean" },
                    "payment": { "type": "object" }
                }
            }
        }),
        json!({
            "name": "prepareCreateAgentTx",
            "description": "Prepare calldata for createAgent wallet execution.",
            "inputSchema": {
                "type": "object",
                "required": ["marketId", "isYes", "priceBps", "size", "cadence", "expiryWindow", "strategy"],
                "properties": {
                    "from": { "type": "string" },
                    "marketId": { "type": "integer", "minimum": 1 },
                    "isYes": { "type": "boolean" },
                    "priceBps": { "type": "integer", "minimum": 1, "maximum": 9999 },
                    "size": { "type": "string" },
                    "cadence": { "type": "integer", "minimum": 1 },
                    "expiryWindow": { "type": "integer", "minimum": 1 },
                    "strategy": { "type": "string", "minLength": 1 },
                    "payment": { "type": "object" }
                }
            }
        }),
        json!({
            "name": "prepareExecuteAgentTx",
            "description": "Prepare calldata for executeAgent wallet execution.",
            "inputSchema": {
                "type": "object",
                "required": ["agentId"],
                "properties": {
                    "from": { "type": "string" },
                    "agentId": { "type": "integer", "minimum": 1 },
                    "payment": { "type": "object" }
                }
            }
        }),
        json!({
            "name": "getX402Quote",
            "description": "Get x402 quote for premium resources.",
            "inputSchema": {
                "type": "object",
                "required": ["resource"],
                "properties": {
                    "resource": { "type": "string", "enum": ["orderbook", "trades", "mcp_tool_call"] }
                }
            }
        }),
        json!({
            "name": "sendSwarmMessage",
            "description": "Send signed XMTP swarm message.",
            "inputSchema": {
                "type": "object",
                "required": ["swarm_id", "sender", "message", "signature"],
                "properties": {
                    "swarm_id": { "type": "string" },
                    "sender": { "type": "string" },
                    "message": { "type": "string" },
                    "signature": { "type": "string" },
                    "metadata": { "type": "object" },
                    "payment": { "type": "object" }
                }
            }
        }),
        json!({
            "name": "listSwarmMessages",
            "description": "List recent XMTP swarm messages.",
            "inputSchema": {
                "type": "object",
                "required": ["swarm_id"],
                "properties": {
                    "swarm_id": { "type": "string" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 200 },
                    "offset": { "type": "integer", "minimum": 0 },
                    "payment": { "type": "object" }
                }
            }
        }),
    ]
}

fn mcp_resources(api_base: &str) -> Vec<Value> {
    vec![
        json!({
            "uri": "neuraminds://markets/live",
            "name": "Live markets",
            "description": "Current market list from MarketCore."
        }),
        json!({
            "uri": "neuraminds://agents/active",
            "name": "Active agents",
            "description": "Active AgentRuntime entries with execution readiness."
        }),
        json!({
            "uri": "neuraminds://xmtp/health",
            "name": "XMTP swarm health",
            "description": "XMTP swarm runtime configuration and limits."
        }),
        json!({
            "uri": format!("{}/web4/capabilities", api_base),
            "name": "Web4 capabilities",
            "description": "Protocol feature status."
        }),
    ]
}

fn mcp_prompts() -> Vec<Value> {
    vec![
        json!({
            "name": "market-analysis",
            "description": "Analyze market structure, liquidity and executable opportunities.",
            "arguments": [
                { "name": "market_id", "description": "Target market id", "required": true }
            ]
        }),
        json!({
            "name": "agent-launch",
            "description": "Generate agent launch params from risk budget and target outcome.",
            "arguments": [
                { "name": "market_id", "description": "Target market id", "required": true },
                { "name": "outcome", "description": "yes or no", "required": true },
                { "name": "budget_usdc", "description": "Budget in USDC", "required": true }
            ]
        }),
        json!({
            "name": "swarm-coordination",
            "description": "Coordinate an XMTP swarm plan for executing market agents.",
            "arguments": [
                { "name": "swarm_id", "description": "Swarm channel id", "required": true },
                { "name": "objective", "description": "Mission objective", "required": true }
            ]
        }),
    ]
}

fn append_query(path: &str, key: &str, value: impl ToString) -> String {
    if path.contains('?') {
        format!("{path}&{key}={}", value.to_string())
    } else {
        format!("{path}?{key}={}", value.to_string())
    }
}

fn parse_payment_arg(args: &Value) -> Result<Option<X402PaymentProof>, ApiError> {
    let Some(payment) = args.get("payment") else {
        return Ok(None);
    };
    let parsed = serde_json::from_value::<X402PaymentProof>(payment.clone()).map_err(|_| {
        ApiError::bad_request(
            "INVALID_X402_PAYMENT_OBJECT",
            "payment must include resource, amount_microusdc, nonce, expires_at, tx_hash, signature",
        )
    })?;
    Ok(Some(parsed))
}

async fn call_internal_api(
    state: &AppState,
    method: reqwest::Method,
    path: &str,
    body: Option<Value>,
    payment: Option<&X402PaymentProof>,
) -> Result<(u16, Value), ApiError> {
    let base = internal_api_base_url(state);
    let url = format!(
        "{}/{}",
        base.trim_end_matches('/'),
        path.trim_start_matches('/')
    );
    let client = reqwest::Client::new();
    let mut request = client.request(method, url);
    if let Some(payload) = body {
        request = request.json(&payload);
    }
    if let Some(proof) = payment {
        request = request.header("x-payment", proof.to_header_value());
    }

    let response = request
        .send()
        .await
        .map_err(|_| ApiError::internal("Failed to call internal API for MCP dispatch"))?;
    let status = response.status().as_u16();
    let payload = response
        .json::<Value>()
        .await
        .unwrap_or_else(|_| json!({ "ok": status < 400 }));
    Ok((status, payload))
}

async fn handle_tool_call(state: &AppState, params: McpToolCallParams) -> Result<Value, ApiError> {
    let mut args = params.arguments;
    if args.is_null() {
        args = json!({});
    }

    match params.name.as_str() {
        "getMarkets" => {
            let mut path = "/evm/markets".to_string();
            if let Some(limit) = args.get("limit").and_then(|v| v.as_u64()) {
                path = append_query(path.as_str(), "limit", limit);
            }
            if let Some(offset) = args.get("offset").and_then(|v| v.as_u64()) {
                path = append_query(path.as_str(), "offset", offset);
            }
            let (status, payload) =
                call_internal_api(state, reqwest::Method::GET, path.as_str(), None, None).await?;
            if status >= 400 {
                return Ok(tool_result_payload(
                    json!({ "status": status, "error": payload }),
                    true,
                ));
            }
            Ok(tool_result_payload(payload, false))
        }
        "getOrderBook" => {
            let market_id = args
                .get("market_id")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| ApiError::bad_request("INVALID_ARGS", "market_id is required"))?;
            let outcome = args
                .get("outcome")
                .and_then(|v| v.as_str())
                .unwrap_or("yes");
            let mut path = format!("/evm/markets/{market_id}/orderbook?outcome={outcome}");
            if let Some(depth) = args.get("depth").and_then(|v| v.as_u64()) {
                path = append_query(path.as_str(), "depth", depth);
            }

            let payment = parse_payment_arg(&args)?;
            if state.config.x402_enabled && payment.is_none() {
                return Ok(tool_result_payload(
                    json!({
                        "status": 402,
                        "error": "x402 payment required",
                        "quote": build_quote(state, X402Resource::OrderBook)
                    }),
                    true,
                ));
            }
            let (status, payload) = call_internal_api(
                state,
                reqwest::Method::GET,
                path.as_str(),
                None,
                payment.as_ref(),
            )
            .await?;
            if status >= 400 {
                return Ok(tool_result_payload(
                    json!({ "status": status, "error": payload }),
                    true,
                ));
            }
            Ok(tool_result_payload(payload, false))
        }
        "getTrades" => {
            let market_id = args
                .get("market_id")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| ApiError::bad_request("INVALID_ARGS", "market_id is required"))?;
            let mut path = format!("/evm/markets/{market_id}/trades");
            if let Some(outcome) = args.get("outcome").and_then(|v| v.as_str()) {
                path = append_query(path.as_str(), "outcome", outcome);
            }
            if let Some(limit) = args.get("limit").and_then(|v| v.as_u64()) {
                path = append_query(path.as_str(), "limit", limit);
            }
            if let Some(offset) = args.get("offset").and_then(|v| v.as_u64()) {
                path = append_query(path.as_str(), "offset", offset);
            }

            let payment = parse_payment_arg(&args)?;
            if state.config.x402_enabled && payment.is_none() {
                return Ok(tool_result_payload(
                    json!({
                        "status": 402,
                        "error": "x402 payment required",
                        "quote": build_quote(state, X402Resource::Trades)
                    }),
                    true,
                ));
            }
            let (status, payload) = call_internal_api(
                state,
                reqwest::Method::GET,
                path.as_str(),
                None,
                payment.as_ref(),
            )
            .await?;
            if status >= 400 {
                return Ok(tool_result_payload(
                    json!({ "status": status, "error": payload }),
                    true,
                ));
            }
            Ok(tool_result_payload(payload, false))
        }
        "getAgents" => {
            let mut path = "/evm/agents".to_string();
            if let Some(limit) = args.get("limit").and_then(|v| v.as_u64()) {
                path = append_query(path.as_str(), "limit", limit);
            }
            if let Some(offset) = args.get("offset").and_then(|v| v.as_u64()) {
                path = append_query(path.as_str(), "offset", offset);
            }
            if let Some(owner) = args.get("owner").and_then(|v| v.as_str()) {
                path = append_query(path.as_str(), "owner", owner);
            }
            if let Some(market_id) = args.get("market_id").and_then(|v| v.as_u64()) {
                path = append_query(path.as_str(), "market_id", market_id);
            }
            if let Some(active) = args.get("active").and_then(|v| v.as_bool()) {
                path = append_query(path.as_str(), "active", active);
            }
            let (status, payload) =
                call_internal_api(state, reqwest::Method::GET, path.as_str(), None, None).await?;
            if status >= 400 {
                return Ok(tool_result_payload(
                    json!({ "status": status, "error": payload }),
                    true,
                ));
            }
            Ok(tool_result_payload(payload, false))
        }
        "prepareCreateAgentTx" => {
            let (status, payload) = call_internal_api(
                state,
                reqwest::Method::POST,
                "/evm/write/agents/create",
                Some(args),
                None,
            )
            .await?;
            if status >= 400 {
                return Ok(tool_result_payload(
                    json!({ "status": status, "error": payload }),
                    true,
                ));
            }
            Ok(tool_result_payload(payload, false))
        }
        "prepareExecuteAgentTx" => {
            let (status, payload) = call_internal_api(
                state,
                reqwest::Method::POST,
                "/evm/write/agents/execute",
                Some(args),
                None,
            )
            .await?;
            if status >= 400 {
                return Ok(tool_result_payload(
                    json!({ "status": status, "error": payload }),
                    true,
                ));
            }
            Ok(tool_result_payload(payload, false))
        }
        "getX402Quote" => {
            let resource = match args.get("resource").and_then(|v| v.as_str()) {
                Some("orderbook") => X402Resource::OrderBook,
                Some("trades") => X402Resource::Trades,
                Some("mcp_tool_call") => X402Resource::McpToolCall,
                _ => {
                    return Ok(tool_result_payload(
                        json!({
                            "status": 400,
                            "error": "resource must be one of: orderbook, trades, mcp_tool_call"
                        }),
                        true,
                    ))
                }
            };
            Ok(tool_result_payload(
                json!(build_quote(state, resource)),
                false,
            ))
        }
        "sendSwarmMessage" => {
            let payload: SwarmSendRequest = serde_json::from_value(args).map_err(|_| {
                ApiError::bad_request("INVALID_SWARM_MESSAGE", "swarm message payload is invalid")
            })?;
            match xmtp_swarm::send_message(state, payload).await {
                Ok(envelope) => Ok(tool_result_payload(json!(envelope), false)),
                Err(err) => Ok(tool_result_payload(
                    json!({
                        "status": err.status,
                        "code": err.code,
                        "error": err.message
                    }),
                    true,
                )),
            }
        }
        "listSwarmMessages" => {
            let swarm_id = args
                .get("swarm_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ApiError::bad_request("INVALID_ARGS", "swarm_id is required"))?;
            let query = SwarmListQuery {
                limit: args.get("limit").and_then(|v| v.as_u64()),
                offset: args.get("offset").and_then(|v| v.as_u64()),
            };
            match xmtp_swarm::list_messages(state, swarm_id, query).await {
                Ok(data) => Ok(tool_result_payload(json!(data), false)),
                Err(err) => Ok(tool_result_payload(
                    json!({
                        "status": err.status,
                        "code": err.code,
                        "error": err.message
                    }),
                    true,
                )),
            }
        }
        _ => Ok(tool_result_payload(
            json!({
                "status": 404,
                "error": format!("Unknown tool: {}", params.name)
            }),
            true,
        )),
    }
}

async fn handle_mcp_method(
    state: &AppState,
    request: &McpJsonRpcRequest,
) -> Result<Value, ApiError> {
    let id = request.id.clone().unwrap_or(Value::Null);

    match request.method.as_str() {
        "initialize" => Ok(mcp_response_result(
            id,
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": { "listChanged": false },
                    "resources": { "subscribe": false, "listChanged": false },
                    "prompts": { "listChanged": false }
                },
                "serverInfo": {
                    "name": "neuraminds-mcp",
                    "version": "1.0.0"
                }
            }),
        )),
        "ping" => Ok(mcp_response_result(id, json!({ "ok": true }))),
        "tools/list" => Ok(mcp_response_result(id, json!({ "tools": mcp_tools() }))),
        "tools/call" => {
            let params: McpToolCallParams =
                serde_json::from_value(request.params.clone().ok_or_else(|| {
                    ApiError::bad_request("INVALID_PARAMS", "tools/call requires params")
                })?)
                .map_err(|_| {
                    ApiError::bad_request("INVALID_PARAMS", "tools/call params are invalid")
                })?;

            if state.config.x402_enabled
                && params.name != "getOrderBook"
                && params.name != "getTrades"
                && params.name != "getX402Quote"
            {
                let payment = parse_payment_arg(&params.arguments)?;
                let Some(proof) = payment else {
                    return Ok(mcp_response_result(
                        id,
                        tool_result_payload(
                            json!({
                                "status": 402,
                                "error": "x402 payment required",
                                "quote": build_quote(state, X402Resource::McpToolCall)
                            }),
                            true,
                        ),
                    ));
                };
                x402::ensure_payment_from_proof(state, &proof, X402Resource::McpToolCall).await?;
            }

            let result = handle_tool_call(state, params).await?;
            Ok(mcp_response_result(id, result))
        }
        "resources/list" => {
            let api_base = infer_api_base_url(state);
            Ok(mcp_response_result(
                id,
                json!({
                    "resources": mcp_resources(api_base.as_str())
                }),
            ))
        }
        "resources/read" => {
            let params: McpResourceReadParams =
                serde_json::from_value(request.params.clone().ok_or_else(|| {
                    ApiError::bad_request("INVALID_PARAMS", "resources/read requires params")
                })?)
                .map_err(|_| {
                    ApiError::bad_request("INVALID_PARAMS", "resources/read params are invalid")
                })?;

            let resource_payload = match params.uri.as_str() {
                "neuraminds://markets/live" => {
                    let (_, payload) = call_internal_api(
                        state,
                        reqwest::Method::GET,
                        "/evm/markets?limit=50",
                        None,
                        None,
                    )
                    .await?;
                    payload
                }
                "neuraminds://agents/active" => {
                    let (_, payload) = call_internal_api(
                        state,
                        reqwest::Method::GET,
                        "/evm/agents?active=true&limit=50",
                        None,
                        None,
                    )
                    .await?;
                    payload
                }
                "neuraminds://xmtp/health" => xmtp_swarm::health(state),
                _ if params.uri.starts_with("http://") || params.uri.starts_with("https://") => {
                    let url = reqwest::Url::parse(params.uri.as_str()).map_err(|_| {
                        ApiError::bad_request("INVALID_RESOURCE_URI", "resource uri is invalid")
                    })?;
                    let relative = format!(
                        "{}{}",
                        url.path(),
                        url.query().map(|v| format!("?{v}")).unwrap_or_default()
                    );
                    let (_, payload) = call_internal_api(
                        state,
                        reqwest::Method::GET,
                        relative.as_str(),
                        None,
                        None,
                    )
                    .await?;
                    payload
                }
                _ => {
                    return Ok(mcp_response_error(
                        id,
                        -32602,
                        "Unknown resource uri",
                        Some(json!({ "uri": params.uri })),
                    ))
                }
            };

            Ok(mcp_response_result(
                id,
                json!({
                    "contents": [
                        {
                            "uri": params.uri,
                            "mimeType": "application/json",
                            "text": serde_json::to_string_pretty(&resource_payload).unwrap_or_else(|_| resource_payload.to_string())
                        }
                    ]
                }),
            ))
        }
        "prompts/list" => Ok(mcp_response_result(id, json!({ "prompts": mcp_prompts() }))),
        "prompts/get" => {
            let params: McpPromptGetParams =
                serde_json::from_value(request.params.clone().ok_or_else(|| {
                    ApiError::bad_request("INVALID_PARAMS", "prompts/get requires params")
                })?)
                .map_err(|_| {
                    ApiError::bad_request("INVALID_PARAMS", "prompts/get params are invalid")
                })?;

            let prompt_text = match params.name.as_str() {
                "market-analysis" => {
                    let market_id = params
                        .arguments
                        .get("market_id")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    format!("Analyze market {market_id} using order book depth, recent trades, and agent execution windows. Return: thesis, confidence (0-100), risk factors, and execution plan.")
                }
                "agent-launch" => {
                    let market_id = params
                        .arguments
                        .get("market_id")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    let outcome = params
                        .arguments
                        .get("outcome")
                        .and_then(|v| v.as_str())
                        .unwrap_or("yes");
                    let budget = params
                        .arguments
                        .get("budget_usdc")
                        .and_then(|v| v.as_str())
                        .unwrap_or("0");
                    format!("Given market {market_id}, target side {outcome}, and budget {budget} USDC, propose createAgent params: priceBps, size, cadence, expiryWindow, and strategy rationale.")
                }
                "swarm-coordination" => {
                    let swarm_id = params
                        .arguments
                        .get("swarm_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("default");
                    let objective = params
                        .arguments
                        .get("objective")
                        .and_then(|v| v.as_str())
                        .unwrap_or("execute market ops");
                    format!("Draft XMTP swarm message plan for swarm {swarm_id} to achieve objective: {objective}. Include role assignments, deadlines, and success criteria.")
                }
                _ => {
                    return Ok(mcp_response_error(
                        id,
                        -32602,
                        "Unknown prompt name",
                        Some(json!({ "name": params.name })),
                    ))
                }
            };

            Ok(mcp_response_result(
                id,
                json!({
                    "description": "Generated prompt",
                    "messages": [
                        {
                            "role": "user",
                            "content": {
                                "type": "text",
                                "text": prompt_text
                            }
                        }
                    ]
                }),
            ))
        }
        "notifications/initialized" => Ok(mcp_response_result(id, json!({ "ok": true }))),
        _ => Ok(mcp_response_error(
            id,
            -32601,
            "Method not found",
            Some(json!({ "method": request.method })),
        )),
    }
}

pub async fn handle_mcp_jsonrpc(
    state: web::Data<Arc<AppState>>,
    body: web::Json<Value>,
) -> impl Responder {
    let payload = body.into_inner();
    let mut responses = Vec::new();

    let requests: Vec<Value> = if payload.is_array() {
        payload.as_array().cloned().unwrap_or_default()
    } else {
        vec![payload]
    };

    if requests.is_empty() {
        return HttpResponse::BadRequest().json(mcp_response_error(
            Value::Null,
            -32600,
            "Invalid Request",
            Some(json!({ "reason": "empty batch" })),
        ));
    }

    for raw in requests {
        let parsed = serde_json::from_value::<McpJsonRpcRequest>(raw.clone());
        let request = match parsed {
            Ok(req) => req,
            Err(_) => {
                responses.push(mcp_response_error(
                    Value::Null,
                    -32600,
                    "Invalid Request",
                    Some(json!({ "payload": raw })),
                ));
                continue;
            }
        };

        if request.jsonrpc != "2.0" {
            responses.push(mcp_response_error(
                request.id.unwrap_or(Value::Null),
                -32600,
                "Invalid Request: jsonrpc must be '2.0'",
                None,
            ));
            continue;
        }

        if request.id.is_none() {
            let _ = handle_mcp_method(&state, &request).await;
            continue;
        }

        match handle_mcp_method(&state, &request).await {
            Ok(response) => responses.push(response),
            Err(err) => responses.push(mcp_response_error(
                request.id.unwrap_or(Value::Null),
                -32000,
                &err.message,
                Some(json!({ "code": err.code, "status": err.status })),
            )),
        }
    }

    if responses.is_empty() {
        HttpResponse::NoContent().finish()
    } else if responses.len() == 1 {
        HttpResponse::Ok().json(responses.remove(0))
    } else {
        HttpResponse::Ok().json(responses)
    }
}

pub async fn get_web4_capabilities(state: web::Data<Arc<AppState>>) -> impl Responder {
    let api_base = infer_api_base_url(&state);

    HttpResponse::Ok().json(json!({
        "project": "neuraminds",
        "mode": "web4-agent-market-network",
        "chain": {
            "name": "base",
            "id": state.config.base_chain_id
        },
        "api_base": api_base,
        "runtime": {
            "market_core_configured": !state.config.market_core_address.trim().is_empty(),
            "order_book_configured": !state.config.order_book_address.trim().is_empty(),
            "agent_runtime_configured": !state.config.agent_runtime_address.trim().is_empty(),
            "evm_reads_enabled": state.config.evm_reads_enabled,
            "evm_writes_enabled": state.config.evm_writes_enabled
        },
        "protocols": [
            {
                "id": "agents-md",
                "status": "implemented",
                "description": "Machine-readable project interface at repository root."
            },
            {
                "id": "mcp-jsonrpc-server",
                "status": "implemented",
                "description": "MCP JSON-RPC server with tools/resources/prompts.",
                "endpoint": "/v1/web4/mcp"
            },
            {
                "id": "a2a-agent-card",
                "status": "implemented",
                "description": "Cross-agent discovery card for external orchestration systems.",
                "endpoint": "/v1/web4/agent-card"
            },
            {
                "id": "erc-8004-identity",
                "status": "implemented",
                "description": "Onchain agent identity and reputation registries integrated into agent snapshots."
            },
            {
                "id": "x402-agent-payments",
                "status": "implemented",
                "description": "x402 payment verification for premium orderbook/trade reads and MCP premium calls."
            },
            {
                "id": "xmtp-swarm",
                "status": "implemented",
                "description": "Signed swarm messaging surfaces exposed under /v1/web4/xmtp/*."
            }
        ]
    }))
}

pub async fn get_mcp_manifest(state: web::Data<Arc<AppState>>) -> impl Responder {
    let api_base = infer_api_base_url(&state);

    HttpResponse::Ok().json(json!({
        "name": "neuraminds",
        "version": "1.0.0",
        "description": "MCP JSON-RPC server for NeuraMinds Base agent market network.",
        "transport": {
            "type": "http+jsonrpc",
            "endpoint": format!("{}/web4/mcp", api_base)
        },
        "jsonrpc": {
            "version": "2.0",
            "supported_methods": [
                "initialize",
                "ping",
                "tools/list",
                "tools/call",
                "resources/list",
                "resources/read",
                "prompts/list",
                "prompts/get"
            ]
        },
        "tools": mcp_tools(),
        "resources": mcp_resources(api_base.as_str()),
        "prompts": mcp_prompts()
    }))
}

pub async fn get_agent_card(state: web::Data<Arc<AppState>>) -> impl Responder {
    let api_base = infer_api_base_url(&state);

    HttpResponse::Ok().json(json!({
        "schema": "a2a-agent-card/v1",
        "name": "NeuraMinds Base Agent Network",
        "description": "Agent-executable prediction market infrastructure on Base.",
        "network": {
            "chain": "base",
            "chain_id": state.config.base_chain_id
        },
        "auth": {
            "type": "siwe+jwt",
            "nonce_endpoint": format!("{}/auth/siwe/nonce", api_base),
            "login_endpoint": format!("{}/auth/siwe/login", api_base)
        },
        "capabilities": [
            {
                "id": "market-data",
                "description": "Query markets, orderbooks and fills from Base contracts."
            },
            {
                "id": "agent-runtime",
                "description": "Discover, launch and execute autonomous market agents."
            },
            {
                "id": "mcp-jsonrpc",
                "description": "MCP tools/resources/prompts via JSON-RPC."
            },
            {
                "id": "x402-payments",
                "description": "x402 payment-gated premium data routes."
            },
            {
                "id": "xmtp-swarm",
                "description": "Signed swarm coordination channels."
            }
        ],
        "actions": [
            {
                "name": "mcp_jsonrpc",
                "method": "POST",
                "url": format!("{}/web4/mcp", api_base)
            },
            {
                "name": "list_markets",
                "method": "GET",
                "url": format!("{}/evm/markets", api_base)
            },
            {
                "name": "list_agents",
                "method": "GET",
                "url": format!("{}/evm/agents?active=true", api_base)
            },
            {
                "name": "prepare_create_agent",
                "method": "POST",
                "url": format!("{}/evm/write/agents/create", api_base)
            },
            {
                "name": "prepare_execute_agent",
                "method": "POST",
                "url": format!("{}/evm/write/agents/execute", api_base)
            },
            {
                "name": "send_swarm_message",
                "method": "POST",
                "url": format!("{}/web4/xmtp/swarm/send", api_base)
            }
        ]
    }))
}

pub async fn get_xmtp_swarm_health(state: web::Data<Arc<AppState>>) -> impl Responder {
    HttpResponse::Ok().json(xmtp_swarm::health(&state))
}

pub async fn send_xmtp_swarm_message(
    state: web::Data<Arc<AppState>>,
    body: web::Json<SwarmSendRequest>,
) -> Result<impl Responder, ApiError> {
    let message = xmtp_swarm::send_message(&state, body.into_inner()).await?;
    Ok(HttpResponse::Created().json(message))
}

pub async fn list_xmtp_swarm_messages(
    state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    query: web::Query<SwarmListQuery>,
) -> Result<impl Responder, ApiError> {
    let response = xmtp_swarm::list_messages(&state, path.as_str(), query.into_inner()).await?;
    Ok(HttpResponse::Ok().json(response))
}
