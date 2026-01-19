//! Metrics and observability service for Polyguard backend
//!
//! Provides application metrics, health checks, and monitoring data.

use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Application metrics collector
pub struct MetricsService {
    start_time: Instant,
    requests_total: AtomicU64,
    requests_success: AtomicU64,
    requests_error: AtomicU64,
    orders_placed: AtomicU64,
    orders_cancelled: AtomicU64,
    trades_executed: AtomicU64,
    total_volume: AtomicU64,
}

impl MetricsService {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            requests_total: AtomicU64::new(0),
            requests_success: AtomicU64::new(0),
            requests_error: AtomicU64::new(0),
            orders_placed: AtomicU64::new(0),
            orders_cancelled: AtomicU64::new(0),
            trades_executed: AtomicU64::new(0),
            total_volume: AtomicU64::new(0),
        }
    }

    /// Record an incoming request
    pub fn record_request(&self) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a successful request
    pub fn record_success(&self) {
        self.requests_success.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a failed request
    pub fn record_error(&self) {
        self.requests_error.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an order placement
    pub fn record_order_placed(&self) {
        self.orders_placed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an order cancellation
    pub fn record_order_cancelled(&self) {
        self.orders_cancelled.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a trade execution with volume
    pub fn record_trade(&self, volume: u64) {
        self.trades_executed.fetch_add(1, Ordering::Relaxed);
        self.total_volume.fetch_add(volume, Ordering::Relaxed);
    }

    /// Get current metrics snapshot
    pub fn get_metrics(&self) -> AppMetrics {
        AppMetrics {
            uptime_seconds: self.start_time.elapsed().as_secs(),
            requests: RequestMetrics {
                total: self.requests_total.load(Ordering::Relaxed),
                success: self.requests_success.load(Ordering::Relaxed),
                error: self.requests_error.load(Ordering::Relaxed),
            },
            orders: OrderMetrics {
                placed: self.orders_placed.load(Ordering::Relaxed),
                cancelled: self.orders_cancelled.load(Ordering::Relaxed),
            },
            trades: TradeMetrics {
                executed: self.trades_executed.load(Ordering::Relaxed),
                total_volume: self.total_volume.load(Ordering::Relaxed),
            },
        }
    }

    /// Export metrics in Prometheus format
    pub fn export_prometheus(&self) -> String {
        let metrics = self.get_metrics();
        let mut output = String::new();

        // Uptime
        output.push_str("# HELP polyguard_uptime_seconds Seconds since service start\n");
        output.push_str("# TYPE polyguard_uptime_seconds gauge\n");
        output.push_str(&format!("polyguard_uptime_seconds {}\n\n", metrics.uptime_seconds));

        // Requests
        output.push_str("# HELP polyguard_requests_total Total HTTP requests\n");
        output.push_str("# TYPE polyguard_requests_total counter\n");
        output.push_str(&format!("polyguard_requests_total{{status=\"success\"}} {}\n", metrics.requests.success));
        output.push_str(&format!("polyguard_requests_total{{status=\"error\"}} {}\n\n", metrics.requests.error));

        // Orders
        output.push_str("# HELP polyguard_orders_total Total orders processed\n");
        output.push_str("# TYPE polyguard_orders_total counter\n");
        output.push_str(&format!("polyguard_orders_total{{action=\"placed\"}} {}\n", metrics.orders.placed));
        output.push_str(&format!("polyguard_orders_total{{action=\"cancelled\"}} {}\n\n", metrics.orders.cancelled));

        // Trades
        output.push_str("# HELP polyguard_trades_total Total trades executed\n");
        output.push_str("# TYPE polyguard_trades_total counter\n");
        output.push_str(&format!("polyguard_trades_total {}\n\n", metrics.trades.executed));

        // Volume
        output.push_str("# HELP polyguard_volume_total Total trading volume in lamports\n");
        output.push_str("# TYPE polyguard_volume_total counter\n");
        output.push_str(&format!("polyguard_volume_total {}\n", metrics.trades.total_volume));

        output
    }
}

impl Default for MetricsService {
    fn default() -> Self {
        Self::new()
    }
}

/// Application metrics snapshot
#[derive(Debug, Clone, Serialize)]
pub struct AppMetrics {
    pub uptime_seconds: u64,
    pub requests: RequestMetrics,
    pub orders: OrderMetrics,
    pub trades: TradeMetrics,
}

#[derive(Debug, Clone, Serialize)]
pub struct RequestMetrics {
    pub total: u64,
    pub success: u64,
    pub error: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrderMetrics {
    pub placed: u64,
    pub cancelled: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TradeMetrics {
    pub executed: u64,
    pub total_volume: u64,
}

/// System health information
#[derive(Debug, Clone, Serialize)]
pub struct SystemHealth {
    pub status: HealthStatus,
    pub version: &'static str,
    pub uptime_seconds: u64,
    pub checks: HealthChecks,
}

/// Health check results for all components
#[derive(Debug, Clone, Serialize)]
pub struct HealthChecks {
    pub database: ComponentHealth,
    pub redis: ComponentHealth,
    pub solana: ComponentHealth,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComponentHealth {
    pub status: HealthStatus,
    pub latency_ms: Option<u64>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

impl ComponentHealth {
    pub fn healthy(latency_ms: u64) -> Self {
        Self {
            status: HealthStatus::Healthy,
            latency_ms: Some(latency_ms),
            message: None,
        }
    }

    pub fn degraded(latency_ms: u64, message: &str) -> Self {
        Self {
            status: HealthStatus::Degraded,
            latency_ms: Some(latency_ms),
            message: Some(message.to_string()),
        }
    }

    pub fn unhealthy(message: &str) -> Self {
        Self {
            status: HealthStatus::Unhealthy,
            latency_ms: None,
            message: Some(message.to_string()),
        }
    }
}

/// Request timing helper
pub struct RequestTimer {
    start: Instant,
}

impl RequestTimer {
    pub fn start() -> Self {
        Self { start: Instant::now() }
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_increment() {
        let metrics = MetricsService::new();

        metrics.record_request();
        metrics.record_request();
        metrics.record_success();
        metrics.record_error();

        let snapshot = metrics.get_metrics();
        assert_eq!(snapshot.requests.total, 2);
        assert_eq!(snapshot.requests.success, 1);
        assert_eq!(snapshot.requests.error, 1);
    }

    #[test]
    fn test_trade_volume() {
        let metrics = MetricsService::new();

        metrics.record_trade(1000);
        metrics.record_trade(2000);

        let snapshot = metrics.get_metrics();
        assert_eq!(snapshot.trades.executed, 2);
        assert_eq!(snapshot.trades.total_volume, 3000);
    }

    #[test]
    fn test_prometheus_export() {
        let metrics = MetricsService::new();
        metrics.record_request();
        metrics.record_success();

        let output = metrics.export_prometheus();
        assert!(output.contains("polyguard_requests_total"));
        assert!(output.contains("polyguard_uptime_seconds"));
    }
}
