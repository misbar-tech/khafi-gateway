//! Redis storage for build job queue

use crate::models::{BuildJob, BuildStatus};
use anyhow::{Context, Result};
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use tracing::{debug, info};

/// Storage backend for build jobs
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

    /// Queue a new build job
    pub async fn queue_job(&mut self, job: &BuildJob) -> Result<()> {
        let key = format!("build:job:{}", job.job_id);

        // Serialize job
        let json = serde_json::to_string(job)
            .context("Failed to serialize job")?;

        // Store job
        self.conn.set(&key, &json).await?;

        // Add to queue
        self.conn.rpush("build:queue", &job.job_id).await?;

        // Add to customer's jobs index
        let customer_key = format!("build:customer:{}", job.customer_id);
        self.conn.sadd(&customer_key, &job.job_id).await?;

        info!("Queued build job: {} for customer: {}", job.job_id, job.customer_id);
        Ok(())
    }

    /// Get a job by ID
    pub async fn get_job(&mut self, job_id: &str) -> Result<Option<BuildJob>> {
        let key = format!("build:job:{}", job_id);

        let json: Option<String> = self.conn.get(&key).await?;

        match json {
            Some(data) => {
                let job: BuildJob = serde_json::from_str(&data)
                    .context("Failed to deserialize job")?;
                Ok(Some(job))
            }
            None => Ok(None),
        }
    }

    /// Update a job
    pub async fn update_job(&mut self, job: &BuildJob) -> Result<()> {
        let key = format!("build:job:{}", job.job_id);

        let json = serde_json::to_string(job)
            .context("Failed to serialize job")?;

        self.conn.set(&key, json).await?;

        debug!("Updated job: {} status: {:?}", job.job_id, job.status);
        Ok(())
    }

    /// Pop next job from queue (blocking)
    pub async fn pop_job(&mut self, timeout_secs: f64) -> Result<Option<BuildJob>> {
        // BLPOP with timeout
        let result: Option<(String, String)> = self.conn
            .blpop("build:queue", timeout_secs)
            .await?;

        match result {
            Some((_, job_id)) => {
                debug!("Popped job from queue: {}", job_id);
                self.get_job(&job_id).await
            }
            None => Ok(None),
        }
    }

    /// Get all jobs for a customer
    pub async fn get_customer_jobs(&mut self, customer_id: &str) -> Result<Vec<BuildJob>> {
        let customer_key = format!("build:customer:{}", customer_id);

        let job_ids: Vec<String> = self.conn.smembers(&customer_key).await?;

        let mut jobs = Vec::new();
        for job_id in job_ids {
            if let Some(job) = self.get_job(&job_id).await? {
                jobs.push(job);
            }
        }

        // Sort by created_at descending
        jobs.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(jobs)
    }

    /// Get queue length
    pub async fn queue_length(&mut self) -> Result<usize> {
        let len: usize = self.conn.llen("build:queue").await?;
        Ok(len)
    }

    /// Get counts by status
    pub async fn get_stats(&mut self) -> Result<BuildStats> {
        // This is a simple implementation - for production you'd want
        // to maintain counters separately for performance
        let queue_len = self.queue_length().await?;

        Ok(BuildStats {
            queued: queue_len,
            building: 0, // Would need to track separately
            completed: 0,
            failed: 0,
        })
    }
}

/// Build statistics
#[derive(Debug, serde::Serialize)]
pub struct BuildStats {
    pub queued: usize,
    pub building: usize,
    pub completed: usize,
    pub failed: usize,
}
