//! Zcash Backend Service
//!
//! Main entry point for the blockchain monitoring and payment verification service.

use anyhow::Result;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod config;
mod lightwalletd_client;
mod mock_node;
mod monitor;
mod note_decryption;
mod parser;
mod storage;

use config::Config;
use monitor::Monitor;
use storage::Storage;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,zcash_backend=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Zcash Backend Service");

    // Load configuration
    let config = Config::from_env()?;
    info!("Configuration loaded");
    info!("  Redis URL: {}", config.redis_url);
    info!("  API address: {}", config.api_address());
    info!("  Mock mode: {}", config.mock_mode);
    info!("  Polling interval: {}s", config.polling_interval_secs);

    // Initialize storage for API server
    let api_storage = Storage::new(&config.redis_url).await?;
    info!("Connected to Redis for API");

    // Create API router
    let app = api::create_router(api_storage);

    // Start API server in background
    let api_addr = config.api_address();
    let listener = tokio::net::TcpListener::bind(&api_addr).await?;
    info!("API server listening on {}", api_addr);

    let api_task = tokio::spawn(async move {
        info!("Starting API server task");
        if let Err(e) = axum::serve(listener, app).await {
            error!("API server error: {:#}", e);
        }
    });

    // Start blockchain monitor in background
    let monitor_config = config.clone();
    let monitor_task = tokio::spawn(async move {
        info!("Starting blockchain monitor task");
        match Monitor::new(monitor_config).await {
            Ok(monitor) => {
                if let Err(e) = monitor.start().await {
                    error!("Monitor error: {:#}", e);
                }
            }
            Err(e) => {
                error!("Failed to create monitor: {:#}", e);
            }
        }
    });

    info!("All tasks started successfully");
    info!("Zcash Backend Service is running");

    // Wait for either task to complete (they should run forever)
    tokio::select! {
        _ = api_task => {
            error!("API task terminated unexpectedly");
        }
        _ = monitor_task => {
            error!("Monitor task terminated unexpectedly");
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal");
        }
    }

    info!("Shutting down Zcash Backend Service");

    Ok(())
}
