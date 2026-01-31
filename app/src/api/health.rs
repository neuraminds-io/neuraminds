use actix_web::{web, HttpResponse, Responder};
use serde::Serialize;
use std::sync::Arc;
use std::time::Instant;

use crate::AppState;
use crate::services::{
    SystemHealth, HealthStatus, ComponentHealth, HealthChecks,
};

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    timestamp: String,
}

/// Basic health check endpoint (for load balancers)
pub async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

/// Detailed health check with component status
pub async fn health_detailed(
    state: web::Data<Arc<AppState>>,
) -> impl Responder {
    let uptime = state.metrics.get_metrics().uptime_seconds;

    // Check database health
    let db_health = check_database_health(&state).await;

    // Check Redis health
    let redis_health = check_redis_health(&state).await;

    // Check Solana RPC health
    let solana_health = check_solana_health(&state).await;

    // Determine overall status
    let overall_status = determine_overall_status(&db_health, &redis_health, &solana_health);

    let health = SystemHealth {
        status: overall_status,
        version: env!("CARGO_PKG_VERSION"),
        uptime_seconds: uptime,
        checks: HealthChecks {
            database: db_health,
            redis: redis_health,
            solana: solana_health,
        },
    };

    match overall_status {
        HealthStatus::Healthy => HttpResponse::Ok().json(health),
        HealthStatus::Degraded => HttpResponse::Ok().json(health),
        HealthStatus::Unhealthy => HttpResponse::ServiceUnavailable().json(health),
    }
}

/// Get application metrics (JSON format)
pub async fn get_metrics(
    state: web::Data<Arc<AppState>>,
) -> impl Responder {
    let metrics = state.metrics.get_metrics();
    HttpResponse::Ok().json(metrics)
}

/// Get application metrics (Prometheus format)
pub async fn get_metrics_prometheus(
    state: web::Data<Arc<AppState>>,
) -> impl Responder {
    let prometheus_output = state.metrics.export_prometheus();
    HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4")
        .body(prometheus_output)
}

async fn check_database_health(state: &web::Data<Arc<AppState>>) -> ComponentHealth {
    let start = Instant::now();

    // Execute actual query to verify database connectivity
    match sqlx::query("SELECT 1")
        .execute(state.db.pool())
        .await
    {
        Ok(_) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            let stats = state.db.pool_stats();

            if stats.size == 0 {
                ComponentHealth::unhealthy("No database connections available")
            } else if latency_ms > 500 {
                ComponentHealth::degraded(latency_ms, "High query latency")
            } else {
                ComponentHealth::healthy(latency_ms)
            }
        }
        Err(e) => ComponentHealth::unhealthy(&format!("Database query failed: {}", e)),
    }
}

async fn check_redis_health(state: &web::Data<Arc<AppState>>) -> ComponentHealth {
    let start = Instant::now();

    // Try a simple get operation
    match state.redis.get::<String>("health_check").await {
        Ok(_) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            if latency_ms > 100 {
                ComponentHealth::degraded(latency_ms, "High latency")
            } else {
                ComponentHealth::healthy(latency_ms)
            }
        }
        Err(e) => ComponentHealth::unhealthy(&format!("Redis error: {}", e)),
    }
}

async fn check_solana_health(state: &web::Data<Arc<AppState>>) -> ComponentHealth {
    let start = Instant::now();

    // Try to get keeper balance as a health check
    let keeper = state.solana.keeper_pubkey();
    match state.solana.get_balance(&keeper).await {
        Ok(_) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            if latency_ms > 2000 {
                ComponentHealth::degraded(latency_ms, "High RPC latency")
            } else {
                ComponentHealth::healthy(latency_ms)
            }
        }
        Err(e) => ComponentHealth::unhealthy(&format!("Solana RPC error: {}", e)),
    }
}

fn determine_overall_status(
    db: &ComponentHealth,
    redis: &ComponentHealth,
    solana: &ComponentHealth,
) -> HealthStatus {
    // If any critical component is unhealthy, overall is unhealthy
    if db.status == HealthStatus::Unhealthy {
        return HealthStatus::Unhealthy;
    }

    // If any component is degraded, overall is degraded
    if db.status == HealthStatus::Degraded
        || redis.status == HealthStatus::Degraded
        || solana.status == HealthStatus::Degraded
    {
        return HealthStatus::Degraded;
    }

    // Redis/Solana being unhealthy means degraded (not fully down)
    if redis.status == HealthStatus::Unhealthy || solana.status == HealthStatus::Unhealthy {
        return HealthStatus::Degraded;
    }

    HealthStatus::Healthy
}
