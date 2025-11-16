//! Nullifier checking to prevent replay attacks

use khafi_common::{Error, Nullifier, Result};
use redis::AsyncCommands;

/// Nullifier checker with Redis backend
pub struct NullifierChecker {
    redis_client: redis::Client,
}

impl NullifierChecker {
    /// Create a new nullifier checker
    ///
    /// # Arguments
    /// * `redis_url` - Redis connection URL (e.g., "redis://localhost:6379")
    pub fn new(redis_url: &str) -> Result<Self> {
        let redis_client = redis::Client::open(redis_url)
            .map_err(|e| Error::Redis(e.to_string()))?;
        Ok(Self { redis_client })
    }

    /// Check if a nullifier has been used before and mark it as used
    ///
    /// This performs an atomic check-and-set operation using Redis SET NX.
    ///
    /// # Arguments
    /// * `nullifier` - The nullifier to check
    ///
    /// # Returns
    /// * `Ok(true)` - Nullifier is new (first time seeing it)
    /// * `Ok(false)` - Nullifier was already used (replay attack detected)
    /// * `Err` - Redis error
    pub async fn check_and_set(&self, nullifier: &Nullifier) -> Result<bool> {
        let mut conn = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        let key = format!("nullifier:{}", nullifier.to_hex());

        // SET NX - set if not exists (atomic operation)
        // Returns true if the key was set (didn't exist before)
        // Returns false if the key already existed
        let result: bool = conn
            .set_nx(&key, "1")
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        if result {
            // Nullifier is new - set TTL for cleanup (30 days)
            let _: () = conn
                .expire(&key, 2592000)
                .await
                .map_err(|e| Error::Redis(e.to_string()))?;
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires Redis to be running
    async fn test_nullifier_check_and_set() {
        let checker = NullifierChecker::new("redis://localhost:6379").unwrap();
        let nullifier = Nullifier::new([42u8; 32]);

        // First time should return true (new)
        let is_new = checker.check_and_set(&nullifier).await.unwrap();
        assert!(is_new);

        // Second time should return false (replay)
        let is_new = checker.check_and_set(&nullifier).await.unwrap();
        assert!(!is_new);
    }
}
