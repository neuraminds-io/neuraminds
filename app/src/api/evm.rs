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
const ORDER_BOOK_COUNT_SELECTOR: &str = "0x2453ffa8";
const ORDER_BOOK_ORDERS_SELECTOR: &str = "0xa85c38ef";
const ORDER_FILLED_TOPIC: &str =
    "0x5aac01386940f75e601757cfe5dc1d4ab2bac84f98d30664486114a8abb38a45";
const MAX_MARKETS_PAGE_SIZE: u64 = 200;
const MAX_ORDERBOOK_DEPTH: u64 = 100;
const MAX_TRADES_PAGE_SIZE: u64 = 200;
const ORDERBOOK_SCAN_WINDOW: u64 = 150;
const TRADES_BLOCK_SCAN_WINDOW: u64 = 25_000;

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
        let calldata = format!(
            "{}{}",
            MARKET_CORE_MARKETS_SELECTOR,
            encode_u256_hex(index)
        );
        let slot = state
            .evm_rpc
            .eth_call(market_core, &calldata)
            .await
            .map_err(map_evm_rpc_error)?;
        markets.push(decode_market_snapshot(index, &slot)?);
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
    let market_id = market_id_raw
        .parse::<u64>()
        .map_err(|_| ApiError::bad_request("INVALID_MARKET_ID", "market_id must be a positive integer"))?;

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

fn is_valid_evm_address(address: &str) -> bool {
    address.len() == 42
        && address.starts_with("0x")
        && address[2..].chars().all(|c| c.is_ascii_hexdigit())
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

    u64::from_str_radix(normalized, 16)
        .map_err(|_| ApiError::internal("Invalid RPC hex result"))
}

fn parse_bool_word(word: &str) -> Result<bool, ApiError> {
    Ok(parse_u64_hex(word)? != 0)
}

fn encode_u256_hex(value: u64) -> String {
    format!("{:064x}", value)
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
        assert_eq!(decoded.resolver, "0x71c7656ec7ab88b098defb751b7401b5f6d8976f");
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
