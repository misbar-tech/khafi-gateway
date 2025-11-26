//! Proof Generation Service
//!
//! Hosts customer guest programs and generates RISC Zero proofs on their behalf.
//! Integrates with Image ID Registry to fetch and load customer deployments.

pub mod handlers;
pub mod models;
pub mod prover;
pub mod registry_client;

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub use handlers::AppState;
pub use models::{GenerateProofRequest, GenerateProofResponse, GuestProgram};
pub use prover::{Prover, ProofResult};
pub use registry_client::RegistryClient;

/// Create the application router
pub fn create_router(state: AppState) -> Router {
    let shared_state = Arc::new(state);

    Router::new()
        .route("/health", get(handlers::health_handler))
        .route("/api/status", get(handlers::status_handler))
        .route("/api/generate-proof", post(handlers::generate_proof_handler))
        .route("/api/load-program", post(handlers::load_program_handler))
        .with_state(shared_state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}
