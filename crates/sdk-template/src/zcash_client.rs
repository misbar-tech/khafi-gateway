//! Zcash backend client
//!
//! This module handles communication with the Zcash backend service
//! to fetch the commitment tree root and check nullifiers.

use anyhow::Context;
use khafi_common::{Nullifier, Result};
use serde::{Deserialize, Serialize};

/// Client for the Zcash backend service
pub struct ZcashClient {
    base_url: String,
    client: reqwest::Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommitmentTreeRoot {
    pub root: [u8; 32],
    pub block_height: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NullifierCheckResponse {
    pub exists: bool,
}

impl ZcashClient {
    /// Create a new Zcash client
    ///
    /// # Arguments
    /// * `base_url` - The URL of the Zcash backend service (e.g., "http://localhost:8081")
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
        }
    }

    /// Fetch the latest Orchard commitment tree root
    ///
    /// This public value is needed by the guest program to prove note inclusion
    pub async fn get_commitment_tree_root(&self) -> Result<CommitmentTreeRoot> {
        let url = format!("{}/api/commitment-tree/root", self.base_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch commitment tree root")?
            .json::<CommitmentTreeRoot>()
            .await
            .context("Failed to parse commitment tree root response")?;

        Ok(response)
    }

    /// Check if a nullifier has already been used
    ///
    /// This prevents replay attacks by ensuring each note can only be spent once
    pub async fn check_nullifier(&self, nullifier: &Nullifier) -> Result<bool> {
        let url = format!(
            "{}/api/nullifier/check/{}",
            self.base_url,
            nullifier.to_hex()
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to check nullifier")?
            .json::<NullifierCheckResponse>()
            .await
            .context("Failed to parse nullifier check response")?;

        Ok(response.exists)
    }

    /// Health check for the Zcash backend service
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/health", self.base_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to reach Zcash backend")?;

        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = ZcashClient::new("http://localhost:8081".to_string());
        assert_eq!(client.base_url, "http://localhost:8081");
    }
}
