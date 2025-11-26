//! Client for Image ID Registry Service

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::debug;

/// Client for interacting with Image ID Registry
pub struct RegistryClient {
    base_url: String,
    client: reqwest::Client,
}

/// Customer deployment information from registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentInfo {
    pub customer_id: String,
    pub image_id: String,
    pub guest_program_path: String,
}

/// Deployment response wrapper
#[derive(Debug, Deserialize)]
struct DeploymentResponse {
    deployment: DeploymentInfo,
}

impl RegistryClient {
    /// Create a new registry client
    pub fn new(registry_url: String) -> Self {
        Self {
            base_url: registry_url,
            client: reqwest::Client::new(),
        }
    }

    /// Get deployment information for a customer
    pub async fn get_deployment(&self, customer_id: &str) -> Result<Option<DeploymentInfo>> {
        let url = format!("{}/api/deployments/{}", self.base_url, customer_id);

        debug!("Fetching deployment from registry: {}", url);

        let response = self.client.get(&url).send().await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to fetch deployment: {}",
                response.status()
            );
        }

        let deployment_response: DeploymentResponse = response
            .json()
            .await
            .context("Failed to parse deployment response")?;

        Ok(Some(deployment_response.deployment))
    }

    /// Get deployment information by image ID
    pub async fn get_deployment_by_image_id(&self, image_id: &str) -> Result<Option<DeploymentInfo>> {
        let url = format!("{}/api/deployments/by-image-id/{}", self.base_url, image_id);

        debug!("Fetching deployment by image_id from registry: {}", url);

        let response = self.client.get(&url).send().await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to fetch deployment by image_id: {}",
                response.status()
            );
        }

        let deployment_response: DeploymentResponse = response
            .json()
            .await
            .context("Failed to parse deployment response")?;

        Ok(Some(deployment_response.deployment))
    }

    /// Check if registry is healthy
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/health", self.base_url);
        let response = self.client.get(&url).send().await?;
        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_client_creation() {
        let client = RegistryClient::new("http://localhost:8083".to_string());
        assert_eq!(client.base_url, "http://localhost:8083");
    }
}
