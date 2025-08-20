//! Production-ready API implementation for dxID node
//! 
//! This module provides:
//! - Standardized API responses
//! - Enhanced security with rate limiting and JWT auth
//! - Input validation
//! - Structured logging and metrics
//! - API versioning

pub mod auth;
pub mod config;
pub mod errors;
pub mod middleware;
pub mod responses;
pub mod routes;
pub mod validation;

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, instrument};

use crate::RpcCtx;

// Re-export main components
pub use auth::*;
pub use config::*;
pub use errors::*;
pub use middleware::*;
pub use responses::*;
pub use routes::*;
pub use validation::*;

/// Create the production-ready API router
pub fn create_api_router(ctx: Arc<RpcCtx>) -> Router {
    Router::new()
        .nest("/api/v1", v1_routes())
        .nest("/admin", admin_routes())
        .route("/health", get(health_check))
        .route("/metrics", get(prometheus_metrics))
        .with_state(ctx)
        .layer(axum::middleware::from_fn(logging_middleware))
        .layer(axum::middleware::from_fn(cors_middleware))
}

/// Enhanced health check endpoint
#[instrument(skip(ctx))]
async fn health_check(
    State(ctx): State<Arc<RpcCtx>>,
) -> Result<Json<ApiResponse<HealthStatus>>, ApiError> {
    let start_time = std::time::Instant::now();
    
    // Check blockchain state
    let blockchain_healthy = {
        let state = ctx.state.lock();
        state.height > 0
    };
    
    // Check storage
    let storage_healthy = std::path::Path::new("./dxid-data").exists();
    
    // Check P2P network
    let p2p_healthy = crate::P2P_NET.get().is_some();
    
    let overall_health = blockchain_healthy && storage_healthy;
    
    let duration = start_time.elapsed();
    info!(
        duration_ms = duration.as_millis(),
        blockchain_healthy,
        storage_healthy,
        p2p_healthy,
        "Health check completed"
    );
    
    Ok(Json(ApiResponse {
        success: overall_health,
        data: Some(HealthStatus {
            status: if overall_health { "healthy" } else { "unhealthy" }.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            checks: HealthChecks {
                blockchain: blockchain_healthy,
                storage: storage_healthy,
                p2p_network: p2p_healthy,
            },
            version: env!("CARGO_PKG_VERSION").to_string(),
        }),
        error: None,
        meta: Some(ResponseMeta {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            request_id: uuid::Uuid::new_v4().to_string(),
            version: "v1".to_string(),
        }),
    }))
}

/// Prometheus metrics endpoint
async fn prometheus_metrics(
    State(ctx): State<Arc<RpcCtx>>,
) -> Result<String, ApiError> {
    use prometheus::{Encoder, TextEncoder};
    
    let encoder = TextEncoder::new();
    let mut buffer = Vec::new();
    
    // Collect metrics
    let metric_families = prometheus::gather();
    
    // Encode to Prometheus format
    encoder.encode(&metric_families, &mut buffer)
        .map_err(|e| ApiError::Internal(format!("Failed to encode metrics: {}", e)))?;
    
    Ok(String::from_utf8(buffer)
        .map_err(|e| ApiError::Internal(format!("Failed to convert metrics to string: {}", e)))?)
}

#[derive(Debug, Serialize)]
struct HealthStatus {
    status: String,
    timestamp: u64,
    checks: HealthChecks,
    version: String,
}

#[derive(Debug, Serialize)]
struct HealthChecks {
    blockchain: bool,
    storage: bool,
    p2p_network: bool,
}
