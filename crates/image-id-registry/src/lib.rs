//! Image ID Registry Service
//!
//! Provides a registry for mapping customer IDs to RISC Zero Image IDs and guest programs.
//! Used by the ZK Verification Service to validate proofs from different customers.

pub mod handlers;
pub mod models;
pub mod storage;

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub use handlers::AppState;
pub use models::{CustomerDeployment, DeploymentMetadata};
pub use storage::Storage;

/// Create the application router
pub fn create_router(state: AppState) -> Router {
    let shared_state = Arc::new(state);

    Router::new()
        .route("/health", get(handlers::health_handler))
        .route(
            "/api/deployments",
            post(handlers::register_deployment_handler),
        )
        .route(
            "/api/deployments",
            get(handlers::list_deployments_handler),
        )
        .route(
            "/api/deployments/:customer_id",
            get(handlers::get_deployment_handler),
        )
        .route(
            "/api/deployments/:customer_id",
            put(handlers::update_deployment_handler),
        )
        .route(
            "/api/deployments/:customer_id",
            delete(handlers::delete_deployment_handler),
        )
        .route(
            "/api/deployments/by-image-id/:image_id",
            get(handlers::get_deployment_by_image_id_handler),
        )
        .with_state(shared_state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}
