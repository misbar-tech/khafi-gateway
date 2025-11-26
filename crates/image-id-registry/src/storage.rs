//! Redis storage for Image ID Registry

use crate::models::CustomerDeployment;
use anyhow::{Context, Result};
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use tracing::{debug, info};

/// Storage backend for customer deployments
pub struct Storage {
    conn: ConnectionManager,
}

impl Storage {
    /// Create a new storage instance
    pub async fn new(redis_url: &str) -> Result<Self> {
        let client = redis::Client::open(redis_url)
            .context("Failed to create Redis client")?;

        let conn = ConnectionManager::new(client)
            .await
            .context("Failed to connect to Redis")?;

        info!("Connected to Redis at {}", redis_url);

        Ok(Self { conn })
    }

    /// Register a new customer deployment
    /// Returns Ok(true) if created, Ok(false) if customer already has a deployment
    pub async fn register_deployment(&mut self, deployment: &CustomerDeployment) -> Result<bool> {
        let key = format!("deployment:{}", deployment.customer_id);

        // Check if deployment already exists
        let exists: bool = self.conn.exists(&key).await?;
        if exists {
            debug!("Deployment already exists for customer: {}", deployment.customer_id);
            return Ok(false);
        }

        // Serialize deployment
        let json = serde_json::to_string(deployment)
            .context("Failed to serialize deployment")?;

        // Store deployment
        self.conn.set(&key, json).await?;

        // Add to index
        self.conn
            .sadd("deployments:all", &deployment.customer_id)
            .await?;

        // Create reverse lookup: image_id -> customer_id
        let image_key = format!("image_id:{}", deployment.image_id);
        self.conn.set(&image_key, &deployment.customer_id).await?;

        info!("Registered deployment for customer: {}", deployment.customer_id);
        Ok(true)
    }

    /// Update an existing customer deployment
    pub async fn update_deployment(&mut self, deployment: &CustomerDeployment) -> Result<bool> {
        let key = format!("deployment:{}", deployment.customer_id);

        // Check if deployment exists
        let exists: bool = self.conn.exists(&key).await?;
        if !exists {
            debug!("Deployment not found for customer: {}", deployment.customer_id);
            return Ok(false);
        }

        // Get old deployment to clean up old image_id mapping
        if let Ok(Some(old_deployment)) = self.get_deployment(&deployment.customer_id).await {
            if old_deployment.image_id != deployment.image_id {
                let old_image_key = format!("image_id:{}", old_deployment.image_id);
                let _: () = self.conn.del(&old_image_key).await?;
            }
        }

        // Serialize and store new deployment
        let json = serde_json::to_string(deployment)
            .context("Failed to serialize deployment")?;

        self.conn.set(&key, json).await?;

        // Update reverse lookup
        let image_key = format!("image_id:{}", deployment.image_id);
        self.conn.set(&image_key, &deployment.customer_id).await?;

        info!("Updated deployment for customer: {}", deployment.customer_id);
        Ok(true)
    }

    /// Get deployment by customer ID
    pub async fn get_deployment(&mut self, customer_id: &str) -> Result<Option<CustomerDeployment>> {
        let key = format!("deployment:{}", customer_id);

        let json: Option<String> = self.conn.get(&key).await?;

        match json {
            Some(data) => {
                let deployment: CustomerDeployment = serde_json::from_str(&data)
                    .context("Failed to deserialize deployment")?;
                Ok(Some(deployment))
            }
            None => Ok(None),
        }
    }

    /// Get deployment by Image ID
    pub async fn get_deployment_by_image_id(&mut self, image_id: &str) -> Result<Option<CustomerDeployment>> {
        let image_key = format!("image_id:{}", image_id);

        // Get customer_id from image_id lookup
        let customer_id: Option<String> = self.conn.get(&image_key).await?;

        match customer_id {
            Some(cid) => self.get_deployment(&cid).await,
            None => Ok(None),
        }
    }

