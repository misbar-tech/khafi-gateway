//! API request handlers for Image ID Registry

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

use crate::{
    models::{CustomerDeployment, DeploymentMetadata},
    storage::Storage,
};

/// Shared application state
pub struct AppState {
    pub storage: Mutex<Storage>,
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

/// Request to register a new deployment
#[derive(Debug, Deserialize)]
pub struct RegisterDeploymentRequest {
    pub customer_id: String,
    pub image_id: String,
    pub guest_program_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<DeploymentMetadata>,
}

/// Response from registration
#[derive(Debug, Serialize)]
pub struct RegisterDeploymentResponse {
    pub success: bool,
    pub message: String,
}

/// Request to update a deployment
#[derive(Debug, Deserialize)]
pub struct UpdateDeploymentRequest {
    pub image_id: String,
    pub guest_program_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<DeploymentMetadata>,
}

/// Deployment info response
#[derive(Debug, Serialize)]
pub struct DeploymentResponse {
    pub deployment: CustomerDeployment,
}

/// List of deployments
#[derive(Debug, Serialize)]
pub struct DeploymentsListResponse {
    pub deployments: Vec<CustomerDeployment>,
    pub total: usize,
}

/// Health check endpoint
pub async fn health_handler() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "image-id-registry"
    }))
}

/// Register a new customer deployment
pub async fn register_deployment_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RegisterDeploymentRequest>,
) -> Result<Json<RegisterDeploymentResponse>, ApiError> {
    info!("Registering deployment for customer: {}", payload.customer_id);

    let deployment = CustomerDeployment::new(
        payload.customer_id.clone(),
        payload.image_id,
        payload.guest_program_path,
        payload.metadata,
    );

    let mut storage = state.storage.lock().await;
    let created = storage.register_deployment(&deployment).await?;

    if created {
        Ok(Json(RegisterDeploymentResponse {
            success: true,
            message: format!("Deployment registered for customer: {}", payload.customer_id),
        }))
    } else {
        Err(ApiError {
            status: StatusCode::CONFLICT,
            message: format!(
                "Deployment already exists for customer: {}",
                payload.customer_id
            ),
        })
    }
}

/// Update an existing customer deployment
pub async fn update_deployment_handler(
    State(state): State<Arc<AppState>>,
    Path(customer_id): Path<String>,
    Json(payload): Json<UpdateDeploymentRequest>,
) -> Result<Json<RegisterDeploymentResponse>, ApiError> {
    info!("Updating deployment for customer: {}", customer_id);

    let deployment = CustomerDeployment::new(
        customer_id.clone(),
        payload.image_id,
        payload.guest_program_path,
        payload.metadata,
    );

    let mut storage = state.storage.lock().await;
    let updated = storage.update_deployment(&deployment).await?;

    if updated {
        Ok(Json(RegisterDeploymentResponse {
            success: true,
            message: format!("Deployment updated for customer: {}", customer_id),
        }))
    } else {
        Err(ApiError {
            status: StatusCode::NOT_FOUND,
            message: format!("Deployment not found for customer: {}", customer_id),
        })
    }
}

/// Get deployment by customer ID
pub async fn get_deployment_handler(
    State(state): State<Arc<AppState>>,
    Path(customer_id): Path<String>,
) -> Result<Json<DeploymentResponse>, ApiError> {
    info!("Getting deployment for customer: {}", customer_id);

    let mut storage = state.storage.lock().await;
    let deployment = storage.get_deployment(&customer_id).await?;

    match deployment {
        Some(d) => Ok(Json(DeploymentResponse { deployment: d })),
        None => Err(ApiError {
            status: StatusCode::NOT_FOUND,
            message: format!("Deployment not found for customer: {}", customer_id),
        }),
    }
}

/// Get deployment by Image ID
pub async fn get_deployment_by_image_id_handler(
    State(state): State<Arc<AppState>>,
    Path(image_id): Path<String>,
) -> Result<Json<DeploymentResponse>, ApiError> {
    info!("Getting deployment for image_id: {}", image_id);

    let mut storage = state.storage.lock().await;
    let deployment = storage.get_deployment_by_image_id(&image_id).await?;

    match deployment {
        Some(d) => Ok(Json(DeploymentResponse { deployment: d })),
        None => Err(ApiError {
            status: StatusCode::NOT_FOUND,
            message: format!("Deployment not found for image_id: {}", image_id),
        }),
    }
}

/// Delete a customer deployment
pub async fn delete_deployment_handler(
    State(state): State<Arc<AppState>>,
    Path(customer_id): Path<String>,
) -> Result<Json<RegisterDeploymentResponse>, ApiError> {
    info!("Deleting deployment for customer: {}", customer_id);

    let mut storage = state.storage.lock().await;
    let deleted = storage.delete_deployment(&customer_id).await?;

    if deleted {
        Ok(Json(RegisterDeploymentResponse {
            success: true,
            message: format!("Deployment deleted for customer: {}", customer_id),
        }))
    } else {
        Err(ApiError {
            status: StatusCode::NOT_FOUND,
            message: format!("Deployment not found for customer: {}", customer_id),
        })
    }
}

/// List all deployments
pub async fn list_deployments_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<DeploymentsListResponse>, ApiError> {
    info!("Listing all deployments");

    let mut storage = state.storage.lock().await;
    let customer_ids = storage.list_customers().await?;

    let mut deployments = Vec::new();
    for customer_id in &customer_ids {
        if let Ok(Some(deployment)) = storage.get_deployment(customer_id).await {
            deployments.push(deployment);
        }
    }

    let total = deployments.len();

    Ok(Json(DeploymentsListResponse { deployments, total }))
}
