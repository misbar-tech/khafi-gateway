//! API handlers for Proof Generation Service

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

use crate::{
    models::{GenerateProofRequest, GenerateProofResponse, GuestProgram},
    prover::Prover,
    registry_client::RegistryClient,
};

/// Shared application state
pub struct AppState {
    pub prover: RwLock<Prover>,
    pub registry_client: RegistryClient,
}

/// API Error type
#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = serde_json::json!({
            "error": self.message
        });

        (self.status, Json(body)).into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: err.to_string(),
        }
    }
}

/// Health check endpoint
pub async fn health_handler() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "proof-generation-service"
    }))
}

/// Generate a proof for customer inputs
pub async fn generate_proof_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<GenerateProofRequest>,
) -> Result<Json<GenerateProofResponse>, ApiError> {
    info!("Generating proof for customer: {}", payload.customer_id);

    // Check if we have the guest program loaded
    let prover = state.prover.read().await;
    let has_program = prover.has_program(&payload.customer_id);
    drop(prover);

    // If not loaded, fetch from registry and load it
    if !has_program {
        info!("Guest program not loaded, fetching from registry");

        let deployment = state
            .registry_client
            .get_deployment(&payload.customer_id)
            .await?
            .ok_or_else(|| ApiError {
                status: StatusCode::NOT_FOUND,
                message: format!(
                    "No deployment found for customer: {}",
                    payload.customer_id
                ),
            })?;

        // Load the guest program
        let guest_program = GuestProgram::load(
            deployment.customer_id.clone(),
            deployment.image_id.clone(),
            deployment.guest_program_path.clone(),
        )
        .map_err(|e| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("Failed to load guest program: {}", e),
        })?;

        let mut prover = state.prover.write().await;
        prover.load_program(guest_program)?;
    }

    // Generate proof
    let prover = state.prover.read().await;
    match prover.generate_proof(
        &payload.customer_id,
        &payload.private_inputs,
        &payload.public_params,
    ) {
        Ok(result) => {
            info!("Proof generated successfully for customer: {}", payload.customer_id);
            Ok(Json(GenerateProofResponse {
                success: true,
                proof: Some(result.proof),
                image_id: Some(result.image_id),
                outputs: Some(result.outputs),
                error: None,
            }))
        }
        Err(e) => {
            error!("Proof generation failed for customer {}: {:#}", payload.customer_id, e);
            Ok(Json(GenerateProofResponse {
                success: false,
                proof: None,
                image_id: None,
                outputs: None,
                error: Some(format!("Proof generation failed: {}", e)),
            }))
        }
    }
}

/// Load a guest program for a customer
pub async fn load_program_handler(
    State(state): State<Arc<AppState>>,
    Json(customer_id): Json<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    info!("Loading guest program for customer: {}", customer_id);

    // Fetch deployment from registry
    let deployment = state
        .registry_client
        .get_deployment(&customer_id)
        .await?
        .ok_or_else(|| ApiError {
            status: StatusCode::NOT_FOUND,
            message: format!("No deployment found for customer: {}", customer_id),
        })?;

    // Load the guest program
    let guest_program = GuestProgram::load(
        deployment.customer_id.clone(),
        deployment.image_id.clone(),
        deployment.guest_program_path.clone(),
    )?;

    let mut prover = state.prover.write().await;
    prover.load_program(guest_program)?;

    info!("Guest program loaded successfully for customer: {}", customer_id);

    Ok(Json(serde_json::json!({
        "success": true,
        "customer_id": customer_id,
        "image_id": deployment.image_id
    })))
}

/// Get service status
pub async fn status_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let prover = state.prover.read().await;
    let program_count = prover.program_count();

    // Check registry health
    let registry_healthy = state.registry_client.health_check().await.unwrap_or(false);

    Ok(Json(serde_json::json!({
        "service": "proof-generation-service",
        "loaded_programs": program_count,
        "registry_healthy": registry_healthy
    })))
}
