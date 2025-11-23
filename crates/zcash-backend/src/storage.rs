//! Redis storage module for payment data
//!
//! Data model:
//! - payment:{nullifier_hex} → Hash with payment fields
//! - payments:all → Set of all nullifiers
//! - payments:unused → Set of unused nullifiers
//! - payments:by_height → Sorted set (score=block_height, member=nullifier)

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use khafi_common::Nullifier;
use redis::{aio::ConnectionManager, AsyncCommands};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// Represents a received Zcash payment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceivedPayment {
    /// Unique nullifier from the Zcash transaction
    pub nullifier: Nullifier,

    /// Payment amount in zatoshis (1 ZEC = 100,000,000 zatoshis)
    pub amount: u64,

    /// Zcash transaction ID
    pub tx_id: String,

    /// Block height where transaction was confirmed
    pub block_height: u32,

    /// Timestamp when payment was received
    pub timestamp: DateTime<Utc>,

    /// Whether this nullifier has been used for API access
    pub used: bool,

    /// When the nullifier was marked as used
    pub used_at: Option<DateTime<Utc>>,
}

impl ReceivedPayment {
    /// Create a new payment record
    pub fn new(nullifier: Nullifier, amount: u64, tx_id: String, block_height: u32) -> Self {
        Self {
            nullifier,
            amount,
            tx_id,
            block_height,
            timestamp: Utc::now(),
            used: false,
            used_at: None,
        }
    }
}

/// Payment statistics
#[derive(Debug, Serialize)]
pub struct PaymentStats {
    pub total_payments: usize,
    pub unused_payments: usize,
    pub total_amount: u64,
}

/// Redis storage client
pub struct Storage {
    conn: ConnectionManager,
}

impl Storage {
    /// Create a new storage client
    pub async fn new(redis_url: &str) -> Result<Self> {
        info!("Connecting to Redis at {}", redis_url);

        let client = redis::Client::open(redis_url).context("Failed to create Redis client")?;

        let conn = ConnectionManager::new(client)
            .await
            .context("Failed to connect to Redis")?;

        info!("Successfully connected to Redis");

        Ok(Self { conn })
    }

    /// Insert a new payment record
    /// Returns Ok(true) if inserted, Ok(false) if already exists
    pub async fn insert_payment(&mut self, payment: &ReceivedPayment) -> Result<bool> {
        let nullifier_hex = payment.nullifier.to_hex();
        let payment_key = format!("payment:{}", nullifier_hex);

        // Check if payment already exists
        let exists: bool = self.conn.exists(&payment_key).await?;
        if exists {
            debug!("Payment {} already exists, skipping", nullifier_hex);
            return Ok(false);
        }

        // Store payment as hash
        self.conn
            .hset_multiple(
                &payment_key,
                &[
                    ("nullifier", nullifier_hex.as_str()),
                    ("amount", &payment.amount.to_string()),
                    ("tx_id", &payment.tx_id),
                    ("block_height", &payment.block_height.to_string()),
                    ("timestamp", &payment.timestamp.to_rfc3339()),
                    ("used", "false"),
                    ("used_at", ""),
                ],
            )
            .await?;

        // Add to indexes
        self.conn.sadd("payments:all", &nullifier_hex).await?;
        self.conn.sadd("payments:unused", &nullifier_hex).await?;
        self.conn
            .zadd(
                "payments:by_height",
                &nullifier_hex,
                payment.block_height as i64,
            )
            .await?;

        info!(
            "Inserted payment: nullifier={}, amount={}, block_height={}",
            nullifier_hex, payment.amount, payment.block_height
        );

        Ok(true)
    }

