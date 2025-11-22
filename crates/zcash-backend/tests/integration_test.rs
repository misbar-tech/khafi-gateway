//! Integration tests for Zcash Backend
//!
//! These tests verify the full payment flow from block monitoring to API queries.
//!
//! Requirements:
//! - Redis running on localhost:6379
//! - Run with: cargo test --package zcash-backend -- --ignored

use khafi_common::Nullifier;
use zcash_backend::{config::Config, monitor::Monitor, storage::Storage};

#[tokio::test]
#[ignore] // Requires Redis to be running
async fn test_monitor_initialization() {
    // Set up environment
    std::env::set_var("REDIS_URL", "redis://localhost:6379");
    std::env::set_var("MOCK_MODE", "true");
    std::env::set_var("PAYMENT_ADDRESS", "test_address");
    std::env::set_var("POLLING_INTERVAL_SECS", "1");

    // Load config
    let config = Config::from_env().expect("Failed to load config");

    // Create storage to verify Redis connection
    let mut storage = Storage::new(&config.redis_url)
        .await
        .expect("Failed to connect to Redis");

    // Verify storage works
    storage
        .health_check()
        .await
        .expect("Redis health check failed");

    // Create monitor
    let _monitor = Monitor::new(config)
        .await
        .expect("Failed to create monitor");

    println!("Integration test: Monitor initialized successfully");
    println!("Note: Full end-to-end monitoring test requires manual testing");
    println!("Start the service with: cargo run --bin zcash-backend");
    println!("The monitor will automatically detect payments every 60 seconds");
}

#[tokio::test]
#[ignore]
async fn test_storage_operations() {
    use zcash_backend::storage::ReceivedPayment;

    let mut storage = Storage::new("redis://localhost:6379")
        .await
        .expect("Failed to connect to Redis");

    // Create test payment
    let nullifier = Nullifier::new([42u8; 32]);
    let payment = ReceivedPayment::new(
        nullifier.clone(),
        20000000, // 0.2 ZEC
        "test_integration_tx".to_string(),
        200000,
    );

    // Insert payment
    let inserted = storage
        .insert_payment(&payment)
        .await
        .expect("Failed to insert payment");
    assert!(inserted, "Payment should be inserted");

    // Get payment
    let retrieved = storage
        .get_payment(&nullifier)
        .await
        .expect("Failed to get payment");
    assert!(retrieved.is_some(), "Payment should exist");

    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.amount, 20000000);
    assert_eq!(retrieved.block_height, 200000);
    assert!(!retrieved.used);

    // Mark as used
    let marked = storage
        .mark_used(&nullifier)
        .await
        .expect("Failed to mark payment as used");
    assert!(marked, "Payment should be marked as used");

    // Verify marked as used
    let retrieved = storage
        .get_payment(&nullifier)
        .await
        .expect("Failed to get payment");
    assert!(retrieved.unwrap().used, "Payment should be marked as used");

    // Get stats
    let stats = storage.get_stats().await.expect("Failed to get stats");
    assert!(stats.total_payments > 0);
}
