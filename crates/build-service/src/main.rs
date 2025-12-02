//! Build Service
//!
//! REST API for queuing builds + background worker for processing them

use anyhow::{Context, Result};
use build_service::{create_router, AppState, Storage, WorkerConfig};
use std::env;
use std::path::PathBuf;
use tokio::sync::Mutex;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "build_service=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load environment variables
    dotenvy::dotenv().ok();

    // Configuration
    let redis_url = env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let host = env::var("BUILD_HOST")
        .unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("BUILD_PORT")
        .unwrap_or_else(|_| "8085".to_string());
    let build_dir = env::var("BUILD_DIR")
        .unwrap_or_else(|_| "/tmp/builds".to_string());
    let registry_url = env::var("REGISTRY_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8083".to_string());
    let gateway_url = env::var("GATEWAY_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());

    info!("Starting Build Service");
    info!("Redis URL: {}", redis_url);
    info!("Registry URL: {}", registry_url);
    info!("Build directory: {}", build_dir);

    // Ensure build directory exists
    std::fs::create_dir_all(&build_dir)
        .context("Failed to create build directory")?;

    // Initialize storage for API
    let api_storage = Storage::new(&redis_url)
        .await
        .context("Failed to initialize API storage")?;

    // Initialize storage for worker
    let worker_storage = Storage::new(&redis_url)
        .await
        .context("Failed to initialize worker storage")?;

    // Create application state
    let state = AppState {
        storage: Mutex::new(api_storage),
    };

    // Create router
    let app = create_router(state);

    // Worker configuration
    let worker_config = WorkerConfig {
        build_dir: PathBuf::from(build_dir),
        registry_url,
        gateway_url,
        num_workers: 1,
    };

    // Spawn worker task
    let worker_handle = tokio::spawn(async move {
        let mut worker = build_service::Worker::new(worker_config, worker_storage);
        if let Err(e) = worker.run().await {
            tracing::error!("Worker error: {}", e);
        }
    });

    // Start API server
    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .context("Failed to bind to address")?;

    info!("Build Service API running on http://{}", addr);
    info!("Worker started, processing build jobs...");

    // Run server (worker runs in background)
    axum::serve(listener, app)
        .await
        .context("Server error")?;

    // Wait for worker (unreachable in normal operation)
    worker_handle.await?;

    Ok(())
}
