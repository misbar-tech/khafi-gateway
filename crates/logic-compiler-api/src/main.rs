//! Logic Compiler API Service
//!
//! REST API service for validating, compiling, and deploying business logic DSL.

mod config;

use anyhow::{Context, Result};
use config::Config;
use logic_compiler_api::{create_router, AppState};
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    info!("Starting Logic Compiler API Service");

    // Load configuration
    let config = Config::from_env().context("Failed to load configuration")?;
    info!(
        "Configuration loaded - listening on {}",
        config.api_address()
    );

    // Ensure directories exist
    config
        .ensure_directories()
        .context("Failed to create directories")?;
    info!(
        "Output directory: {}",
        config.sdk_output_dir.display()
    );
    info!("Templates directory: {}", config.templates_dir.display());

    // Create application state
    let state = AppState::new(config.sdk_output_dir.clone(), config.templates_dir.clone());

    // Create router
    let app = create_router(state);

    // Start server
    let listener = TcpListener::bind(&config.api_address())
        .await
        .with_context(|| format!("Failed to bind to {}", config.api_address()))?;

    info!("Logic Compiler API listening on {}", config.api_address());
    info!("Health check: http://{}/health", config.api_address());
    info!("API endpoints:");
    info!("  POST /api/validate - Validate DSL");
    info!("  POST /api/compile - Compile DSL to code");
    info!("  POST /api/sdk/generate - Generate SDK package");
    info!("  GET /api/sdk/download/{{id}} - Download SDK");
    info!("  GET /api/templates - List templates");
    info!("  GET /api/templates/{{name}} - Get template");

    axum::serve(listener, app)
        .await
        .context("Server error")?;

    Ok(())
}
