//! Proof Generation Service
//!
//! REST API for generating RISC Zero proofs for customer guest programs

use anyhow::{Context, Result};
use proof_generation_service::{create_router, AppState, Prover, RegistryClient};
use std::env;
use tokio::sync::RwLock;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "proof_generation_service=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load environment variables
    dotenvy::dotenv().ok();

    // Configuration
    let registry_url = env::var("REGISTRY_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8083".to_string());
    let host = env::var("PROVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("PROVER_PORT").unwrap_or_else(|_| "8084".to_string());

    info!("Starting Proof Generation Service");
    info!("Registry URL: {}", registry_url);
    info!("Listening on {}:{}", host, port);

    // Initialize prover
    let prover = Prover::new();

    // Initialize registry client
    let registry_client = RegistryClient::new(registry_url);

    // Check registry health
    info!("Checking Image ID Registry health...");
    match registry_client.health_check().await {
        Ok(true) => info!("Image ID Registry is healthy"),
        Ok(false) => info!("Warning: Image ID Registry returned non-success status"),
        Err(e) => info!("Warning: Failed to connect to Image ID Registry: {}", e),
    }

    // Create application state
    let state = AppState {
        prover: RwLock::new(prover),
        registry_client,
    };

    // Create router
    let app = create_router(state);

    // Bind and serve
    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .context("Failed to bind to address")?;

    info!("Proof Generation Service running on http://{}", addr);

    axum::serve(listener, app)
        .await
        .context("Server error")?;

    Ok(())
}
