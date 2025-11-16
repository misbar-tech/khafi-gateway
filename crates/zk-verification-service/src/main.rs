//! ZK Verification Service
//!
//! gRPC service that implements Envoy ExtAuth protocol for ZK proof verification

mod config;
mod nullifier;
mod service;

use config::Config;
use service::AuthorizationService;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing/logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("Starting ZK Verification Service...");

    // Load configuration
    let config = Config::from_env();
    tracing::info!("Redis URL: {}", config.redis_url);
    tracing::info!("Image ID: {}", hex::encode(config.image_id));

    // Create authorization service
    let auth_service = AuthorizationService::new(config).await?;
    tracing::info!("Authorization service initialized");

    // Server address
    let addr = "0.0.0.0:50051".parse()?;
    tracing::info!("ZK Verification Service listening on {}", addr);

    // Start gRPC server
    Server::builder()
        .add_service(auth_service.into_service())
        .serve(addr)
        .await?;

    Ok(())
}
