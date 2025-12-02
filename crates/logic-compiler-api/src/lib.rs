//! Logic Compiler REST API
//!
//! This service provides HTTP endpoints for the Logic Compiler, enabling
//! SaaS customers to validate, compile, and deploy custom business logic
//! without requiring local Rust development environment.
//!
//! ## Architecture
//!
//! The API wraps the `logic-compiler` library and provides:
//! - DSL validation endpoint
//! - Code compilation endpoint
//! - SDK package generation and download
//! - Template management
//!
//! ## Endpoints
//!
//! - `POST /api/validate` - Validate DSL without compiling
//! - `POST /api/compile` - Compile DSL to guest program code
//! - `POST /api/sdk/generate` - Generate complete SDK package
//! - `GET /api/sdk/download/:id` - Download SDK package as tarball
//! - `GET /api/templates` - List available templates
//! - `GET /api/templates/:name` - Get specific template
//! - `GET /health` - Health check

pub mod config;
pub mod handlers;

use axum::{
    routing::{get, post},
    Router,
};
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    /// Directory where SDK packages are generated
    pub sdk_output_dir: PathBuf,

    /// Directory containing template files
    pub templates_dir: PathBuf,
}

impl AppState {
    /// Create new application state
    pub fn new(sdk_output_dir: PathBuf, templates_dir: PathBuf) -> Self {
        Self {
            sdk_output_dir,
            templates_dir,
        }
    }
}

/// Create the API router
pub fn create_router(state: AppState) -> Router {
    let state = Arc::new(state);

    Router::new()
        // Health check
        .route("/health", get(handlers::health_handler))
        // DSL validation and compilation
        .route("/api/validate", post(handlers::validate_handler))
        .route("/api/compile", post(handlers::compile_handler))
        // Deployment (async via Build Service)
        .route("/api/deploy", post(handlers::deploy_handler))
        .route("/api/deploy/status/{job_id}", get(handlers::deploy_status_handler))
        // SDK generation and download (legacy)
        .route("/api/sdk/generate", post(handlers::generate_sdk_handler))
        .route(
            "/api/sdk/download/{id}",
            get(handlers::download_sdk_handler),
        )
        // Template management
        .route("/api/templates", get(handlers::list_templates_handler))
        .route("/api/templates/{name}", get(handlers::get_template_handler))
        // Middleware
        .layer(
            CorsLayer::permissive(), // Allow all origins for development
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
