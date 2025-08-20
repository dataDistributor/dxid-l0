//! Standardized API response structures
//! 
//! This module provides consistent response formats for all API endpoints

use axum::{
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use super::errors::ApiError;

/// Standard API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ApiErrorResponse>,
    pub meta: Option<ResponseMeta>,
}

/// API error response
#[derive(Debug, Serialize)]
pub struct ApiErrorResponse {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

/// Response metadata
#[derive(Debug, Serialize)]
pub struct ResponseMeta {
    pub timestamp: u64,
    pub request_id: String,
    pub version: String,
}

/// Success response helper
pub fn success_response<T>(data: T) -> ApiResponse<T> {
    ApiResponse {
        success: true,
        data: Some(data),
        error: None,
        meta: Some(ResponseMeta {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            request_id: uuid::Uuid::new_v4().to_string(),
            version: "v1".to_string(),
        }),
    }
}

/// Error response helper
pub fn error_response(error: ApiError) -> ApiResponse<()> {
    let (status_code, error_code) = match &error {
        ApiError::Validation(_) => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR"),
        ApiError::Authentication(_) => (StatusCode::UNAUTHORIZED, "AUTHENTICATION_ERROR"),
        ApiError::Authorization(_) => (StatusCode::FORBIDDEN, "AUTHORIZATION_ERROR"),
        ApiError::NotFound(_) => (StatusCode::NOT_FOUND, "NOT_FOUND"),
        ApiError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "RATE_LIMITED"),
        ApiError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
        ApiError::ServiceUnavailable(_) => (StatusCode::SERVICE_UNAVAILABLE, "SERVICE_UNAVAILABLE"),
    };

    ApiResponse {
        success: false,
        data: None,
        error: Some(ApiErrorResponse {
            code: error_code.to_string(),
            message: error.to_string(),
            details: None,
        }),
        meta: Some(ResponseMeta {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            request_id: uuid::Uuid::new_v4().to_string(),
            version: "v1".to_string(),
        }),
    }
}

/// Convert ApiError to HTTP response
impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let response = error_response(self);
        Json(response).into_response()
    }
}

/// Blockchain status response
#[derive(Debug, Serialize)]
pub struct BlockchainStatus {
    pub height: u64,
    pub last_block_hash: String,
    pub state_root: String,
    pub chain_id: u32,
    pub block_time: u64,
    pub total_transactions: u64,
}

/// Account balance response
#[derive(Debug, Serialize)]
pub struct AccountBalance {
    pub address: String,
    pub exists: bool,
    pub balance: String,
    pub nonce: u64,
    pub layer0_balance: String,
    pub longyield_balance: String,
}

/// Block response
#[derive(Debug, Serialize)]
pub struct BlockResponse {
    pub height: u64,
    pub hash: String,
    pub parent_hash: String,
    pub timestamp: u64,
    pub transactions: Vec<TransactionResponse>,
    pub state_root: String,
    pub tx_root: String,
}

/// Transaction response
#[derive(Debug, Serialize)]
pub struct TransactionResponse {
    pub hash: String,
    pub from: String,
    pub to: String,
    pub amount: String,
    pub fee: String,
    pub nonce: u64,
    pub timestamp: u64,
    pub status: String,
}

/// Transaction submission response
#[derive(Debug, Serialize)]
pub struct TransactionSubmissionResponse {
    pub success: bool,
    pub transaction_hash: String,
    pub queued: bool,
    pub file_path: String,
    pub estimated_confirmation_time: Option<u64>,
}

/// API key response
#[derive(Debug, Serialize)]
pub struct ApiKeyResponse {
    pub id: String,
    pub name: String,
    pub created_at: u64,
    pub enabled: bool,
    pub permissions: Vec<String>,
    pub last_used: Option<u64>,
}

/// Webhook response
#[derive(Debug, Serialize)]
pub struct WebhookResponse {
    pub id: String,
    pub url: String,
    pub events: Vec<String>,
    pub created_at: u64,
    pub enabled: bool,
    pub secret: Option<String>, // Only shown on creation
}

/// Network status response
#[derive(Debug, Serialize)]
pub struct NetworkStatusResponse {
    pub auto_discovery_enabled: bool,
    pub p2p_enabled: bool,
    pub chain_id: u32,
    pub peer_count: usize,
    pub discovery_active: bool,
    pub total_peers: usize,
    pub bootstrap_peers: usize,
    pub network_latency_ms: Option<u64>,
}

/// Proof response
#[derive(Debug, Serialize)]
pub struct ProofResponse {
    pub proof_type: String,
    pub proof_data: String,
    pub public_inputs: Vec<String>,
    pub verification_key: Option<String>,
    pub created_at: u64,
    pub expires_at: Option<u64>,
}
