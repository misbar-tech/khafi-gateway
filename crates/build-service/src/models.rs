//! Data models for Build Service

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Build job status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildStatus {
    /// Job is queued, waiting to be processed
    Queued,
    /// Job is currently being built
    Building,
    /// Build completed successfully
    Completed,
    /// Build failed
    Failed,
}

/// A build job in the queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildJob {
    /// Unique job identifier
    pub job_id: String,

    /// Customer identifier
    pub customer_id: String,

    /// DSL specification (JSON)
    pub dsl: serde_json::Value,

    /// Current status
    pub status: BuildStatus,

    /// When the job was created
    pub created_at: DateTime<Utc>,

    /// When the job started building
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,

    /// When the job completed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,

    /// Resulting Image ID (if successful)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_id: Option<String>,

    /// Path to built guest program ELF
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elf_path: Option<String>,

    /// Error message (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Optional webhook URL to notify on completion
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook_url: Option<String>,
}

impl BuildJob {
    /// Create a new build job
    pub fn new(job_id: String, customer_id: String, dsl: serde_json::Value) -> Self {
        Self {
            job_id,
            customer_id,
            dsl,
            status: BuildStatus::Queued,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            image_id: None,
            elf_path: None,
            error: None,
            webhook_url: None,
        }
    }

    /// Mark job as building
    pub fn mark_building(&mut self) {
        self.status = BuildStatus::Building;
        self.started_at = Some(Utc::now());
    }

    /// Mark job as completed
    pub fn mark_completed(&mut self, image_id: String, elf_path: String) {
        self.status = BuildStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.image_id = Some(image_id);
        self.elf_path = Some(elf_path);
    }

    /// Mark job as failed
    pub fn mark_failed(&mut self, error: String) {
        self.status = BuildStatus::Failed;
        self.completed_at = Some(Utc::now());
        self.error = Some(error);
    }
}

/// Request to queue a new build
#[derive(Debug, Deserialize)]
pub struct QueueBuildRequest {
    /// Customer identifier
    pub customer_id: String,

    /// DSL specification
    pub dsl: serde_json::Value,

    /// Optional webhook URL for completion notification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook_url: Option<String>,
}

/// Response from queuing a build
#[derive(Debug, Serialize)]
pub struct QueueBuildResponse {
    /// Whether the job was queued successfully
    pub success: bool,

    /// Job ID for tracking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,

    /// Error message if queueing failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Response with job status
#[derive(Debug, Serialize)]
pub struct BuildStatusResponse {
    /// The build job details
    pub job: BuildJob,
}

/// Webhook payload sent on job completion
#[derive(Debug, Serialize)]
pub struct WebhookPayload {
    /// Job ID
    pub job_id: String,

    /// Customer ID
    pub customer_id: String,

    /// Final status
    pub status: BuildStatus,

    /// Image ID (if successful)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_id: Option<String>,

    /// API endpoint (if successful)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_endpoint: Option<String>,

    /// Error message (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
