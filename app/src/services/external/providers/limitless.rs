use reqwest::Client;
use serde_json::Value;

use crate::api::ApiError;
use crate::services::external::types::{
    clamp_probability, is_binary_yes_no, now_rfc3339, price_to_bps, ExternalMarketSnapshot,
    ExternalOrderBookLevel, ExternalOrderBookSnapshot, ExternalOutcome, ExternalTradeSnapshot,
    ExternalTradesSnapshot,
};

fn api_error(prefix: &str, err: impl ToString) -> ApiError {
    ApiError::internal(&format!("{}: {}", prefix, err.to_string()))
}

fn parse_string(value: Option<&Value>) -> String {
    value
        .and_then(|entry| entry.as_str())
        .unwrap_or_default()
        .to_string()
}

fn parse_f64(value: Option<&Value>) -> f64 {
    if let Some(raw) = value {
        if let Some(number) = raw.as_f64() {
            return number;
        }
        if let Some(raw_str) = raw.as_str() {
            if let Ok(number) = raw_str.parse::<f64>() {
                return number;
            }
        }
    }
    0.0
}

fn parse_u64(value: Option<&Value>) -> u64 {
    if let Some(raw) = value {
        if let Some(number) = raw.as_u64() {
            return number;
        }
        if let Some(raw_str) = raw.as_str() {
            if let Ok(number) = raw_str.parse::<u64>() {
                return number;
            }
        }
    }
    0
}

fn clean_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn first_sentence(value: &str, max_length: usize) -> String {
    let compact = clean_text(value);
    if compact.is_empty() {
        return String::new();
    }

    let mut sentence = compact.clone();
    if let Some(position) = compact.find(|ch: char| ['.', '?', '!'].contains(&ch)) {
        sentence = compact[..=position].to_string();
    }
    let sentence = clean_text(sentence.as_str());
    if sentence.chars().count() <= max_length {
        return sentence;
    }

    let truncated: String = sentence
        .chars()
        .take(max_length.saturating_sub(1))
        .collect();
    format!("{}…", truncated.trim_end())
}

fn slug_to_question(slug: &str) -> String {
    let normalized = clean_text(&slug.replace(['-', '_'], " "));
    if normalized.is_empty() {
        "Limitless market".to_string()
    } else {
        normalized
    }
}

fn is_generic_description(value: &str, question: &str, slug: &str) -> bool {
    let normalized = clean_text(value).to_ascii_lowercase();
    if normalized.is_empty() {
        return true;
    }

    let slug_question = slug_to_question(slug).to_ascii_lowercase();
    normalized == clean_text(question).to_ascii_lowercase()
        || normalized == clean_text(slug).to_ascii_lowercase()
        || normalized == slug_question
        || normalized == "limitless market"
}

fn build_limitless_question(
    raw_title: Option<&Value>,
    raw_proxy_title: Option<&Value>,
    slug: &str,
) -> String {
    let title = clean_text(parse_string(raw_title).as_str());
    if !title.is_empty() {
        return title;
    }

    let proxy_title = clean_text(parse_string(raw_proxy_title).as_str());
    if !proxy_title.is_empty() {
        return proxy_title;
    }

    slug_to_question(slug)
}

fn build_limitless_description(
    raw_description: Option<&Value>,
    raw_proxy_title: Option<&Value>,
    question: &str,
    slug: &str,
    close_time_secs: u64,
) -> String {
    let description = clean_text(parse_string(raw_description).as_str());
    if !description.is_empty() && !is_generic_description(description.as_str(), question, slug) {
        return first_sentence(description.as_str(), 420);
    }

    let proxy = clean_text(parse_string(raw_proxy_title).as_str());
    if !proxy.is_empty() && !is_generic_description(proxy.as_str(), question, slug) {
        return first_sentence(proxy.as_str(), 320);
    }

    if close_time_secs > 0 {
        if let Some(close_time) =
            chrono::DateTime::<chrono::Utc>::from_timestamp(close_time_secs as i64, 0)
        {
            return format!(
                "Binary prediction market on Limitless for \"{}\". Trading ends {}.",
                question,
                close_time.to_rfc3339()
            );
        }
    }

    format!(
        "Binary prediction market on Limitless for \"{}\".",
        question
    )
}

fn millis_to_secs(value: u64) -> u64 {
    if value > 100_000_000_000 {
        value / 1000
    } else {
        value
    }
}

fn parse_prices(entry: &Value) -> (f64, f64) {
    if let Some(prices) = entry.get("prices").and_then(|raw| raw.as_array()) {
        let yes = clamp_probability(parse_f64(prices.first()));
        let no = clamp_probability(parse_f64(prices.get(1)));
        if yes > 0.0 || no > 0.0 {
            return (yes, no);
        }
    }
    (0.5, 0.5)
}

