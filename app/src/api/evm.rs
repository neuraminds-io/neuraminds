use actix_web::{web, HttpResponse, Responder};
use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::api::ApiError;
use crate::AppState;

const ERC20_TOTAL_SUPPLY_SELECTOR: &str = "0x18160ddd";
const ERC20_DECIMALS_SELECTOR: &str = "0x313ce567";
const MARKET_CORE_COUNT_SELECTOR: &str = "0xec979082";
const MARKET_CORE_MARKETS_SELECTOR: &str = "0xb1283e77";
const MARKET_CORE_METADATA_SELECTOR: &str = "0x6b6445b6";
const MARKET_CORE_CREATE_RICH_SELECTOR: &str = "0xddabefe7";
const ORDER_BOOK_COUNT_SELECTOR: &str = "0x2453ffa8";
const ORDER_BOOK_ORDERS_SELECTOR: &str = "0xa85c38ef";
const ORDER_BOOK_PLACE_SELECTOR: &str = "0xa8dd6515";
const ORDER_BOOK_CANCEL_SELECTOR: &str = "0x514fcac7";
const ORDER_BOOK_CLAIM_SELECTOR: &str = "0x379607f5";
const ORDER_BOOK_MATCH_SELECTOR: &str = "0xc6437097";
const AGENT_RUNTIME_CREATE_SELECTOR: &str = "0x325993ba";
const AGENT_RUNTIME_EXECUTE_SELECTOR: &str = "0xe2a343a5";
const ORDER_FILLED_TOPIC: &str =
    "0x5aac01386940f75e601757cfe5dc1d4ab2bac84f98d30664486114a8abb38a45";
const MAX_MARKETS_PAGE_SIZE: u64 = 200;
const MAX_ORDERBOOK_DEPTH: u64 = 100;
const MAX_TRADES_PAGE_SIZE: u64 = 200;
const ORDERBOOK_SCAN_WINDOW: u64 = 150;
const TRADES_BLOCK_SCAN_WINDOW: u64 = 25_000;
const MAX_MARKET_TEXT_LENGTH: usize = 2_048;

#[derive(Serialize)]
pub struct BaseTokenStateResponse {
    pub chain_id: u64,
    pub token_address: String,
    pub total_supply_hex: String,
    pub decimals: u8,
}

