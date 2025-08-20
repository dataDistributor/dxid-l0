//! API error handling
//! 
//! This module provides comprehensive error types and handling for the API

use axum::{
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

use super::responses::{ApiResponse, ApiErrorResponse, ResponseMeta};

/// API error types
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Authentication failed: {0}")]
    Authentication(String),
    
    #[error("Authorization failed: {0}")]
    Authorization(String),
    
    #[error("Resource not found: {0}")]
    NotFound(String),
    
    #[error("Rate limit exceeded")]
    RateLimited,
    
    #[error("Internal server error: {0}")]
    Internal(String),
    
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
    
    #[error("Bad request: {0}")]
    BadRequest(String),
    
    #[error("Conflict: {0}")]
    Conflict(String),
    
    #[error("Unprocessable entity: {0}")]
    UnprocessableEntity(String),
}

impl ApiError {
    /// Get the HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            ApiError::Validation(_) => StatusCode::BAD_REQUEST,
            ApiError::Authentication(_) => StatusCode::UNAUTHORIZED,
            ApiError::Authorization(_) => StatusCode::FORBIDDEN,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::UnprocessableEntity(_) => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }
    
    /// Get the error code string
    pub fn error_code(&self) -> &'static str {
        match self {
            ApiError::Validation(_) => "VALIDATION_ERROR",
            ApiError::Authentication(_) => "AUTHENTICATION_ERROR",
            ApiError::Authorization(_) => "AUTHORIZATION_ERROR",
            ApiError::NotFound(_) => "NOT_FOUND",
            ApiError::RateLimited => "RATE_LIMITED",
            ApiError::Internal(_) => "INTERNAL_ERROR",
            ApiError::ServiceUnavailable(_) => "SERVICE_UNAVAILABLE",
            ApiError::BadRequest(_) => "BAD_REQUEST",
            ApiError::Conflict(_) => "CONFLICT",
            ApiError::UnprocessableEntity(_) => "UNPROCESSABLE_ENTITY",
        }
    }
    
    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        ApiError::Validation(message.into())
    }
    
    /// Create an authentication error
    pub fn authentication(message: impl Into<String>) -> Self {
        ApiError::Authentication(message.into())
    }
    
    /// Create an authorization error
    pub fn authorization(message: impl Into<String>) -> Self {
        ApiError::Authorization(message.into())
    }
    
    /// Create a not found error
    pub fn not_found(resource: impl Into<String>) -> Self {
        ApiError::NotFound(resource.into())
    }
    
    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        ApiError::Internal(message.into())
    }
    
    /// Create a bad request error
    pub fn bad_request(message: impl Into<String>) -> Self {
        ApiError::BadRequest(message.into())
    }
    
    /// Create a conflict error
    pub fn conflict(message: impl Into<String>) -> Self {
        ApiError::Conflict(message.into())
    }
}

/// Convert ApiError to HTTP response
impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let status = self.status_code();
        let error_response = ApiResponse::<()> {
            success: false,
            data: None,
            error: Some(ApiErrorResponse {
                code: self.error_code().to_string(),
                message: self.to_string(),
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
        };
        
        (status, Json(error_response)).into_response()
    }
}

/// Result type for API operations
pub type ApiResult<T> = Result<T, ApiError>;

/// Convert std::io::Error to ApiError
impl From<std::io::Error> for ApiError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::NotFound => ApiError::NotFound("File not found".to_string()),
            std::io::ErrorKind::PermissionDenied => ApiError::Authorization("Permission denied".to_string()),
            _ => ApiError::Internal(format!("IO error: {}", err)),
        }
    }
}

/// Convert serde_json::Error to ApiError
impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        ApiError::Validation(format!("JSON error: {}", err))
    }
}

/// Convert hex::FromHexError to ApiError
impl From<hex::FromHexError> for ApiError {
    fn from(err: hex::FromHexError) -> Self {
        ApiError::Validation(format!("Invalid hex format: {}", err))
    }
}

/// Convert anyhow::Error to ApiError
impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::Internal(err.to_string())
    }
}

/// Convert validator::ValidationErrors to ApiError
impl From<validator::ValidationErrors> for ApiError {
    fn from(err: validator::ValidationErrors) -> Self {
        let details = serde_json::to_value(err).unwrap_or_default();
        ApiError::Validation("Validation failed".to_string())
    }
}

/// Error context for better debugging
pub trait ErrorContext<T> {
    fn context(self, msg: impl Into<String>) -> Result<T, ApiError>;
    fn with_context<F>(self, f: F) -> Result<T, ApiError>
    where
        F: FnOnce() -> String;
}

impl<T, E> ErrorContext<T> for Result<T, E>
where
    E: Into<ApiError>,
{
    fn context(self, msg: impl Into<String>) -> Result<T, ApiError> {
        self.map_err(|e| {
            let api_error: ApiError = e.into();
            match api_error {
                ApiError::Internal(inner_msg) => ApiError::Internal(format!("{}: {}", msg.into(), inner_msg)),
                _ => api_error,
            }
        })
    }
    
    fn with_context<F>(self, f: F) -> Result<T, ApiError>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| {
            let api_error: ApiError = e.into();
            match api_error {
                ApiError::Internal(inner_msg) => ApiError::Internal(format!("{}: {}", f(), inner_msg)),
                _ => api_error,
            }
        })
    }
}

/// Error logging helper
pub fn log_error(error: &ApiError, request_id: &str) {
    use tracing::{error, warn, info};
    
    match error {
        ApiError::Internal(msg) => {
            error!(
                request_id = %request_id,
                error = %error,
                "Internal server error: {}", msg
            );
        }
        ApiError::Validation(msg) => {
            warn!(
                request_id = %request_id,
                error = %error,
                "Validation error: {}", msg
            );
        }
        ApiError::Authentication(msg) => {
            warn!(
                request_id = %request_id,
                error = %error,
                "Authentication error: {}", msg
            );
        }
        ApiError::Authorization(msg) => {
            warn!(
                request_id = %request_id,
                error = %error,
                "Authorization error: {}", msg
            );
        }
        ApiError::RateLimited => {
            info!(
                request_id = %request_id,
                error = %error,
                "Rate limit exceeded"
            );
        }
        _ => {
            warn!(
                request_id = %request_id,
                error = %error,
                "API error occurred"
            );
        }
    }
}
