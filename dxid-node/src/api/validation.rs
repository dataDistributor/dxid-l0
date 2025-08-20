//! Input validation for API requests
//! 
//! This module provides validation for all API request inputs

use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};
use std::collections::HashSet;

use super::errors::ApiError;

/// Validated transaction submission request
#[derive(Debug, Deserialize, Validate)]
pub struct SubmitTxRequest {
    #[validate(length(equal = 64))]
    #[validate(regex = "^[0-9a-fA-F]+$")]
    pub from: String,
    
    #[validate(length(equal = 64))]
    #[validate(regex = "^[0-9a-fA-F]+$")]
    pub to: String,
    
    #[validate(range(min = 1, max = 1000000000000))]
    pub amount: u128,
    
    #[validate(range(min = 0, max = 1000000))]
    pub fee: u128,
    
    pub signature: dxid_crypto::StarkSignature,
}

/// Validated Layer0 transfer request
#[derive(Debug, Deserialize, Validate)]
pub struct Layer0TransferRequest {
    #[validate(length(equal = 64))]
    #[validate(regex = "^[0-9a-fA-F]+$")]
    pub from: String,
    
    #[validate(length(equal = 64))]
    #[validate(regex = "^[0-9a-fA-F]+$")]
    pub to: String,
    
    #[validate(range(min = 1, max = 1000000000000))]
    pub amount: u128,
    
    #[validate(range(min = 0, max = 1000000))]
    pub fee: u128,
    
    pub signature: dxid_crypto::StarkSignature,
}

/// Validated LongYield transfer request
#[derive(Debug, Deserialize, Validate)]
pub struct LongYieldTransferRequest {
    #[validate(length(equal = 64))]
    #[validate(regex = "^[0-9a-fA-F]+$")]
    pub from: String,
    
    #[validate(length(equal = 64))]
    #[validate(regex = "^[0-9a-fA-F]+$")]
    pub to: String,
    
    #[validate(range(min = 1, max = 1000000000000))]
    pub amount: u128,
    
    #[validate(range(min = 0, max = 1000000))]
    pub fee: u128,
    
    pub signature: dxid_crypto::StarkSignature,
}

/// Validated API key creation request
#[derive(Debug, Deserialize, Validate)]
pub struct CreateApiKeyRequest {
    #[validate(length(min = 1, max = 100))]
    #[validate(regex = "^[a-zA-Z0-9_-]+$")]
    pub name: String,
    
    #[validate(length(min = 0, max = 10))]
    pub permissions: Vec<String>,
    
    #[validate(range(min = 1, max = 10000))]
    pub rate_limit: Option<u32>,
}

/// Validated webhook creation request
#[derive(Debug, Deserialize, Validate)]
pub struct CreateWebhookRequest {
    #[validate(length(min = 1, max = 100))]
    pub api_key_id: String,
    
    #[validate(url)]
    pub url: String,
    
    #[validate(length(min = 1, max = 20))]
    pub events: Vec<String>,
    
    #[validate(length(min = 0, max = 100))]
    pub secret: Option<String>,
}

/// Validated proof verification request
#[derive(Debug, Deserialize, Validate)]
pub struct VerifyProofRequest {
    #[validate(length(min = 1, max = 10000))]
    pub proof_data: String,
    
    #[validate(length(min = 1, max = 1000))]
    pub public_inputs: Vec<String>,
    
    #[validate(length(min = 0, max = 1000))]
    pub verification_key: Option<String>,
}

/// Validate hex string
pub fn validate_hex(value: &str) -> Result<(), ValidationError> {
    if value.len() != 64 {
        return Err(ValidationError::new("invalid_hex_length"));
    }
    
    if !value.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ValidationError::new("invalid_hex_format"));
    }
    
    Ok(())
}

/// Validate address format
pub fn validate_address(value: &str) -> Result<(), ValidationError> {
    validate_hex(value)
}

/// Validate amount range
pub fn validate_amount(value: &u128) -> Result<(), ValidationError> {
    if *value == 0 {
        return Err(ValidationError::new("zero_amount"));
    }
    
    if *value > 1000000000000 {
        return Err(ValidationError::new("amount_too_large"));
    }
    
    Ok(())
}

/// Validate fee range
pub fn validate_fee(value: &u128) -> Result<(), ValidationError> {
    if *value > 1000000 {
        return Err(ValidationError::new("fee_too_high"));
    }
    
    Ok(())
}

/// Validate event types
pub fn validate_event_types(value: &Vec<String>) -> Result<(), ValidationError> {
    let valid_events: HashSet<&str> = [
        "block",
        "transaction",
        "transfer",
        "transfer_to",
        "proof",
        "error",
    ].iter().cloned().collect();
    
    for event in value {
        if !valid_events.contains(event.as_str()) {
            return Err(ValidationError::new("invalid_event_type"));
        }
    }
    
    Ok(())
}

/// Validate permissions
pub fn validate_permissions(value: &Vec<String>) -> Result<(), ValidationError> {
    let valid_permissions: HashSet<&str> = [
        "read",
        "write",
        "admin",
        "proof",
        "webhook",
    ].iter().cloned().collect();
    
    for permission in value {
        if !valid_permissions.contains(permission.as_str()) {
            return Err(ValidationError::new("invalid_permission"));
        }
    }
    
    Ok(())
}

/// Validate request and return ApiError if invalid
pub fn validate_request<T: Validate>(request: &T) -> Result<(), ApiError> {
    request.validate()
        .map_err(|errors| {
            let error_details = serde_json::to_value(errors).unwrap_or_default();
            ApiError::Validation(format!("Validation failed: {:?}", error_details))
        })
}

/// Validate hex address
pub fn validate_hex_address(address: &str) -> Result<[u8; 32], ApiError> {
    if address.len() != 64 {
        return Err(ApiError::Validation("Address must be 64 hex characters".to_string()));
    }
    
    if !address.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ApiError::Validation("Address must be valid hex".to_string()));
    }
    
    let bytes = hex::decode(address)
        .map_err(|e| ApiError::Validation(format!("Invalid hex format: {}", e)))?;
    
    if bytes.len() != 32 {
        return Err(ApiError::Validation("Address must be 32 bytes".to_string()));
    }
    
    let mut result = [0u8; 32];
    result.copy_from_slice(&bytes);
    Ok(result)
}

/// Validate amount
pub fn validate_amount_range(amount: u128) -> Result<(), ApiError> {
    if amount == 0 {
        return Err(ApiError::Validation("Amount cannot be zero".to_string()));
    }
    
    if amount > 1000000000000 {
        return Err(ApiError::Validation("Amount too large".to_string()));
    }
    
    Ok(())
}

/// Validate fee
pub fn validate_fee_range(fee: u128) -> Result<(), ApiError> {
    if fee > 1000000 {
        return Err(ApiError::Validation("Fee too high".to_string()));
    }
    
    Ok(())
}

/// Validate signature
pub fn validate_signature(signature: &dxid_crypto::StarkSignature) -> Result<(), ApiError> {
    // Basic signature validation
    if signature.pubkey_hash == [0u8; 32] {
        return Err(ApiError::Validation("Invalid signature".to_string()));
    }
    
    Ok(())
}