fn parse_limitless_market(entry: &Value) -> Option<ExternalMarketSnapshot> {
    let slug = parse_string(entry.get("slug"));
    if slug.is_empty() {
        return None;
    }

    let close_time = millis_to_secs(parse_u64(entry.get("expirationTimestamp")));
    let question =
        build_limitless_question(entry.get("title"), entry.get("proxyTitle"), slug.as_str());
    let description = build_limitless_description(
        entry.get("description"),
        entry.get("proxyTitle"),
        question.as_str(),
        slug.as_str(),
        close_time,
    );
    let category = entry
        .get("categories")
        .and_then(|value| value.as_array())
        .and_then(|values| values.first())
        .and_then(|value| value.as_str())
        .unwrap_or("external")
        .to_ascii_lowercase();

    let resolved = parse_string(entry.get("status")).eq_ignore_ascii_case("resolved")
        || parse_u64(entry.get("winningOutcomeIndex")) <= 1
            && entry.get("winningOutcomeIndex").is_some();

    let outcome = match entry
        .get("winningOutcomeIndex")
        .and_then(|value| value.as_u64())
    {
        Some(0) => Some("yes".to_string()),
        Some(1) => Some("no".to_string()),
        _ => None,
    };

    let (yes_price, no_price) = parse_prices(entry);
    let volume = parse_f64(entry.get("volume"));
    let outcomes = vec![
        ExternalOutcome {
            label: "Yes".to_string(),
            probability: yes_price,
        },
        ExternalOutcome {
            label: "No".to_string(),
            probability: no_price,
        },
    ];
    let executable = is_binary_yes_no(&outcomes);

    Some(ExternalMarketSnapshot {
        id: format!("limitless:{}", slug),
        question,
        description,
        category,
        status: parse_string(entry.get("status")).to_ascii_lowercase(),
        close_time,
        resolved,
        outcome,
        yes_price,
        no_price,
        volume,
        source: "external_limitless".to_string(),
        provider: "limitless".to_string(),
        is_external: true,
        external_url: format!("https://limitless.exchange/markets/{}", slug),
        chain_id: 8453,
        requires_credentials: true,
        execution_users: executable,
        execution_agents: executable,
        outcomes,
        provider_market_ref: parse_string(entry.get("id")),
    })
}

fn parse_orderbook_levels(values: Option<&Value>) -> Vec<ExternalOrderBookLevel> {
    let mut levels = Vec::new();
    let Some(rows) = values.and_then(|value| value.as_array()) else {
        return levels;
    };

    for row in rows {
        let price = clamp_probability(parse_f64(row.get("price")));
        let quantity = parse_f64(row.get("size")).max(0.0);
        if price <= 0.0 || quantity <= 0.0 {
            continue;
        }

        levels.push(ExternalOrderBookLevel {
            price,
            quantity,
            orders: parse_u64(row.get("count")).max(1),
        });
    }

    levels
}

pub async fn fetch_active_markets(
    client: &Client,
    api_base: &str,
    limit: u64,
    offset: u64,
) -> Result<Vec<ExternalMarketSnapshot>, ApiError> {
    let safe_limit = limit.clamp(1, 250);
    let page = (offset / safe_limit) + 1;
    let url = format!(
        "{}/markets/active?limit={}&page={}",
        api_base.trim_end_matches('/'),
        safe_limit,
        page
    );

    let payload = client
        .get(url)
        .send()
        .await
        .map_err(|err| api_error("limitless active markets request failed", err))?
        .error_for_status()
        .map_err(|err| api_error("limitless active markets response failed", err))?
        .json::<Value>()
        .await
        .map_err(|err| api_error("limitless active markets payload invalid", err))?;

    let mut markets = Vec::new();
    if let Some(data) = payload.get("data").and_then(|value| value.as_array()) {
        for row in data {
            if let Some(market) = parse_limitless_market(row) {
                markets.push(market);
            }
        }
    }

    Ok(markets)
}

pub async fn fetch_market_by_slug(
    client: &Client,
    api_base: &str,
    slug: &str,
) -> Result<ExternalMarketSnapshot, ApiError> {
    let url = format!("{}/markets/{}", api_base.trim_end_matches('/'), slug.trim());

    let payload = client
        .get(url)
        .send()
        .await
        .map_err(|err| api_error("limitless market request failed", err))?
        .error_for_status()
        .map_err(|err| api_error("limitless market response failed", err))?
        .json::<Value>()
        .await
        .map_err(|err| api_error("limitless market payload invalid", err))?;

    parse_limitless_market(&payload).ok_or_else(|| {
        ApiError::bad_request(
            "LIMITLESS_MARKET_PARSE_FAILED",
            "failed to parse Limitless market payload",
        )
    })
}

