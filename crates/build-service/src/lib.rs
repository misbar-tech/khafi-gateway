//! Build Service
//!
//! Async build service for RISC Zero guest programs.
//! Processes build jobs from a Redis queue and registers
//! completed builds with the Image ID Registry.

pub mod handlers;
pub mod models;
pub mod storage;
pub mod worker;

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub use handlers::AppState;
pub use models::{BuildJob, BuildStatus, QueueBuildRequest, QueueBuildResponse};
pub use storage::Storage;
pub use worker::{Worker, WorkerConfig};

/// Create the API router
pub fn create_router(state: AppState) -> Router {
    let shared_state = Arc::new(state);

    Router::new()
        .route("/health", get(handlers::health_handler))
        .route("/api/stats", get(handlers::get_stats_handler))
        .route("/api/build", post(handlers::queue_build_handler))
        .route("/api/build/:job_id", get(handlers::get_job_status_handler))
        .route(
            "/api/customer/:customer_id/builds",
            get(handlers::get_customer_jobs_handler),
        )
        .with_state(shared_state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}