#[derive(Deserialize)]
pub struct BaseMarketsQuery {
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[derive(Deserialize)]
pub struct BaseOrderBookQuery {
    pub outcome: Option<String>,
    pub depth: Option<u64>,
}

#[derive(Deserialize)]
pub struct BaseTradesQuery {
    pub outcome: Option<String>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[derive(Serialize)]
pub struct BaseMarketsResponse {
    pub markets: Vec<BaseMarketSnapshot>,
    pub total: u64,
    pub limit: u64,
    pub offset: u64,
    pub source: String,
}

#[derive(Serialize)]
pub struct BaseMarketSnapshot {
    pub id: String,
    pub question_hash: String,
    pub question: String,
    pub description: String,
    pub category: String,
    pub resolution_source: String,
    pub resolver: String,
    pub close_time: u64,
    pub resolve_time: u64,
    pub resolved: bool,
    pub outcome: Option<String>,
    pub status: String,
}

#[derive(Serialize)]
pub struct BaseOrderBookLevel {
    pub price: f64,
    pub quantity: f64,
    pub orders: u64,
}

#[derive(Serialize)]
pub struct BaseOrderBookResponse {
    pub market_id: String,
    pub outcome: String,
    pub bids: Vec<BaseOrderBookLevel>,
    pub asks: Vec<BaseOrderBookLevel>,
    pub last_updated: String,
    pub source: String,
}

#[derive(Serialize)]
pub struct BaseTradeSnapshot {
    pub id: String,
    pub market_id: String,
    pub outcome: String,
    pub price: f64,
    pub price_bps: u64,
    pub quantity: u64,
    pub tx_hash: String,
    pub block_number: u64,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct BaseTradesResponse {
    pub trades: Vec<BaseTradeSnapshot>,
    pub total: u64,
    pub limit: u64,
    pub offset: u64,
    pub has_more: bool,
    pub source: String,
}

#[derive(Serialize)]
pub struct PreparedEvmWriteResponse {
    pub chain_id: u64,
    pub from: Option<String>,
    pub to: String,
    pub data: String,
    pub value: String,
    pub method: String,
}

#[derive(Serialize)]
pub struct RelayRawTransactionResponse {
    pub chain_id: u64,
    pub tx_hash: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareCreateMarketWriteRequest {
    pub from: Option<String>,
    pub question: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub resolution_source: Option<String>,
    pub close_time: u64,
    pub resolver: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparePlaceOrderWriteRequest {
    pub from: Option<String>,
    pub market_id: u64,
    pub outcome: String,
    pub price_bps: u64,
    pub size: String,
    pub expiry: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareCancelOrderWriteRequest {
    pub from: Option<String>,
    pub order_id: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareClaimWriteRequest {
    pub from: Option<String>,
    pub market_id: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareMatchOrdersWriteRequest {
    pub from: Option<String>,
    pub first_order_id: u64,
    pub second_order_id: u64,
    pub fill_size: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareCreateAgentWriteRequest {
    pub from: Option<String>,
    pub market_id: u64,
    pub is_yes: bool,
    pub price_bps: u64,
    pub size: String,
    pub cadence: u64,
    pub expiry_window: u64,
    pub strategy: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareExecuteAgentWriteRequest {
    pub from: Option<String>,
    pub agent_id: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelayRawTransactionRequest {
    pub raw_tx: String,
}

#[derive(Default)]
struct LevelAggregate {
    quantity: u64,
    orders: u64,
}

struct BaseRawOrder {
    market_id: u64,
    is_yes: bool,
    price_bps: u64,
    remaining: u64,
    expiry: u64,
    canceled: bool,
}

#[derive(Clone)]
struct PendingTrade {
    id: String,
    order_id: u64,
    block_number: u64,
    log_index: u64,
    tx_hash: String,
    quantity: u64,
    outcome: String,
    price_bps: u64,
    created_at: String,
}

pub async fn get_neura_token_state(
    state: web::Data<Arc<AppState>>,
) -> Result<impl Responder, ApiError> {
    if !state.config.evm_enabled || !state.config.evm_reads_enabled {
        return Err(ApiError::bad_request(
            "EVM_DISABLED",
            "EVM services are disabled",
        ));
    }

    let token_address = state.config.neura_token_address.trim();
    if token_address.is_empty() {
        return Err(ApiError::bad_request(
            "TOKEN_ADDRESS_NOT_CONFIGURED",
            "NEURA_TOKEN_ADDRESS is not configured",
        ));
    }
    if !is_valid_evm_address(token_address) {
        return Err(ApiError::bad_request(
            "INVALID_TOKEN_ADDRESS",
            "NEURA_TOKEN_ADDRESS must be a valid 0x EVM address",
        ));
    }

    let total_supply_hex = state
        .evm_rpc
        .eth_call(token_address, ERC20_TOTAL_SUPPLY_SELECTOR)
        .await
        .map_err(map_evm_rpc_error)?;
    let decimals_hex = state
        .evm_rpc
        .eth_call(token_address, ERC20_DECIMALS_SELECTOR)
        .await
        .map_err(map_evm_rpc_error)?;
    let decimals = parse_u8_hex(&decimals_hex)?;

    Ok(HttpResponse::Ok().json(BaseTokenStateResponse {
        chain_id: state.config.base_chain_id,
        token_address: token_address.to_ascii_lowercase(),
        total_supply_hex,
        decimals,
    }))
}

pub async fn get_base_markets(
    state: web::Data<Arc<AppState>>,
    query: web::Query<BaseMarketsQuery>,
) -> Result<impl Responder, ApiError> {
    if !state.config.evm_enabled || !state.config.evm_reads_enabled {
        return Err(ApiError::bad_request(
            "EVM_DISABLED",
            "EVM services are disabled",
        ));
    }

    let market_core = state.config.market_core_address.trim();
    if market_core.is_empty() {
        return Err(ApiError::bad_request(
            "MARKET_CORE_ADDRESS_NOT_CONFIGURED",
            "MARKET_CORE_ADDRESS must be configured for Base markets",
        ));
    }

    if !is_valid_evm_address(market_core) {
        return Err(ApiError::bad_request(
            "INVALID_MARKET_CORE_ADDRESS",
            "MARKET_CORE_ADDRESS must be a valid 0x EVM address",
        ));
    }

    let total_hex = state
        .evm_rpc
        .eth_call(market_core, MARKET_CORE_COUNT_SELECTOR)
        .await
        .map_err(map_evm_rpc_error)?;
    let total = parse_u64_hex(&total_hex)?;

    let limit = query.limit.unwrap_or(50).min(MAX_MARKETS_PAGE_SIZE);
    let offset = query.offset.unwrap_or(0);

    if total == 0 || offset >= total {
        return Ok(HttpResponse::Ok().json(BaseMarketsResponse {
            markets: vec![],
            total,
            limit,
            offset,
            source: "market_core".to_string(),
        }));
    }

    let end = (offset + limit).min(total);
    let mut markets = Vec::new();
    for index in (offset + 1)..=end {
        let calldata = format!("{}{}", MARKET_CORE_MARKETS_SELECTOR, encode_u256_hex(index));
        let slot = state
            .evm_rpc
            .eth_call(market_core, &calldata)
            .await
            .map_err(map_evm_rpc_error)?;
        let mut snapshot = decode_market_snapshot(index, &slot)?;

        let metadata_calldata = format!(
            "{}{}",
            MARKET_CORE_METADATA_SELECTOR,
            encode_u256_hex(index)
        );
        if let Ok(payload) = state.evm_rpc.eth_call(market_core, &metadata_calldata).await {
            if let Ok((question, description, category, resolution_source)) =
                decode_market_metadata_tuple(&payload)
            {
                snapshot.question = question;
                snapshot.description = description;
                snapshot.category = category;
                snapshot.resolution_source = resolution_source;
            }
        }

        markets.push(snapshot);
    }

    Ok(HttpResponse::Ok().json(BaseMarketsResponse {
        markets,
        total,
        limit,
        offset,
        source: "market_core".to_string(),
    }))
}

pub async fn get_base_orderbook(
    state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    query: web::Query<BaseOrderBookQuery>,
) -> Result<impl Responder, ApiError> {
    if !state.config.evm_enabled || !state.config.evm_reads_enabled {
        return Err(ApiError::bad_request(
            "EVM_DISABLED",
            "EVM services are disabled",
        ));
    }

    let market_id_raw = path.into_inner();
    let market_id = market_id_raw.parse::<u64>().map_err(|_| {
        ApiError::bad_request("INVALID_MARKET_ID", "market_id must be a positive integer")
    })?;

    let outcome = match query.outcome.as_deref().unwrap_or("yes") {
        "yes" => "yes",
        "no" => "no",
        _ => {
            return Err(ApiError::bad_request(
                "INVALID_OUTCOME",
                "outcome must be either 'yes' or 'no'",
            ));
        }
    };
    let outcome_is_yes = outcome == "yes";
    let depth = query.depth.unwrap_or(20).min(MAX_ORDERBOOK_DEPTH);

    let order_book = state.config.order_book_address.trim();
    if order_book.is_empty() {
        return Err(ApiError::bad_request(
            "ORDER_BOOK_ADDRESS_NOT_CONFIGURED",
            "ORDER_BOOK_ADDRESS must be configured for Base order books",
        ));
    }

    if !is_valid_evm_address(order_book) {
        return Err(ApiError::bad_request(
            "INVALID_ORDER_BOOK_ADDRESS",
            "ORDER_BOOK_ADDRESS must be a valid 0x EVM address",
        ));
    }

    let total_hex = state
        .evm_rpc
        .eth_call(order_book, ORDER_BOOK_COUNT_SELECTOR)
        .await
        .map_err(map_evm_rpc_error)?;
    let total = parse_u64_hex(&total_hex)?;
    if total == 0 {
        return Ok(HttpResponse::Ok().json(BaseOrderBookResponse {
            market_id: market_id_raw,
            outcome: outcome.to_string(),
            bids: vec![],
            asks: vec![],
            last_updated: Utc::now().to_rfc3339(),
            source: "order_book_contract".to_string(),
        }));
    }

    let start = if total > ORDERBOOK_SCAN_WINDOW {
        total - ORDERBOOK_SCAN_WINDOW + 1
    } else {
        1
    };

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ApiError::internal("System time error"))?
        .as_secs();

    let mut bid_levels: BTreeMap<u64, LevelAggregate> = BTreeMap::new();
    let mut ask_levels: BTreeMap<u64, LevelAggregate> = BTreeMap::new();

    for order_id in (start..=total).rev() {
        let calldata = format!(
            "{}{}",
            ORDER_BOOK_ORDERS_SELECTOR,
            encode_u256_hex(order_id)
        );
        let payload = state
            .evm_rpc
            .eth_call(order_book, &calldata)
            .await
            .map_err(map_evm_rpc_error)?;
        let Some(order) = decode_order_snapshot(&payload)? else {
            continue;
        };

        if order.market_id != market_id
            || order.canceled
            || order.remaining == 0
            || order.expiry < now
            || order.price_bps == 0
            || order.price_bps >= 10_000
        {
            continue;
        }

        if order.is_yes == outcome_is_yes {
            let level = bid_levels.entry(order.price_bps).or_default();
            level.quantity += order.remaining;
            level.orders += 1;
        } else {
            let ask_price_bps = 10_000 - order.price_bps;
            if ask_price_bps == 0 || ask_price_bps >= 10_000 {
                continue;
            }
            let level = ask_levels.entry(ask_price_bps).or_default();
            level.quantity += order.remaining;
            level.orders += 1;
        }
    }

    let bids = bid_levels
        .into_iter()
        .rev()
        .take(depth as usize)
        .map(|(price_bps, level)| BaseOrderBookLevel {
            price: (price_bps as f64) / 10_000.0,
            quantity: level.quantity as f64,
            orders: level.orders,
        })
        .collect::<Vec<_>>();

    let asks = ask_levels
        .into_iter()
        .take(depth as usize)
        .map(|(price_bps, level)| BaseOrderBookLevel {
            price: (price_bps as f64) / 10_000.0,
            quantity: level.quantity as f64,
            orders: level.orders,
        })
        .collect::<Vec<_>>();

    Ok(HttpResponse::Ok().json(BaseOrderBookResponse {
        market_id: market_id_raw,
        outcome: outcome.to_string(),
        bids,
        asks,
        last_updated: Utc::now().to_rfc3339(),
        source: "order_book_contract".to_string(),
    }))
}

pub async fn get_base_trades(
    state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    query: web::Query<BaseTradesQuery>,
) -> Result<impl Responder, ApiError> {
    if !state.config.evm_enabled || !state.config.evm_reads_enabled {
        return Err(ApiError::bad_request(
            "EVM_DISABLED",
            "EVM services are disabled",
        ));
    }

    let market_id_raw = path.into_inner();
    let market_id = market_id_raw.parse::<u64>().map_err(|_| {
        ApiError::bad_request("INVALID_MARKET_ID", "market_id must be a positive integer")
    })?;
    let limit = query.limit.unwrap_or(50).min(MAX_TRADES_PAGE_SIZE);
    let offset = query.offset.unwrap_or(0);

    let outcome_filter = match query.outcome.as_deref() {
        None => None,
        Some("yes") => Some(true),
        Some("no") => Some(false),
        Some(_) => {
            return Err(ApiError::bad_request(
                "INVALID_OUTCOME",
                "outcome must be either 'yes' or 'no'",
            ))
        }
    };

    let order_book = state.config.order_book_address.trim();
    if order_book.is_empty() {
        return Err(ApiError::bad_request(
            "ORDER_BOOK_ADDRESS_NOT_CONFIGURED",
            "ORDER_BOOK_ADDRESS must be configured for Base trades",
        ));
    }

    if !is_valid_evm_address(order_book) {
        return Err(ApiError::bad_request(
            "INVALID_ORDER_BOOK_ADDRESS",
            "ORDER_BOOK_ADDRESS must be a valid 0x EVM address",
        ));
    }

    let latest_block = state
        .evm_rpc
        .eth_block_number()
        .await
        .map_err(map_evm_rpc_error)?;
    if latest_block == 0 {
        return Ok(HttpResponse::Ok().json(BaseTradesResponse {
            trades: vec![],
            total: 0,
            limit,
            offset,
            has_more: false,
            source: "order_book_contract".to_string(),
        }));
    }

    let from_block = latest_block.saturating_sub(TRADES_BLOCK_SCAN_WINDOW);
    let _ = state
        .evm_indexer
        .sync(
            state.config.market_core_address.trim(),
            order_book,
            TRADES_BLOCK_SCAN_WINDOW,
            &[ORDER_FILLED_TOPIC],
        )
        .await;

    let indexed_logs = state.evm_indexer.logs_by_topic(ORDER_FILLED_TOPIC).await;
    let mut logs = indexed_logs
        .into_iter()
        .filter(|entry| {
            entry
                .block_number
                .as_deref()
                .and_then(|v| parse_u64_hex(v).ok())
                .map(|block| block >= from_block && block <= latest_block)
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    if logs.is_empty() {
        logs = state
            .evm_rpc
            .eth_get_logs(order_book, ORDER_FILLED_TOPIC, from_block, latest_block)
            .await
            .map_err(map_evm_rpc_error)?;
    }

    let mut trades = Vec::new();
    let mut block_timestamp_cache: HashMap<u64, u64> = HashMap::new();
    for log in logs {
        let order_id = match log.topics.get(1) {
            Some(topic) => match parse_u64_hex(topic) {
                Ok(value) => value,
                Err(_) => continue,
            },
            None => continue,
        };

        let block_number = match log.block_number.as_deref() {
            Some(value) => match parse_u64_hex(value) {
                Ok(parsed) => parsed,
                Err(_) => continue,
            },
            None => continue,
        };
        let log_index = match log.log_index.as_deref() {
            Some(value) => parse_u64_hex(value).unwrap_or(0),
            None => 0,
        };

        let fill_size = match word_at(&log.data, 0).and_then(parse_u64_hex) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if fill_size == 0 {
            continue;
        }

        let calldata = format!(
            "{}{}",
            ORDER_BOOK_ORDERS_SELECTOR,
            encode_u256_hex(order_id)
        );
        let payload = state
            .evm_rpc
            .eth_call(order_book, &calldata)
            .await
            .map_err(map_evm_rpc_error)?;
        let Some(order) = decode_order_snapshot(&payload)? else {
            continue;
        };

        if order.market_id != market_id {
            continue;
        }
        if let Some(expected) = outcome_filter {
            if order.is_yes != expected {
                continue;
            }
        }
        if order.price_bps == 0 || order.price_bps >= 10_000 {
            continue;
        }

        let timestamp = if let Some(ts) = block_timestamp_cache.get(&block_number) {
            *ts
        } else {
            let ts = state
                .evm_rpc
                .eth_get_block_timestamp(block_number)
                .await
                .map_err(map_evm_rpc_error)?;
            block_timestamp_cache.insert(block_number, ts);
            ts
        };

        let tx_hash = log.transaction_hash.unwrap_or_default();
        let id = if tx_hash.is_empty() {
            format!("base-{}-{}", order_id, log_index)
        } else {
            format!("base-{}-{}", tx_hash, log_index)
        };

        trades.push(PendingTrade {
            id,
            order_id,
            block_number,
            log_index,
            tx_hash,
            quantity: fill_size,
            outcome: if order.is_yes {
                "yes".to_string()
            } else {
                "no".to_string()
            },
            price_bps: order.price_bps,
            created_at: unix_to_rfc3339(timestamp),
        });
    }

    trades.sort_by(|a, b| {
        b.block_number
            .cmp(&a.block_number)
            .then_with(|| b.log_index.cmp(&a.log_index))
            .then_with(|| b.order_id.cmp(&a.order_id))
    });

    let total = trades.len() as u64;
    if offset >= total {
        return Ok(HttpResponse::Ok().json(BaseTradesResponse {
            trades: vec![],
            total,
            limit,
            offset,
            has_more: false,
            source: "order_book_contract".to_string(),
        }));
    }

    let end = (offset + limit).min(total);
    let mut page = Vec::new();
    for entry in trades
        .into_iter()
        .skip(offset as usize)
        .take((end - offset) as usize)
    {
        page.push(BaseTradeSnapshot {
            id: entry.id,
            market_id: market_id_raw.clone(),
            outcome: entry.outcome,
            price: (entry.price_bps as f64) / 10_000.0,
            price_bps: entry.price_bps,
            quantity: entry.quantity,
            tx_hash: entry.tx_hash,
            block_number: entry.block_number,
            created_at: entry.created_at,
        });
    }

    Ok(HttpResponse::Ok().json(BaseTradesResponse {
        trades: page,
        total,
        limit,
        offset,
        has_more: end < total,
        source: "order_book_contract".to_string(),
    }))
}

pub async fn prepare_create_market_write(
    state: web::Data<Arc<AppState>>,
    body: web::Json<PrepareCreateMarketWriteRequest>,
) -> Result<impl Responder, ApiError> {
    ensure_evm_writes_enabled(&state)?;

    let market_core = configured_address(
        &state.config.market_core_address,
        "MARKET_CORE_ADDRESS_NOT_CONFIGURED",
        "MARKET_CORE_ADDRESS must be configured for write operations",
    )?;

    let resolver = normalize_required_address(
        body.resolver.as_str(),
        "INVALID_RESOLVER",
        "resolver must be a valid 0x EVM address",
    )?;
    let from = normalize_optional_address(body.from.as_ref())?;

    let question = body.question.trim();
    if question.is_empty() {
        return Err(ApiError::bad_request(
            "INVALID_QUESTION",
            "question must not be empty",
        ));
    }
    if question.len() > MAX_MARKET_TEXT_LENGTH {
        return Err(ApiError::bad_request(
            "QUESTION_TOO_LONG",
            "question exceeds max length",
        ));
    }

    let description = body.description.as_deref().unwrap_or("").trim();
    let category = body.category.as_deref().unwrap_or("").trim();
    let resolution_source = body.resolution_source.as_deref().unwrap_or("").trim();
    if description.len() > MAX_MARKET_TEXT_LENGTH
        || category.len() > MAX_MARKET_TEXT_LENGTH
        || resolution_source.len() > MAX_MARKET_TEXT_LENGTH
    {
        return Err(ApiError::bad_request(
            "MARKET_TEXT_TOO_LONG",
            "description/category/resolutionSource exceeds max length",
        ));
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ApiError::internal("System time error"))?
        .as_secs();
    if body.close_time <= now {
        return Err(ApiError::bad_request(
            "INVALID_CLOSE_TIME",
            "closeTime must be in the future",
        ));
    }

    let data = encode_create_market_rich_calldata(
        question,
        description,
        category,
        resolution_source,
        body.close_time,
        &resolver,
    )?;

    Ok(HttpResponse::Ok().json(prepared_write_response(
        state.config.base_chain_id,
        from,
        market_core,
        data,
        "createMarketRich",
    )))
}

pub async fn prepare_place_order_write(
    state: web::Data<Arc<AppState>>,
    body: web::Json<PreparePlaceOrderWriteRequest>,
) -> Result<impl Responder, ApiError> {
    ensure_evm_writes_enabled(&state)?;

    let order_book = configured_address(
        &state.config.order_book_address,
        "ORDER_BOOK_ADDRESS_NOT_CONFIGURED",
        "ORDER_BOOK_ADDRESS must be configured for write operations",
    )?;
    let from = normalize_optional_address(body.from.as_ref())?;

    let is_yes = match body.outcome.as_str() {
        "yes" => true,
        "no" => false,
        _ => {
            return Err(ApiError::bad_request(
                "INVALID_OUTCOME",
                "outcome must be either 'yes' or 'no'",
            ))
        }
    };

    if body.price_bps == 0 || body.price_bps >= 10_000 {
        return Err(ApiError::bad_request(
            "INVALID_PRICE_BPS",
            "priceBps must be between 1 and 9999",
        ));
    }

    let size = parse_u128_decimal(&body.size, "size")?;
    if size == 0 {
        return Err(ApiError::bad_request(
            "INVALID_SIZE",
            "size must be greater than zero",
        ));
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ApiError::internal("System time error"))?
        .as_secs();
    if body.expiry <= now {
        return Err(ApiError::bad_request(
            "INVALID_EXPIRY",
            "expiry must be in the future",
        ));
    }

    let data = format!(
        "{}{}{}{}{}{}",
        ORDER_BOOK_PLACE_SELECTOR,
        encode_u256_hex(body.market_id),
        encode_bool_word(is_yes),
        encode_u256_hex_u128(body.price_bps as u128),
        encode_u256_hex_u128(size),
        encode_u256_hex(body.expiry),
    );

    Ok(HttpResponse::Ok().json(prepared_write_response(
        state.config.base_chain_id,
        from,
        order_book,
        data,
        "placeOrder",
    )))
}

pub async fn prepare_cancel_order_write(
    state: web::Data<Arc<AppState>>,
    body: web::Json<PrepareCancelOrderWriteRequest>,
) -> Result<impl Responder, ApiError> {
    ensure_evm_writes_enabled(&state)?;

    let order_book = configured_address(
        &state.config.order_book_address,
        "ORDER_BOOK_ADDRESS_NOT_CONFIGURED",
        "ORDER_BOOK_ADDRESS must be configured for write operations",
    )?;
    let from = normalize_optional_address(body.from.as_ref())?;

    let data = format!(
        "{}{}",
        ORDER_BOOK_CANCEL_SELECTOR,
        encode_u256_hex(body.order_id)
    );

    Ok(HttpResponse::Ok().json(prepared_write_response(
        state.config.base_chain_id,
        from,
        order_book,
        data,
        "cancelOrder",
    )))
}

pub async fn prepare_claim_write(
    state: web::Data<Arc<AppState>>,
    body: web::Json<PrepareClaimWriteRequest>,
) -> Result<impl Responder, ApiError> {
    ensure_evm_writes_enabled(&state)?;

    let order_book = configured_address(
        &state.config.order_book_address,
        "ORDER_BOOK_ADDRESS_NOT_CONFIGURED",
        "ORDER_BOOK_ADDRESS must be configured for write operations",
    )?;
    let from = normalize_optional_address(body.from.as_ref())?;

    let data = format!(
        "{}{}",
        ORDER_BOOK_CLAIM_SELECTOR,
        encode_u256_hex(body.market_id)
    );

    Ok(HttpResponse::Ok().json(prepared_write_response(
        state.config.base_chain_id,
        from,
        order_book,
        data,
        "claim",
    )))
}

pub async fn prepare_match_orders_write(
    state: web::Data<Arc<AppState>>,
    body: web::Json<PrepareMatchOrdersWriteRequest>,
) -> Result<impl Responder, ApiError> {
    ensure_evm_writes_enabled(&state)?;

    let order_book = configured_address(
        &state.config.order_book_address,
        "ORDER_BOOK_ADDRESS_NOT_CONFIGURED",
        "ORDER_BOOK_ADDRESS must be configured for write operations",
    )?;
    let from = normalize_optional_address(body.from.as_ref())?;

    if body.first_order_id == body.second_order_id {
        return Err(ApiError::bad_request(
            "INVALID_MATCH_PAIR",
            "firstOrderId and secondOrderId must differ",
        ));
    }
    let fill_size = parse_u128_decimal(&body.fill_size, "fillSize")?;
    if fill_size == 0 {
        return Err(ApiError::bad_request(
            "INVALID_FILL_SIZE",
            "fillSize must be greater than zero",
        ));
    }

    let data = format!(
        "{}{}{}{}",
        ORDER_BOOK_MATCH_SELECTOR,
        encode_u256_hex(body.first_order_id),
        encode_u256_hex(body.second_order_id),
        encode_u256_hex_u128(fill_size),
    );

    Ok(HttpResponse::Ok().json(prepared_write_response(
        state.config.base_chain_id,
        from,
        order_book,
        data,
        "matchOrders",
    )))
}

pub async fn prepare_create_agent_write(
    state: web::Data<Arc<AppState>>,
    body: web::Json<PrepareCreateAgentWriteRequest>,
) -> Result<impl Responder, ApiError> {
    ensure_evm_writes_enabled(&state)?;

    let agent_runtime = configured_address(
        &state.config.agent_runtime_address,
        "AGENT_RUNTIME_ADDRESS_NOT_CONFIGURED",
        "AGENT_RUNTIME_ADDRESS must be configured for write operations",
    )?;
    let from = normalize_optional_address(body.from.as_ref())?;

    if body.price_bps == 0 || body.price_bps >= 10_000 {
        return Err(ApiError::bad_request(
            "INVALID_PRICE_BPS",
            "priceBps must be between 1 and 9999",
        ));
    }
    let size = parse_u128_decimal(&body.size, "size")?;
    if size == 0 {
        return Err(ApiError::bad_request(
            "INVALID_SIZE",
            "size must be greater than zero",
        ));
    }
    if body.cadence == 0 || body.expiry_window == 0 {
        return Err(ApiError::bad_request(
            "INVALID_AGENT_TIMING",
            "cadence and expiryWindow must be greater than zero",
        ));
    }
    if body.strategy.len() > MAX_MARKET_TEXT_LENGTH {
        return Err(ApiError::bad_request(
            "STRATEGY_TOO_LONG",
            "strategy exceeds max length",
        ));
    }

    let data = encode_create_agent_calldata(
        body.market_id,
        body.is_yes,
        body.price_bps,
        size,
        body.cadence,
        body.expiry_window,
        body.strategy.as_str(),
    )?;

    Ok(HttpResponse::Ok().json(prepared_write_response(
        state.config.base_chain_id,
        from,
        agent_runtime,
        data,
        "createAgent",
    )))
}

pub async fn prepare_execute_agent_write(
    state: web::Data<Arc<AppState>>,
    body: web::Json<PrepareExecuteAgentWriteRequest>,
) -> Result<impl Responder, ApiError> {
    ensure_evm_writes_enabled(&state)?;

    let agent_runtime = configured_address(
        &state.config.agent_runtime_address,
        "AGENT_RUNTIME_ADDRESS_NOT_CONFIGURED",
        "AGENT_RUNTIME_ADDRESS must be configured for write operations",
    )?;
    let from = normalize_optional_address(body.from.as_ref())?;
    let data = format!(
        "{}{}",
        AGENT_RUNTIME_EXECUTE_SELECTOR,
        encode_u256_hex(body.agent_id)
    );

    Ok(HttpResponse::Ok().json(prepared_write_response(
        state.config.base_chain_id,
        from,
        agent_runtime,
        data,
        "executeAgent",
    )))
}

pub async fn relay_raw_transaction(
    state: web::Data<Arc<AppState>>,
    body: web::Json<RelayRawTransactionRequest>,
) -> Result<impl Responder, ApiError> {
    ensure_evm_writes_enabled(&state)?;

    if !is_valid_hex_payload(body.raw_tx.as_str()) {
        return Err(ApiError::bad_request(
            "INVALID_RAW_TX",
            "rawTx must be a valid 0x-prefixed hex string",
        ));
    }

    let tx_hash = state
        .evm_rpc
        .eth_send_raw_transaction(body.raw_tx.as_str())
        .await
        .map_err(map_evm_rpc_error)?;

    Ok(HttpResponse::Ok().json(RelayRawTransactionResponse {
        chain_id: state.config.base_chain_id,
        tx_hash,
    }))
}

fn is_valid_evm_address(address: &str) -> bool {
    address.len() == 42
        && address.starts_with("0x")
        && address[2..].chars().all(|c| c.is_ascii_hexdigit())
}

fn is_valid_hex_payload(value: &str) -> bool {
    value.len() >= 4
        && value.starts_with("0x")
        && value.len() % 2 == 0
        && value[2..].chars().all(|c| c.is_ascii_hexdigit())
}

fn ensure_evm_writes_enabled(state: &Arc<AppState>) -> Result<(), ApiError> {
    if !state.config.evm_enabled || !state.config.evm_writes_enabled {
        return Err(ApiError::bad_request(
            "EVM_WRITES_DISABLED",
            "EVM write operations are disabled",
        ));
    }
    Ok(())
}

fn configured_address(address: &str, code: &str, message: &str) -> Result<String, ApiError> {
    let trimmed = address.trim();
    if trimmed.is_empty() {
        return Err(ApiError::bad_request(code, message));
    }
    normalize_required_address(trimmed, code, message)
}

fn normalize_required_address(address: &str, code: &str, message: &str) -> Result<String, ApiError> {
    let trimmed = address.trim();
    if !is_valid_evm_address(trimmed) {
        return Err(ApiError::bad_request(code, message));
    }
    Ok(trimmed.to_ascii_lowercase())
}

fn normalize_optional_address(address: Option<&String>) -> Result<Option<String>, ApiError> {
    match address {
        None => Ok(None),
        Some(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            if !is_valid_evm_address(trimmed) {
                return Err(ApiError::bad_request(
                    "INVALID_FROM_ADDRESS",
                    "from must be a valid 0x EVM address",
                ));
            }
            Ok(Some(trimmed.to_ascii_lowercase()))
        }
    }
}

fn parse_u8_hex(value: &str) -> Result<u8, ApiError> {
    let parsed = parse_u64_hex(value)?;
    if parsed > u8::MAX as u64 {
        return Err(ApiError::internal("RPC value out of range for u8"));
    }

    Ok(parsed as u8)
}

fn parse_u64_hex(value: &str) -> Result<u64, ApiError> {
    let trimmed = value.trim_start_matches("0x");
    if trimmed.is_empty() {
        return Err(ApiError::internal("Invalid RPC hex result"));
    }

    let normalized = trimmed.trim_start_matches('0');
    if normalized.is_empty() {
        return Ok(0);
    }
    if normalized.len() > 16 {
        return Err(ApiError::internal("RPC value out of range for u64"));
    }

    u64::from_str_radix(normalized, 16).map_err(|_| ApiError::internal("Invalid RPC hex result"))
}

fn parse_bool_word(word: &str) -> Result<bool, ApiError> {
    Ok(parse_u64_hex(word)? != 0)
}

fn encode_u256_hex(value: u64) -> String {
    format!("{:064x}", value)
}

fn encode_u256_hex_u128(value: u128) -> String {
    format!("{:064x}", value)
}

fn encode_bool_word(value: bool) -> String {
    if value {
        format!("{:064x}", 1)
    } else {
        format!("{:064x}", 0)
    }
}

fn encode_address_word(value: &str) -> Result<String, ApiError> {
    if !is_valid_evm_address(value) {
        return Err(ApiError::bad_request(
            "INVALID_ADDRESS",
            "address must be a valid 0x EVM address",
        ));
    }
    Ok(format!("{:0>64}", value[2..].to_ascii_lowercase()))
}

fn encode_dynamic_string_tail(value: &str) -> String {
    let encoded = hex::encode(value.as_bytes());
    let padded_len = if encoded.is_empty() {
        0
    } else {
        ((encoded.len() + 63) / 64) * 64
    };
    let mut padded = encoded;
    if padded.len() < padded_len {
        padded.push_str(&"0".repeat(padded_len - padded.len()));
    }
    format!("{}{}", encode_u256_hex_u128(value.len() as u128), padded)
}

fn encode_create_market_rich_calldata(
    question: &str,
    description: &str,
    category: &str,
    resolution_source: &str,
    close_time: u64,
    resolver: &str,
) -> Result<String, ApiError> {
    let question_tail = encode_dynamic_string_tail(question);
    let description_tail = encode_dynamic_string_tail(description);
    let category_tail = encode_dynamic_string_tail(category);
    let source_tail = encode_dynamic_string_tail(resolution_source);
    let resolver_word = encode_address_word(resolver)?;

    let head_len_bytes = 32usize * 6usize;
    let question_offset = head_len_bytes;
    let description_offset = question_offset + (question_tail.len() / 2);
    let category_offset = description_offset + (description_tail.len() / 2);
    let source_offset = category_offset + (category_tail.len() / 2);

    Ok(format!(
        "{}{}{}{}{}{}{}{}{}{}",
        MARKET_CORE_CREATE_RICH_SELECTOR,
        encode_u256_hex_u128(question_offset as u128),
        encode_u256_hex_u128(description_offset as u128),
        encode_u256_hex_u128(category_offset as u128),
        encode_u256_hex_u128(source_offset as u128),
        encode_u256_hex(close_time),
        resolver_word,
        question_tail,
        description_tail,
        format!("{}{}", category_tail, source_tail),
    ))
}

fn encode_create_agent_calldata(
    market_id: u64,
    is_yes: bool,
    price_bps: u64,
    size: u128,
    cadence: u64,
    expiry_window: u64,
    strategy: &str,
) -> Result<String, ApiError> {
    let strategy_tail = encode_dynamic_string_tail(strategy);
    let head_len_bytes = 32usize * 7usize;

    Ok(format!(
        "{}{}{}{}{}{}{}{}{}",
        AGENT_RUNTIME_CREATE_SELECTOR,
        encode_u256_hex(market_id),
        encode_bool_word(is_yes),
        encode_u256_hex_u128(price_bps as u128),
        encode_u256_hex_u128(size),
        encode_u256_hex(cadence),
        encode_u256_hex(expiry_window),
        encode_u256_hex_u128(head_len_bytes as u128),
        strategy_tail,
    ))
}

fn parse_u128_decimal(value: &str, field: &str) -> Result<u128, ApiError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ApiError::bad_request(
            "INVALID_NUMERIC_FIELD",
            &format!("{} is required", field),
        ));
    }
    trimmed.parse::<u128>().map_err(|_| {
        ApiError::bad_request(
            "INVALID_NUMERIC_FIELD",
            &format!("{} must be an unsigned integer string", field),
        )
    })
}

fn prepared_write_response(
    chain_id: u64,
    from: Option<String>,
    to: String,
    data: String,
    method: &str,
) -> PreparedEvmWriteResponse {
    PreparedEvmWriteResponse {
        chain_id,
        from,
        to,
        data: format!("0x{}", data.trim_start_matches("0x")),
        value: "0x0".to_string(),
        method: method.to_string(),
    }
}

fn word_at(data: &str, index: usize) -> Result<&str, ApiError> {
    if !data.starts_with("0x") {
        return Err(ApiError::internal("Invalid RPC hex result"));
    }

    let start = 2 + (index * 64);
    let end = start + 64;
    if data.len() < end {
        return Err(ApiError::internal("Invalid market slot payload"));
    }
    Ok(&data[start..end])
}

fn decode_market_metadata_tuple(payload: &str) -> Result<(String, String, String, String), ApiError> {
    Ok((
        decode_abi_string_at_offset(payload, word_at(payload, 0)?)?,
        decode_abi_string_at_offset(payload, word_at(payload, 1)?)?,
        decode_abi_string_at_offset(payload, word_at(payload, 2)?)?,
        decode_abi_string_at_offset(payload, word_at(payload, 3)?)?,
    ))
}

fn decode_abi_string_at_offset(payload: &str, offset_word: &str) -> Result<String, ApiError> {
    let offset = parse_u64_hex(offset_word)? as usize;
    if !payload.starts_with("0x") {
        return Err(ApiError::internal("Invalid ABI payload"));
    }

    let head = 2 + (offset * 2);
    if payload.len() < head + 64 {
        return Err(ApiError::internal("Invalid ABI payload"));
    }
    let len_word = &payload[head..head + 64];
    let length = parse_u64_hex(len_word)? as usize;
    let data_start = head + 64;
    let data_end = data_start + (length * 2);
    if payload.len() < data_end {
        return Err(ApiError::internal("Invalid ABI payload"));
    }

    let raw = &payload[data_start..data_end];
    let bytes = hex::decode(raw).map_err(|_| ApiError::internal("Invalid ABI payload"))?;
    String::from_utf8(bytes).map_err(|_| ApiError::internal("Invalid UTF-8 market metadata"))
}

fn decode_market_snapshot(index: u64, slot: &str) -> Result<BaseMarketSnapshot, ApiError> {
    let question_hash = format!("0x{}", word_at(slot, 0)?);
    let close_time = parse_u64_hex(word_at(slot, 1)?)?;
    let resolve_time = parse_u64_hex(word_at(slot, 2)?)?;
    let resolver_word = word_at(slot, 3)?;
    let resolver = format!("0x{}", &resolver_word[24..]).to_ascii_lowercase();
    let resolved = parse_bool_word(word_at(slot, 4)?)?;
    let outcome_true = parse_bool_word(word_at(slot, 5)?)?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ApiError::internal("System time error"))?
        .as_secs();

    let status = if resolved {
        "resolved".to_string()
    } else if close_time <= now {
        "closed".to_string()
    } else {
        "active".to_string()
    };

    Ok(BaseMarketSnapshot {
        id: index.to_string(),
        question_hash,
        question: String::new(),
        description: String::new(),
        category: String::new(),
        resolution_source: String::new(),
        resolver,
        close_time,
        resolve_time,
        resolved,
        outcome: if resolved {
            Some(if outcome_true {
                "yes".to_string()
            } else {
                "no".to_string()
            })
        } else {
            None
        },
        status,
    })
}

fn decode_order_snapshot(slot: &str) -> Result<Option<BaseRawOrder>, ApiError> {
    let maker_word = word_at(slot, 0)?;
    if maker_word.chars().all(|c| c == '0') {
        return Ok(None);
    }

    Ok(Some(BaseRawOrder {
        market_id: parse_u64_hex(word_at(slot, 1)?)?,
        is_yes: parse_bool_word(word_at(slot, 2)?)?,
        price_bps: parse_u64_hex(word_at(slot, 3)?)?,
        remaining: parse_u64_hex(word_at(slot, 5)?)?,
        expiry: parse_u64_hex(word_at(slot, 6)?)?,
        canceled: parse_bool_word(word_at(slot, 7)?)?,
    }))
}

fn map_evm_rpc_error(err: anyhow::Error) -> ApiError {
    ApiError::internal(&format!("Base RPC request failed: {}", err))
}

fn unix_to_rfc3339(timestamp: u64) -> String {
    Utc.timestamp_opt(timestamp as i64, 0)
        .single()
        .map(|value| value.to_rfc3339())
        .unwrap_or_else(|| Utc::now().to_rfc3339())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_evm_address() {
        assert!(is_valid_evm_address(
            "0x71C7656EC7ab88b098defB751B7401B5f6d8976F"
        ));
        assert!(!is_valid_evm_address("0x123"));
        assert!(!is_valid_evm_address(
            "71C7656EC7ab88b098defB751B7401B5f6d8976F"
        ));
    }

    #[test]
    fn test_parse_u8_hex() {
        assert_eq!(parse_u8_hex("0x12").unwrap(), 0x12);
        assert_eq!(
            parse_u8_hex("0x0000000000000000000000000000000000000000000000000000000000000006")
                .unwrap(),
            6
        );
        assert!(parse_u8_hex("0x100").is_err());
        assert!(parse_u8_hex("0x").is_err());
    }

    #[test]
    fn test_parse_u64_hex() {
        assert_eq!(parse_u64_hex("0x0").unwrap(), 0);
        assert_eq!(parse_u64_hex("0x2a").unwrap(), 42);
        assert_eq!(
            parse_u64_hex("0x00000000000000000000000000000000000000000000000000000000000000ff")
                .unwrap(),
            255
        );
    }

    #[test]
    fn test_encode_u256_hex() {
        let encoded = encode_u256_hex(42);
        assert_eq!(encoded.len(), 64);
        assert!(encoded.ends_with("2a"));
    }

    #[test]
    fn test_is_valid_hex_payload() {
        assert!(is_valid_hex_payload("0x1234"));
        assert!(!is_valid_hex_payload("0x123"));
        assert!(!is_valid_hex_payload("1234"));
    }

    #[test]
    fn test_decode_market_metadata_tuple() {
        let q = encode_dynamic_string_tail("question?");
        let d = encode_dynamic_string_tail("description");
        let c = encode_dynamic_string_tail("crypto");
        let s = encode_dynamic_string_tail("source");

        let head = format!(
            "{}{}{}{}",
            encode_u256_hex_u128(128),
            encode_u256_hex_u128(128 + q.len() as u128 / 2),
            encode_u256_hex_u128(128 + q.len() as u128 / 2 + d.len() as u128 / 2),
            encode_u256_hex_u128(128 + q.len() as u128 / 2 + d.len() as u128 / 2 + c.len() as u128 / 2),
        );
        let payload = format!("0x{}{}{}{}{}", head, q, d, c, s);
        let decoded = decode_market_metadata_tuple(&payload).unwrap();
        assert_eq!(decoded.0, "question?");
        assert_eq!(decoded.1, "description");
        assert_eq!(decoded.2, "crypto");
        assert_eq!(decoded.3, "source");
    }

    #[test]
    fn test_decode_market_snapshot() {
        let question_hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let close_time = format!("{:064x}", 1u64);
        let resolve_time = format!("{:064x}", 2u64);
        let resolver = "00000000000000000000000071c7656ec7ab88b098defb751b7401b5f6d8976f";
        let resolved = format!("{:064x}", 1u64);
        let outcome = format!("{:064x}", 1u64);

        let payload = format!(
            "0x{}{}{}{}{}{}",
            question_hash, close_time, resolve_time, resolver, resolved, outcome
        );

        let decoded = decode_market_snapshot(7, &payload).unwrap();
        assert_eq!(decoded.id, "7");
        assert_eq!(
            decoded.question_hash,
            "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        );
        assert_eq!(
            decoded.resolver,
            "0x71c7656ec7ab88b098defb751b7401b5f6d8976f"
        );
        assert_eq!(decoded.status, "resolved");
        assert_eq!(decoded.outcome.as_deref(), Some("yes"));
    }

    #[test]
    fn test_decode_order_snapshot() {
        let maker = "00000000000000000000000071c7656ec7ab88b098defb751b7401b5f6d8976f";
        let market_id = format!("{:064x}", 5u64);
        let is_yes = format!("{:064x}", 1u64);
        let price_bps = format!("{:064x}", 6300u64);
        let size = format!("{:064x}", 100u64);
        let remaining = format!("{:064x}", 25u64);
        let expiry = format!("{:064x}", 1_800_000_000u64);
        let canceled = format!("{:064x}", 0u64);

        let payload = format!(
            "0x{}{}{}{}{}{}{}{}",
            maker, market_id, is_yes, price_bps, size, remaining, expiry, canceled
        );
        let decoded = decode_order_snapshot(&payload).unwrap().unwrap();

        assert_eq!(decoded.market_id, 5);
        assert!(decoded.is_yes);
        assert_eq!(decoded.price_bps, 6300);
        assert_eq!(decoded.remaining, 25);
        assert_eq!(decoded.expiry, 1_800_000_000);
        assert!(!decoded.canceled);
    }

    #[test]
    fn test_decode_order_snapshot_empty_slot() {
        let maker = "0000000000000000000000000000000000000000000000000000000000000000";
        let payload = format!(
            "0x{}{}{}{}{}{}{}{}",
            maker,
            format!("{:064x}", 0u64),
            format!("{:064x}", 0u64),
            format!("{:064x}", 0u64),
            format!("{:064x}", 0u64),
            format!("{:064x}", 0u64),
            format!("{:064x}", 0u64),
            format!("{:064x}", 0u64)
        );
        assert!(decode_order_snapshot(&payload).unwrap().is_none());
    }

    #[test]
    fn test_unix_to_rfc3339() {
        let value = unix_to_rfc3339(1_700_000_000);
        assert!(value.starts_with("2023-"));
    }
}