    /// Delete a customer deployment
    pub async fn delete_deployment(&mut self, customer_id: &str) -> Result<bool> {
        let key = format!("deployment:{}", customer_id);

        // Get deployment to clean up image_id mapping
        if let Ok(Some(deployment)) = self.get_deployment(customer_id).await {
            let image_key = format!("image_id:{}", deployment.image_id);
            let _: () = self.conn.del(&image_key).await?;
        }

        // Delete deployment
        let deleted: bool = self.conn.del(&key).await?;

        if deleted {
            // Remove from index
            self.conn
                .srem("deployments:all", customer_id)
                .await?;

            info!("Deleted deployment for customer: {}", customer_id);
        }

        Ok(deleted)
    }

    /// List all customer IDs with deployments
    pub async fn list_customers(&mut self) -> Result<Vec<String>> {
        let customers: Vec<String> = self.conn.smembers("deployments:all").await?;
        Ok(customers)
    }

    /// Get total count of deployments
    pub async fn count_deployments(&mut self) -> Result<usize> {
        let count: usize = self.conn.scard("deployments:all").await?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DeploymentMetadata;

    async fn get_test_storage() -> Storage {
        Storage::new("redis://127.0.0.1:6379/15")
            .await
            .expect("Failed to connect to test Redis")
    }

    #[tokio::test]
    async fn test_register_and_get_deployment() {
        let mut storage = get_test_storage().await;

        let deployment = CustomerDeployment::new(
            "customer-123".to_string(),
            "image-abc-def".to_string(),
            "/path/to/guest.elf".to_string(),
            Some(DeploymentMetadata {
                use_case: "age_verification".to_string(),
                description: "Verify user age".to_string(),
                version: "1.0".to_string(),
            }),
        );

        // Register
        let created = storage.register_deployment(&deployment).await.unwrap();
        assert!(created);

        // Get by customer ID
        let retrieved = storage
            .get_deployment("customer-123")
            .await
            .unwrap()
            .expect("Deployment not found");

        assert_eq!(retrieved.customer_id, "customer-123");
        assert_eq!(retrieved.image_id, "image-abc-def");

        // Get by image ID
        let by_image = storage
            .get_deployment_by_image_id("image-abc-def")
            .await
            .unwrap()
            .expect("Deployment not found");

        assert_eq!(by_image.customer_id, "customer-123");

        // Clean up
        storage.delete_deployment("customer-123").await.unwrap();
    }

    #[tokio::test]
    async fn test_duplicate_registration() {
        let mut storage = get_test_storage().await;

        let deployment = CustomerDeployment::new(
            "customer-456".to_string(),
            "image-xyz".to_string(),
            "/path/to/guest.elf".to_string(),
            None,
        );

        // First registration should succeed
        let created = storage.register_deployment(&deployment).await.unwrap();
        assert!(created);

        // Second registration should fail
        let created_again = storage.register_deployment(&deployment).await.unwrap();
        assert!(!created_again);

        // Clean up
        storage.delete_deployment("customer-456").await.unwrap();
    }

    #[tokio::test]
    async fn test_update_deployment() {
        let mut storage = get_test_storage().await;

        let mut deployment = CustomerDeployment::new(
            "customer-789".to_string(),
            "image-old".to_string(),
            "/path/to/old.elf".to_string(),
            None,
        );

        // Register
        storage.register_deployment(&deployment).await.unwrap();

        // Update
        deployment.image_id = "image-new".to_string();
        deployment.guest_program_path = "/path/to/new.elf".to_string();

        let updated = storage.update_deployment(&deployment).await.unwrap();
        assert!(updated);

        // Verify update
        let retrieved = storage
            .get_deployment("customer-789")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(retrieved.image_id, "image-new");

        // Old image_id should not resolve
        let old_lookup = storage
            .get_deployment_by_image_id("image-old")
            .await
            .unwrap();
        assert!(old_lookup.is_none());

        // New image_id should resolve
        let new_lookup = storage
            .get_deployment_by_image_id("image-new")
            .await
            .unwrap();
        assert!(new_lookup.is_some());

        // Clean up
        storage.delete_deployment("customer-789").await.unwrap();
    }
}
