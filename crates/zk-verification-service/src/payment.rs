//! Payment verification and two-phase reservation
//!
//! Verifies Zcash payments exist in Redis and manages payment reservations
//! to prevent double-spending during proof generation.

use khafi_common::{Error, Nullifier, Result};
use redis::AsyncCommands;
use tracing::{debug, info, warn};

/// Reservation TTL - payments reserved for 5 minutes max
const RESERVATION_TTL_SECS: u64 = 300;

/// Default minimum payment amount in zatoshis (0.001 ZEC)
pub const DEFAULT_MIN_PAYMENT_AMOUNT: u64 = 100_000;

/// Default minimum confirmations required
pub const DEFAULT_MIN_CONFIRMATIONS: u32 = 1;

/// Payment information from Redis
#[derive(Debug)]
pub struct PaymentInfo {
    /// Amount in zatoshis
    pub amount: u64,
    /// Block height when payment was confirmed
    pub block_height: u32,
    /// Whether payment has been used
    pub used: bool,
    /// Transaction ID
    pub tx_id: String,
}

/// Payment verification configuration
#[derive(Clone)]
pub struct PaymentConfig {
    /// Whether payment verification is required
    pub require_payment: bool,
    /// Minimum payment amount in zatoshis
    pub min_payment_amount: u64,
    /// Minimum confirmations required
    pub min_confirmations: u32,
}

impl Default for PaymentConfig {
    fn default() -> Self {
        Self {
            require_payment: false,
            min_payment_amount: DEFAULT_MIN_PAYMENT_AMOUNT,
            min_confirmations: DEFAULT_MIN_CONFIRMATIONS,
        }
    }
}