pub async fn fetch_orderbook(
    client: &Client,
    api_base: &str,
    slug: &str,
    outcome: &str,
    depth: u64,
) -> Result<ExternalOrderBookSnapshot, ApiError> {
    let url = format!(
        "{}/markets/{}/orderbook",
        api_base.trim_end_matches('/'),
        slug.trim()
    );

    let payload = client
        .get(url)
        .send()
        .await
        .map_err(|err| api_error("limitless orderbook request failed", err))?
        .error_for_status()
        .map_err(|err| api_error("limitless orderbook response failed", err))?
        .json::<Value>()
        .await
        .map_err(|err| api_error("limitless orderbook payload invalid", err))?;

    let mut bids = parse_orderbook_levels(payload.get("bids"));
    let mut asks = parse_orderbook_levels(payload.get("asks"));
    bids.truncate(depth as usize);
    asks.truncate(depth as usize);

    Ok(ExternalOrderBookSnapshot {
        market_id: format!("limitless:{}", slug),
        outcome: outcome.to_string(),
        bids,
        asks,
        last_updated: now_rfc3339(),
        source: "external_limitless".to_string(),
        provider: "limitless".to_string(),
        chain_id: 8453,
        provider_market_ref: parse_string(payload.get("tokenId")),
        is_synthetic: false,
    })
}

pub async fn fetch_trades(
    client: &Client,
    api_base: &str,
    slug: &str,
    outcome_filter: Option<&str>,
    limit: u64,
    offset: u64,
) -> Result<ExternalTradesSnapshot, ApiError> {
    let safe_limit = limit.clamp(1, 200);
    let page = (offset / safe_limit) + 1;
    let url = format!(
        "{}/markets/{}/events?limit={}&page={}",
        api_base.trim_end_matches('/'),
        slug.trim(),
        safe_limit,
        page
    );

    let payload = client
        .get(url)
        .send()
        .await
        .map_err(|err| api_error("limitless events request failed", err))?
        .error_for_status()
        .map_err(|err| api_error("limitless events response failed", err))?
        .json::<Value>()
        .await
        .map_err(|err| api_error("limitless events payload invalid", err))?;

    let mut trades = Vec::new();
    if let Some(events) = payload.get("events").and_then(|value| value.as_array()) {
        for (index, event) in events.iter().enumerate() {
            let side = event
                .get("side")
                .and_then(|value| value.as_i64())
                .unwrap_or(0);
            let inferred_outcome = if side == 1 { "no" } else { "yes" };
            if let Some(filter) = outcome_filter {
                if filter != inferred_outcome {
                    continue;
                }
            }

            let price = clamp_probability(parse_f64(event.get("price")));
            let quantity_raw = parse_f64(event.get("matchedSize")).max(0.0);
            let quantity = quantity_raw.round().clamp(0.0, u64::MAX as f64) as u64;
            let id = parse_string(event.get("id"));

            trades.push(ExternalTradeSnapshot {
                id: if id.is_empty() {
                    format!("limitless:{}:{}", slug, index)
                } else {
                    format!("limitless:{}", id)
                },
                market_id: format!("limitless:{}", slug),
                outcome: inferred_outcome.to_string(),
                price,
                price_bps: price_to_bps(price),
                quantity,
                tx_hash: parse_string(event.get("transactionHash")),
                block_number: parse_u64(event.get("blockNumber")),
                created_at: parse_string(event.get("createdAt")),
            });
        }
    }

    let total = trades.len() as u64;
    let has_more = total >= safe_limit;

    Ok(ExternalTradesSnapshot {
        trades,
        total,
        limit: safe_limit,
        offset,
        has_more,
        source: "external_limitless".to_string(),
        provider: "limitless".to_string(),
        chain_id: 8453,
        provider_market_ref: slug.to_string(),
        is_synthetic: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_limitless_market_uses_slug_when_title_missing() {
        let payload = json!({
            "slug": "btc-above-100k",
            "description": "",
            "categories": ["crypto"],
            "expirationTimestamp": 1893456000000u64,
            "status": "active",
            "prices": [0.61, 0.39],
            "volume": 51234.0
        });

        let market = parse_limitless_market(&payload).expect("market");
        assert_eq!(market.question, "btc above 100k");
        assert!(market
            .description
            .contains("Binary prediction market on Limitless"));
    }

    #[test]
    fn parse_limitless_market_discards_generic_description() {
        let payload = json!({
            "slug": "eth-new-ath",
            "title": "ETH new ATH",
            "description": "ETH new ATH",
            "proxyTitle": "ETH new ATH",
            "categories": ["crypto"],
            "expirationTimestamp": 1893456000000u64,
            "status": "active",
            "prices": [0.4, 0.6],
            "volume": 12000.0
        });

        let market = parse_limitless_market(&payload).expect("market");
        assert_ne!(market.description, "ETH new ATH");
        assert!(market
            .description
            .contains("Binary prediction market on Limitless"));
    }

    #[test]
    fn parse_limitless_market_keeps_first_sentence_only() {
        let payload = json!({
            "slug": "sol-through-300",
            "title": "SOL through 300",
            "description": "First sentence. Second sentence should not be present.",
            "categories": ["crypto"],
            "expirationTimestamp": 1893456000000u64,
            "status": "active",
            "prices": [0.5, 0.5],
            "volume": 9999.0
        });

        let market = parse_limitless_market(&payload).expect("market");
        assert_eq!(market.description, "First sentence.");
    }
}