    /// Get a payment by nullifier
    pub async fn get_payment(&mut self, nullifier: &Nullifier) -> Result<Option<ReceivedPayment>> {
        let nullifier_hex = nullifier.to_hex();
        let payment_key = format!("payment:{}", nullifier_hex);

        // Check if payment exists
        let exists: bool = self.conn.exists(&payment_key).await?;
        if !exists {
            return Ok(None);
        }

        // Fetch all fields
        let fields: Vec<String> = self.conn.hgetall(&payment_key).await?;

        // Parse fields (Redis returns flat array: [key1, val1, key2, val2, ...])
        let mut map = std::collections::HashMap::new();
        for chunk in fields.chunks(2) {
            if chunk.len() == 2 {
                map.insert(chunk[0].clone(), chunk[1].clone());
            }
        }

        // Reconstruct payment
        let payment = ReceivedPayment {
            nullifier: nullifier.clone(),
            amount: map.get("amount").and_then(|s| s.parse().ok()).unwrap_or(0),
            tx_id: map.get("tx_id").cloned().unwrap_or_default(),
            block_height: map
                .get("block_height")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
            timestamp: map
                .get("timestamp")
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now),
            used: map.get("used").map(|s| s == "true").unwrap_or(false),
            used_at: map
                .get("used_at")
                .filter(|s| !s.is_empty())
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc)),
        };

        Ok(Some(payment))
    }

    /// Check if a payment exists
    pub async fn check_exists(&mut self, nullifier: &Nullifier) -> Result<bool> {
        let nullifier_hex = nullifier.to_hex();
        let payment_key = format!("payment:{}", nullifier_hex);
        Ok(self.conn.exists(&payment_key).await?)
    }

    /// Mark a payment as used
    /// Returns Ok(true) if marked, Ok(false) if already used or doesn't exist
    pub async fn mark_used(&mut self, nullifier: &Nullifier) -> Result<bool> {
        let nullifier_hex = nullifier.to_hex();
        let payment_key = format!("payment:{}", nullifier_hex);

        // Check if payment exists and is not already used
        let used: Option<String> = self.conn.hget(&payment_key, "used").await?;
        match used {
            None => {
                warn!("Cannot mark nonexistent payment as used: {}", nullifier_hex);
                return Ok(false);
            }
            Some(val) if val == "true" => {
                debug!("Payment {} already marked as used", nullifier_hex);
                return Ok(false);
            }
            _ => {}
        }

        // Mark as used
        let now = Utc::now().to_rfc3339();
        self.conn
            .hset_multiple(&payment_key, &[("used", "true"), ("used_at", &now)])
            .await?;

        // Remove from unused set
        self.conn.srem("payments:unused", &nullifier_hex).await?;

        info!("Marked payment as used: {}", nullifier_hex);

        Ok(true)
    }

    /// Get payment statistics
    pub async fn get_stats(&mut self) -> Result<PaymentStats> {
        let total_payments: usize = self.conn.scard("payments:all").await?;
        let unused_payments: usize = self.conn.scard("payments:unused").await?;

        // Calculate total amount (requires fetching all payments)
        let all_nullifiers: Vec<String> = self.conn.smembers("payments:all").await?;
        let mut total_amount = 0u64;

        for nullifier_hex in all_nullifiers {
            let payment_key = format!("payment:{}", nullifier_hex);
            if let Ok(Some(amount_str)) = self
                .conn
                .hget::<_, _, Option<String>>(&payment_key, "amount")
                .await
            {
                if let Ok(amount) = amount_str.parse::<u64>() {
                    total_amount += amount;
                }
            }
        }

        Ok(PaymentStats {
            total_payments,
            unused_payments,
            total_amount,
        })
    }

    /// Get the latest block height we've processed
    pub async fn get_latest_block_height(&mut self) -> Result<Option<u32>> {
        // Get the highest score (block height) from the sorted set
        let result: Vec<(String, i64)> = self
            .conn
            .zrevrange_withscores("payments:by_height", 0, 0)
            .await?;

        if let Some((_, height)) = result.first() {
            Ok(Some(*height as u32))
        } else {
            Ok(None)
        }
    }

    /// Health check - verify Redis connection
    pub async fn health_check(&mut self) -> Result<()> {
        let _: String = redis::cmd("PING")
            .query_async(&mut self.conn)
            .await
            .context("Redis health check failed")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integration tests require Redis to be running
    // Run with: docker compose up -d redis

    #[tokio::test]
    #[ignore] // Only run when Redis is available
    async fn test_insert_and_get_payment() {
        let mut storage = Storage::new("redis://localhost:6379")
            .await
            .expect("Failed to connect to Redis");

        let nullifier = Nullifier::new([1u8; 32]);
        let payment = ReceivedPayment::new(
            nullifier.clone(),
            10000000, // 0.1 ZEC
            "test_tx_123".to_string(),
            12345,
        );

        // Insert payment
        let inserted = storage.insert_payment(&payment).await.unwrap();
        assert!(inserted);

        // Get payment
        let retrieved = storage.get_payment(&nullifier).await.unwrap();
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.amount, 10000000);
        assert_eq!(retrieved.tx_id, "test_tx_123");
        assert!(!retrieved.used);
    }

    #[tokio::test]
    #[ignore]
    async fn test_mark_used() {
        let mut storage = Storage::new("redis://localhost:6379")
            .await
            .expect("Failed to connect to Redis");

        let nullifier = Nullifier::new([2u8; 32]);
        let payment =
            ReceivedPayment::new(nullifier.clone(), 5000000, "test_tx_456".to_string(), 12346);

        storage.insert_payment(&payment).await.unwrap();

        // Mark as used
        let marked = storage.mark_used(&nullifier).await.unwrap();
        assert!(marked);

        // Verify it's marked as used
        let retrieved = storage.get_payment(&nullifier).await.unwrap().unwrap();
        assert!(retrieved.used);
        assert!(retrieved.used_at.is_some());

        // Try marking again - should return false
        let marked_again = storage.mark_used(&nullifier).await.unwrap();
        assert!(!marked_again);
    }
}
