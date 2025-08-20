//! API middleware
//! 
//! This module provides middleware for logging, CORS, rate limiting, and authentication

use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn, error, instrument};

use super::errors::ApiError;

/// Rate limiting state
#[derive(Clone)]
pub struct RateLimitState {
    pub requests_per_minute: u32,
    pub burst_size: u32,
}

/// In-memory rate limiter (for production, use Redis or similar)
#[derive(Clone)]
pub struct RateLimiter {
    requests: Arc<RwLock<HashMap<String, Vec<Instant>>>>,
    config: RateLimitState,
}

impl RateLimiter {
    pub fn new(config: RateLimitState) -> Self {
        Self {
            requests: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }
    
    pub async fn check_rate_limit(&self, key: &str) -> bool {
        let mut requests = self.requests.write().await;
        let now = Instant::now();
        let window = Duration::from_secs(60);
        
        // Clean old requests
        if let Some(timestamps) = requests.get_mut(key) {
            timestamps.retain(|&timestamp| now.duration_since(timestamp) < window);
            
            // Check if within limits
            if timestamps.len() >= self.config.requests_per_minute as usize {
                return false;
            }
            
            timestamps.push(now);
        } else {
            requests.insert(key.to_string(), vec![now]);
        }
        
        true
    }
}

/// Logging middleware
#[instrument(skip(request, next))]
pub async fn logging_middleware(
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let start_time = Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();
    let headers = request.headers().clone();
    
    // Extract request ID
    let request_id = headers
        .get("X-Request-ID")
        .and_then(|h| h.to_str().ok())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    
    info!(
        request_id = %request_id,
        method = %method,
        uri = %uri,
        "Request started"
    );
    
    // Process request
    let response = next.run(request).await;
    
    let duration = start_time.elapsed();
    let status = response.status();
    
    // Log response
    if status.is_success() {
        info!(
            request_id = %request_id,
            method = %method,
            uri = %uri,
            status = %status,
            duration_ms = duration.as_millis(),
            "Request completed successfully"
        );
    } else if status.is_client_error() {
        warn!(
            request_id = %request_id,
            method = %method,
            uri = %uri,
            status = %status,
            duration_ms = duration.as_millis(),
            "Client error"
        );
    } else {
        error!(
            request_id = %request_id,
            method = %method,
            uri = %uri,
            status = %status,
            duration_ms = duration.as_millis(),
            "Server error"
        );
    }
    
    Ok(response)
}

/// CORS middleware
pub fn cors_middleware() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_credentials(false)
        .max_age(Duration::from_secs(3600))
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Extract client identifier (IP + API key)
    let client_id = extract_client_id(&request);
    
    // Check rate limit (this would be injected via state in real implementation)
    // For now, we'll just pass through
    // let rate_limiter = extract_rate_limiter_from_state(&request);
    // if !rate_limiter.check_rate_limit(&client_id).await {
    //     return Err(ApiError::RateLimited);
    // }
    
    next.run(request).await
}

/// Authentication middleware
pub async fn auth_middleware(
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let headers = request.headers();
    
    // Check for API key
    if let Some(api_key) = headers.get("X-Api-Key") {
        if let Ok(key) = api_key.to_str() {
            // Validate API key (this would be injected via state in real implementation)
            // For now, we'll just pass through
            // let api_key_store = extract_api_key_store_from_state(&request);
            // if !api_key_store.validate_key(key).await {
            //     return Err(ApiError::Authentication("Invalid API key".to_string()));
            // }
        }
    }
    
    next.run(request).await
}

/// Admin authentication middleware
pub async fn admin_auth_middleware(
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let headers = request.headers();
    
    // Check for admin token
    if let Some(admin_token) = headers.get("X-Admin-Token") {
        if let Ok(token) = admin_token.to_str() {
            // Validate admin token (this would be injected via state in real implementation)
            // For now, we'll just pass through
            // let admin_token = extract_admin_token_from_state(&request);
            // if token != admin_token {
            //     return Err(ApiError::Authorization("Invalid admin token".to_string()));
            // }
        } else {
            return Err(ApiError::Authentication("Invalid admin token format".to_string()));
        }
    } else {
        return Err(ApiError::Authentication("Missing admin token".to_string()));
    }
    
    next.run(request).await
}

/// Request timeout middleware
pub async fn timeout_middleware(
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let timeout = Duration::from_secs(30);
    
    match tokio::time::timeout(timeout, next.run(request)).await {
        Ok(response) => Ok(response),
        Err(_) => Err(ApiError::ServiceUnavailable("Request timeout".to_string())),
    }
}

/// Extract client identifier for rate limiting
fn extract_client_id(request: &Request) -> String {
    // In a real implementation, this would extract IP + API key
    // For now, just use a placeholder
    "client".to_string()
}

/// Metrics middleware
pub async fn metrics_middleware(
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let start_time = Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();
    
    // Process request
    let response = next.run(request).await;
    
    let duration = start_time.elapsed();
    let status = response.status();
    
    // Update metrics (this would be injected via state in real implementation)
    // let metrics = extract_metrics_from_state(&request);
    // metrics.requests_total.inc();
    // metrics.request_duration.observe(duration.as_secs_f64());
    // if !status.is_success() {
    //     metrics.errors_total.inc();
    // }
    
    Ok(response)
}
