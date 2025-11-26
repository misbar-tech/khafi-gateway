//! Data models for Image ID Registry

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Customer deployment record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerDeployment {
    /// Unique customer identifier
    pub customer_id: String,

    /// RISC Zero Image ID (32-byte hash as hex string)
    pub image_id: String,

    /// Path to the built guest program ELF file
    pub guest_program_path: String,

    /// When this deployment was created
    pub created_at: DateTime<Utc>,

    /// Optional metadata about the deployment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<DeploymentMetadata>,
}

/// Optional metadata for a deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentMetadata {
    /// Use case name from DSL
    pub use_case: String,

    /// Description from DSL
    pub description: String,

    /// DSL version
    pub version: String,
}

impl CustomerDeployment {
    /// Create a new deployment record
    pub fn new(
        customer_id: String,
        image_id: String,
        guest_program_path: String,
        metadata: Option<DeploymentMetadata>,
    ) -> Self {
        Self {
            customer_id,
            image_id,
            guest_program_path,
            created_at: Utc::now(),
            metadata,
        }
    }
}
