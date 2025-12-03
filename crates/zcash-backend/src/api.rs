//! REST API module for Zcash Backend
//!
//! Provides HTTP endpoints for querying payment status.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::storage::{ReceivedPayment, Storage};
use khafi_common::Nullifier;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<Mutex<Storage>>,
}

/// Payment status response
#[derive(Debug, Serialize)]
pub struct PaymentStatusResponse {
    pub exists: bool,
    pub used: bool,
    pub amount: Option<u64>,
    pub block_height: Option<u32>,
    pub tx_id: Option<String>,
}

/// Admin payment insertion request
#[derive(Debug, Deserialize)]
pub struct InsertPaymentRequest {
    pub nullifier_hex: String,
    pub amount: u64,
    pub tx_id: String,
    pub block_height: u32,
}

/// Stats response
#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub total_payments: usize,
    pub unused_payments: usize,
    pub total_amount_zec: f64,
}

/// API error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// Create the API router
pub fn create_router(storage: Storage) -> Router {
    let state = AppState {
        storage: Arc::new(Mutex::new(storage)),
    };

    Router::new()
        .route("/health", get(health_handler))
        .route("/payment/{nullifier}", get(get_payment_handler))
        .route("/admin/payment", post(insert_payment_handler))
        .route("/stats", get(stats_handler))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Health check endpoint
async fn health_handler(State(state): State<AppState>) -> Response {
    let mut storage = state.storage.lock().await;

    match storage.health_check().await {
        Ok(_) => (StatusCode::OK, "OK").into_response(),
        Err(e) => (
            StatusCode::SERVICE_UNAVAILABLE,
            format!("Redis connection failed: {}", e),
        )
            .into_response(),
    }
}

/// Get payment status by nullifier
///
/// GET /payment/:nullifier
async fn get_payment_handler(
    State(state): State<AppState>,
    Path(nullifier_hex): Path<String>,
) -> Response {
    // Parse nullifier
    let nullifier = match parse_nullifier(&nullifier_hex) {
        Ok(n) => n,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("Invalid nullifier: {}", e),
                }),
            )
                .into_response()
        }
    };

    let mut storage = state.storage.lock().await;

    // Get payment
    match storage.get_payment(&nullifier).await {
        Ok(Some(payment)) => {
            let response = PaymentStatusResponse {
                exists: true,
                used: payment.used,
                amount: Some(payment.amount),
                block_height: Some(payment.block_height),
                tx_id: Some(payment.tx_id),
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Ok(None) => {
            let response = PaymentStatusResponse {
                exists: false,
                used: false,
                amount: None,
                block_height: None,
                tx_id: None,
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Storage error: {}", e),
            }),
        )
            .into_response(),
    }
}

/// Insert payment manually (admin endpoint for testing)
///
/// POST /admin/payment
async fn insert_payment_handler(
    State(state): State<AppState>,
    Json(req): Json<InsertPaymentRequest>,
) -> Response {
    // Parse nullifier
    let nullifier = match parse_nullifier(&req.nullifier_hex) {
        Ok(n) => n,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("Invalid nullifier: {}", e),
                }),
            )
                .into_response()
        }
    };

    // Create payment
    let payment = ReceivedPayment::new(nullifier, req.amount, req.tx_id, req.block_height);

    let mut storage = state.storage.lock().await;

    match storage.insert_payment(&payment).await {
        Ok(true) => {
            info!("Manually inserted payment: {}", req.nullifier_hex);
            (StatusCode::CREATED, "Payment inserted").into_response()
        }
        Ok(false) => (
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "Payment already exists".to_string(),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Storage error: {}", e),
            }),
        )
            .into_response(),
    }
}

/// Get payment statistics
///
/// GET /stats
async fn stats_handler(State(state): State<AppState>) -> Response {
    let mut storage = state.storage.lock().await;

    match storage.get_stats().await {
        Ok(stats) => {
            let response = StatsResponse {
                total_payments: stats.total_payments,
                unused_payments: stats.unused_payments,
                total_amount_zec: stats.total_amount as f64 / 100_000_000.0,
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Storage error: {}", e),
            }),
        )
            .into_response(),
    }
}

/// Parse nullifier from hex string
fn parse_nullifier(hex: &str) -> Result<Nullifier, String> {
    let bytes = hex::decode(hex).map_err(|e| format!("Invalid hex: {}", e))?;

    if bytes.len() != 32 {
        return Err(format!("Nullifier must be 32 bytes, got {}", bytes.len()));
    }

    let mut array = [0u8; 32];
    array.copy_from_slice(&bytes);

    Ok(Nullifier::new(array))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_nullifier() {
        let hex = "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20";
        let result = parse_nullifier(hex);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_nullifier_invalid_length() {
        let hex = "0102030405"; // Too short
        let result = parse_nullifier(hex);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_nullifier_invalid_hex() {
        let hex = "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz";
        let result = parse_nullifier(hex);
        assert!(result.is_err());
    }
}
