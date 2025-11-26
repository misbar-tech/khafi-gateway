//! Image ID Registry Service
//!
//! REST API for managing customer deployments and Image ID lookups

use anyhow::{Context, Result};
use image_id_registry::{create_router, AppState, Storage};
use std::env;
use tokio::sync::Mutex;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "image_id_registry=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load environment variables
    dotenvy::dotenv().ok();

    // Configuration
    let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let host = env::var("REGISTRY_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("REGISTRY_PORT").unwrap_or_else(|_| "8083".to_string());

    info!("Starting Image ID Registry Service");
    info!("Redis URL: {}", redis_url);
    info!("Listening on {}:{}", host, port);

    // Initialize storage
    let storage = Storage::new(&redis_url)
        .await
        .context("Failed to initialize storage")?;

    // Create application state
    let state = AppState {
        storage: Mutex::new(storage),
    };

    // Create router
    let app = create_router(state);

    // Bind and serve
    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .context("Failed to bind to address")?;

    info!("Image ID Registry Service running on http://{}", addr);

    axum::serve(listener, app)
        .await
        .context("Server error")?;

    Ok(())
}
