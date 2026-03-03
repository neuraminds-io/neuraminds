pub mod credentials;
pub mod providers;
pub mod types;

use reqwest::Client;

use crate::api::ApiError;
use crate::config::AppConfig;
use crate::services::redis::RedisService;

use self::providers::{limitless, polymarket};
use self::types::{
    ExternalMarketId, ExternalMarketSnapshot, ExternalOrderBookSnapshot, ExternalProvider,
    ExternalTradesSnapshot,
};

const MARKET_CACHE_TTL_SECONDS: u64 = 30;
const ORDERBOOK_CACHE_TTL_SECONDS: u64 = 5;
const TRADES_CACHE_TTL_SECONDS: u64 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalMarketSource {
    All,
    Internal,
    Limitless,
    Polymarket,
}

impl ExternalMarketSource {
    pub fn from_query(value: Option<&str>) -> Result<Self, ApiError> {
        match value.unwrap_or("all").trim().to_ascii_lowercase().as_str() {
            "all" => Ok(Self::All),
            "internal" => Ok(Self::Internal),
            "limitless" => Ok(Self::Limitless),
            "polymarket" => Ok(Self::Polymarket),
            _ => Err(ApiError::bad_request(
                "INVALID_SOURCE",
                "source must be one of: all, internal, limitless, polymarket",
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradableFilter {
    All,
    User,
    Agent,
}

impl TradableFilter {
    pub fn from_query(value: Option<&str>) -> Result<Self, ApiError> {
        match value.unwrap_or("all").trim().to_ascii_lowercase().as_str() {
            "all" => Ok(Self::All),
            "user" => Ok(Self::User),
            "agent" => Ok(Self::Agent),
            _ => Err(ApiError::bad_request(
                "INVALID_TRADABLE_FILTER",
                "tradable must be one of: all, user, agent",
            )),
        }
    }
}

fn http_client() -> Result<Client, ApiError> {
    Client::builder()
        .timeout(std::time::Duration::from_secs(12))
        .build()
        .map_err(|err| {
            ApiError::internal(&format!("failed to build external http client: {}", err))
        })
}

fn include_market(filter: TradableFilter, market: &ExternalMarketSnapshot) -> bool {
    match filter {
        TradableFilter::All => true,
        TradableFilter::User => market.execution_users,
        TradableFilter::Agent => market.execution_agents,
    }
}

fn with_retries<T, F, Fut>(mut action: F) -> impl std::future::Future<Output = Result<T, ApiError>>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, ApiError>>,
{
    async move {
        let mut last_error: Option<ApiError> = None;
        for attempt in 0..3 {
            match action().await {
                Ok(value) => return Ok(value),
                Err(err) => {
                    last_error = Some(err);
                    if attempt < 2 {
                        tokio::time::sleep(std::time::Duration::from_millis(
                            200 * (attempt + 1) as u64,
                        ))
                        .await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| ApiError::internal("external provider request failed")))
    }
}

pub async fn fetch_markets(
    config: &AppConfig,
    redis: &RedisService,
    source: ExternalMarketSource,
    tradable_filter: TradableFilter,
    limit: u64,
    offset: u64,
) -> Result<Vec<ExternalMarketSnapshot>, ApiError> {
    if !config.external_markets_enabled {
        return Ok(Vec::new());
    }

    let cache_key = format!(
        "external:markets:{}:{}:{}:{}",
        match source {
            ExternalMarketSource::All => "all",
            ExternalMarketSource::Internal => "internal",
            ExternalMarketSource::Limitless => "limitless",
            ExternalMarketSource::Polymarket => "polymarket",
        },
        match tradable_filter {
            TradableFilter::All => "all",
            TradableFilter::User => "user",
            TradableFilter::Agent => "agent",
        },
        limit,
        offset
    );

    if let Ok(Some(cached)) = redis.get::<Vec<ExternalMarketSnapshot>>(&cache_key).await {
        return Ok(cached);
    }

    let client = http_client()?;
    let mut markets = Vec::new();

    if matches!(
        source,
        ExternalMarketSource::All | ExternalMarketSource::Limitless
    ) && config.limitless_enabled
    {
        let fetched = with_retries(|| {
            limitless::fetch_active_markets(
                &client,
                config.limitless_api_base.as_str(),
                limit.max(50),
                offset,
            )
        })
        .await?;
        markets.extend(
            fetched
                .into_iter()
                .filter(|entry| include_market(tradable_filter, entry)),
        );
    }

    if matches!(
        source,
        ExternalMarketSource::All | ExternalMarketSource::Polymarket
    ) && config.polymarket_enabled
    {
        let fetched = with_retries(|| {
            polymarket::fetch_active_markets(
                &client,
                config.polymarket_gamma_api_base.as_str(),
                limit.max(50),
                offset,
            )
        })
        .await?;
        markets.extend(
            fetched
                .into_iter()
                .filter(|entry| include_market(tradable_filter, entry)),
        );
    }

    markets.sort_by(|a, b| {
        b.close_time
            .cmp(&a.close_time)
            .then_with(|| a.id.cmp(&b.id))
    });

    let _ = redis
        .set(&cache_key, &markets, Some(MARKET_CACHE_TTL_SECONDS))
        .await;

    Ok(markets)
}

pub async fn fetch_market_by_id(
    config: &AppConfig,
    market_id: &ExternalMarketId,
) -> Result<ExternalMarketSnapshot, ApiError> {
    if !config.external_markets_enabled {
        return Err(ApiError::bad_request(
            "EXTERNAL_MARKETS_DISABLED",
            "external market integration is disabled",
        ));
    }

    let client = http_client()?;
    match market_id.provider {
        ExternalProvider::Limitless => {
            if !config.limitless_enabled {
                return Err(ApiError::bad_request(
                    "LIMITLESS_DISABLED",
                    "Limitless integration is disabled",
                ));
            }
            with_retries(|| {
                limitless::fetch_market_by_slug(
                    &client,
                    config.limitless_api_base.as_str(),
                    market_id.value.as_str(),
                )
            })
            .await
        }
        ExternalProvider::Polymarket => {
            if !config.polymarket_enabled {
                return Err(ApiError::bad_request(
                    "POLYMARKET_DISABLED",
                    "Polymarket integration is disabled",
                ));
            }
            with_retries(|| {
                polymarket::fetch_market_by_id(
                    &client,
                    config.polymarket_gamma_api_base.as_str(),
                    market_id.value.as_str(),
                )
            })
            .await
        }
    }
}

pub async fn fetch_orderbook(
    config: &AppConfig,
    redis: &RedisService,
    market_id: &ExternalMarketId,
    outcome: &str,
    depth: u64,
) -> Result<ExternalOrderBookSnapshot, ApiError> {
    if !config.external_markets_enabled {
        return Err(ApiError::bad_request(
            "EXTERNAL_MARKETS_DISABLED",
            "external market integration is disabled",
        ));
    }

    let cache_key = format!(
        "external:orderbook:{}:{}:{}",
        market_id.full_id(),
        outcome,
        depth
    );
    if let Ok(Some(cached)) = redis.get::<ExternalOrderBookSnapshot>(&cache_key).await {
        return Ok(cached);
    }

    let client = http_client()?;
    let value = match market_id.provider {
        ExternalProvider::Limitless => {
            with_retries(|| {
                limitless::fetch_orderbook(
                    &client,
                    config.limitless_api_base.as_str(),
                    market_id.value.as_str(),
                    outcome,
                    depth,
                )
            })
            .await?
        }
        ExternalProvider::Polymarket => {
            with_retries(|| {
                polymarket::fetch_orderbook(
                    &client,
                    config.polymarket_gamma_api_base.as_str(),
                    config.polymarket_clob_api_base.as_str(),
                    market_id.value.as_str(),
                    outcome,
                    depth,
                )
            })
            .await?
        }
    };

    let _ = redis
        .set(&cache_key, &value, Some(ORDERBOOK_CACHE_TTL_SECONDS))
        .await;

    Ok(value)
}

pub async fn fetch_trades(
    config: &AppConfig,
    redis: &RedisService,
    market_id: &ExternalMarketId,
    outcome: Option<&str>,
    limit: u64,
    offset: u64,
) -> Result<ExternalTradesSnapshot, ApiError> {
    if !config.external_markets_enabled {
        return Err(ApiError::bad_request(
            "EXTERNAL_MARKETS_DISABLED",
            "external market integration is disabled",
        ));
    }

    let cache_key = format!(
        "external:trades:{}:{}:{}:{}",
        market_id.full_id(),
        outcome.unwrap_or("all"),
        limit,
        offset
    );
    if let Ok(Some(cached)) = redis.get::<ExternalTradesSnapshot>(&cache_key).await {
        return Ok(cached);
    }

    let client = http_client()?;
    let value = match market_id.provider {
        ExternalProvider::Limitless => {
            with_retries(|| {
                limitless::fetch_trades(
                    &client,
                    config.limitless_api_base.as_str(),
                    market_id.value.as_str(),
                    outcome,
                    limit,
                    offset,
                )
            })
            .await?
        }
        ExternalProvider::Polymarket => {
            with_retries(|| {
                polymarket::fetch_trades(
                    &client,
                    config.polymarket_gamma_api_base.as_str(),
                    config.polymarket_clob_api_base.as_str(),
                    market_id.value.as_str(),
                    outcome,
                    limit,
                    offset,
                )
            })
            .await?
        }
    };

    let _ = redis
        .set(&cache_key, &value, Some(TRADES_CACHE_TTL_SECONDS))
        .await;

    Ok(value)
}
