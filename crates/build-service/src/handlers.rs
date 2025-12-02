//! API handlers for Build Service

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;
use uuid::Uuid;

use crate::{
    models::{BuildJob, BuildStatusResponse, QueueBuildRequest, QueueBuildResponse},
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

/// Health check
pub async fn health_handler() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "build-service"
    }))
}

/// Queue a new build job
pub async fn queue_build_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<QueueBuildRequest>,
) -> Result<Json<QueueBuildResponse>, ApiError> {
    info!("Queueing build for customer: {}", payload.customer_id);

    // Generate job ID
    let job_id = Uuid::new_v4().to_string();

    // Create job
    let mut job = BuildJob::new(
        job_id.clone(),
        payload.customer_id.clone(),
        payload.dsl,
    );
    job.webhook_url = payload.webhook_url;

    // Queue job
    let mut storage = state.storage.lock().await;
    storage.queue_job(&job).await?;

    info!("Build job queued: {} for customer: {}", job_id, payload.customer_id);

    Ok(Json(QueueBuildResponse {
        success: true,
        job_id: Some(job_id),
        error: None,
    }))
}

/// Get build job status
pub async fn get_job_status_handler(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<String>,
) -> Result<Json<BuildStatusResponse>, ApiError> {
    info!("Getting status for job: {}", job_id);

    let mut storage = state.storage.lock().await;
    let job = storage.get_job(&job_id).await?;

    match job {
        Some(j) => Ok(Json(BuildStatusResponse { job: j })),
        None => Err(ApiError {
            status: StatusCode::NOT_FOUND,
            message: format!("Job not found: {}", job_id),
        }),
    }
}

/// Get all jobs for a customer
pub async fn get_customer_jobs_handler(
    State(state): State<Arc<AppState>>,
    Path(customer_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    info!("Getting jobs for customer: {}", customer_id);

    let mut storage = state.storage.lock().await;
    let jobs = storage.get_customer_jobs(&customer_id).await?;

    Ok(Json(serde_json::json!({
        "customer_id": customer_id,
        "jobs": jobs,
        "total": jobs.len()
    })))
}

/// Get service stats
pub async fn get_stats_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut storage = state.storage.lock().await;
    let stats = storage.get_stats().await?;
    let queue_len = storage.queue_length().await?;

    Ok(Json(serde_json::json!({
        "service": "build-service",
        "queue_length": queue_len,
        "stats": stats
    })))
}