impl PaymentConfig {
    /// Load payment configuration from environment
    pub fn from_env() -> Self {
        let require_payment = std::env::var("REQUIRE_PAYMENT")
            .map(|v| v.to_lowercase() == "true" || v == "1")
            .unwrap_or(false);

        let min_payment_amount = std::env::var("MIN_PAYMENT_AMOUNT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_MIN_PAYMENT_AMOUNT);

        let min_confirmations = std::env::var("MIN_CONFIRMATIONS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_MIN_CONFIRMATIONS);

        Self {
            require_payment,
            min_payment_amount,
            min_confirmations,
        }
    }
}

/// Payment checker with Redis backend
pub struct PaymentChecker {
    redis_client: redis::Client,
    config: PaymentConfig,
}

impl PaymentChecker {
    /// Create a new payment checker
    pub fn new(redis_url: &str, config: PaymentConfig) -> Result<Self> {
        let redis_client =
            redis::Client::open(redis_url).map_err(|e| Error::Redis(e.to_string()))?;
        Ok(Self {
            redis_client,
            config,
        })
    }

    /// Check if payment verification is required
    pub fn is_required(&self) -> bool {
        self.config.require_payment
    }

    /// Check if payment exists and meets requirements
    ///
    /// # Arguments
    /// * `nullifier` - The nullifier associated with the payment
    ///
    /// # Returns
    /// * `Ok(PaymentInfo)` - Payment found and valid
    /// * `Err` - Payment not found or invalid
    pub async fn check_payment(&self, nullifier: &Nullifier) -> Result<PaymentInfo> {
        let mut conn = self.get_connection().await?;
        let nullifier_hex = nullifier.to_hex();
        let payment_key = format!("payment:{}", nullifier_hex);

        // Check if payment exists
        let exists: bool = conn
            .exists(&payment_key)
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        if !exists {
            debug!("Payment not found for nullifier: {}", nullifier_hex);
            return Err(Error::Zcash("Payment not found".to_string()));
        }

        // Get payment details as hash
        let fields: Vec<(String, String)> = conn
            .hgetall(&payment_key)
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        let info = self.parse_payment_fields(&fields)?;

        // Check if already used
        if info.used {
            warn!("Payment already used: {}", nullifier_hex);
            return Err(Error::Zcash("Payment already used".to_string()));
        }

        // Check if already reserved by another request
        let reserved_key = format!("reserved:{}", nullifier_hex);
        let is_reserved: bool = conn
            .exists(&reserved_key)
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        if is_reserved {
            warn!("Payment is reserved by another request: {}", nullifier_hex);
            return Err(Error::Zcash(
                "Payment is reserved by another request".to_string(),
            ));
        }

        // Check minimum amount
        if info.amount < self.config.min_payment_amount {
            warn!(
                "Payment amount {} below minimum {}",
                info.amount, self.config.min_payment_amount
            );
            return Err(Error::Zcash(format!(
                "Payment amount {} below minimum {}",
                info.amount, self.config.min_payment_amount
            )));
        }

        // Check confirmations
        let current_height = self.get_current_block_height().await?;
        let confirmations = current_height.saturating_sub(info.block_height);

        if confirmations < self.config.min_confirmations {
            warn!(
                "Payment has {} confirmations, need at least {}",
                confirmations, self.config.min_confirmations
            );
            return Err(Error::Zcash(format!(
                "Payment has {} confirmations, need at least {}",
                confirmations, self.config.min_confirmations
            )));
        }

        debug!(
            "Payment verified: {} zatoshis, {} confirmations",
            info.amount, confirmations
        );

        Ok(info)
    }

    /// Reserve a payment with TTL (two-phase commit - phase 1)
    ///
    /// # Arguments
    /// * `nullifier` - The nullifier to reserve
    ///
    /// # Returns
    /// * `Ok(())` - Reservation successful
    /// * `Err` - Already reserved or Redis error
    pub async fn reserve_payment(&self, nullifier: &Nullifier) -> Result<()> {
        let mut conn = self.get_connection().await?;
        let nullifier_hex = nullifier.to_hex();
        let reserved_key = format!("reserved:{}", nullifier_hex);

        // SET NX with TTL - atomic reservation
        let set_result: Option<String> = redis::cmd("SET")
            .arg(&reserved_key)
            .arg("1")
            .arg("NX")
            .arg("EX")
            .arg(RESERVATION_TTL_SECS)
            .query_async(&mut conn)
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        if set_result.is_none() {
            warn!("Payment already reserved: {}", nullifier_hex);
            return Err(Error::Zcash("Payment already reserved".to_string()));
        }

        // Add to reserved set for tracking
        conn.sadd::<_, _, ()>("payments:reserved", &nullifier_hex)
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        info!("Payment reserved: {}", nullifier_hex);
        Ok(())
    }

    /// Confirm payment usage (two-phase commit - phase 2)
    ///
    /// Called after successful proof generation
    ///
    /// # Arguments
    /// * `nullifier` - The nullifier to confirm
    pub async fn confirm_payment(&self, nullifier: &Nullifier) -> Result<()> {
        let mut conn = self.get_connection().await?;
        let nullifier_hex = nullifier.to_hex();
        let payment_key = format!("payment:{}", nullifier_hex);
        let reserved_key = format!("reserved:{}", nullifier_hex);

        // Mark as used
        let now = chrono::Utc::now().to_rfc3339();
        conn.hset_multiple::<_, _, _, ()>(&payment_key, &[("used", "true"), ("used_at", &now)])
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        // Remove from unused set
        conn.srem::<_, _, ()>("payments:unused", &nullifier_hex)
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        // Remove reservation
        conn.del::<_, ()>(&reserved_key)
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;
        conn.srem::<_, _, ()>("payments:reserved", &nullifier_hex)
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        info!("Payment confirmed as used: {}", nullifier_hex);
        Ok(())
    }

    /// Release reservation (on failure)
    ///
    /// Called when proof generation fails to allow retry
    ///
    /// # Arguments
    /// * `nullifier` - The nullifier to release
    pub async fn release_reservation(&self, nullifier: &Nullifier) -> Result<()> {
        let mut conn = self.get_connection().await?;
        let nullifier_hex = nullifier.to_hex();
        let reserved_key = format!("reserved:{}", nullifier_hex);

        conn.del::<_, ()>(&reserved_key)
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;
        conn.srem::<_, _, ()>("payments:reserved", &nullifier_hex)
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        info!("Payment reservation released: {}", nullifier_hex);
        Ok(())
    }

    /// Get current block height from Redis (set by Zcash Backend)
    pub async fn get_current_block_height(&self) -> Result<u32> {
        let mut conn = self.get_connection().await?;

        let height: Option<String> = conn
            .get("chain:block_height")
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        height
            .and_then(|h| h.parse().ok())
            .ok_or_else(|| Error::Zcash("Block height not available".to_string()))
    }

    async fn get_connection(&self) -> Result<redis::aio::MultiplexedConnection> {
        self.redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| Error::Redis(e.to_string()))
    }

    fn parse_payment_fields(&self, fields: &[(String, String)]) -> Result<PaymentInfo> {
        let map: std::collections::HashMap<_, _> = fields.iter().cloned().collect();

        Ok(PaymentInfo {
            amount: map
                .get("amount")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
            block_height: map
                .get("block_height")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
            used: map.get("used").map(|s| s == "true").unwrap_or(false),
            tx_id: map.get("tx_id").cloned().unwrap_or_default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payment_config_defaults() {
        let config = PaymentConfig::default();
        assert!(!config.require_payment);
        assert_eq!(config.min_payment_amount, DEFAULT_MIN_PAYMENT_AMOUNT);
        assert_eq!(config.min_confirmations, DEFAULT_MIN_CONFIRMATIONS);
    }

    #[test]
    fn test_payment_config_from_env() {
        // Clear any existing env vars
        std::env::remove_var("REQUIRE_PAYMENT");
        std::env::remove_var("MIN_PAYMENT_AMOUNT");
        std::env::remove_var("MIN_CONFIRMATIONS");

        let config = PaymentConfig::from_env();
        assert!(!config.require_payment);
        assert_eq!(config.min_payment_amount, DEFAULT_MIN_PAYMENT_AMOUNT);
        assert_eq!(config.min_confirmations, DEFAULT_MIN_CONFIRMATIONS);
    }
}
