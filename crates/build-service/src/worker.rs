//! Build worker - processes build jobs from the queue

use crate::models::{BuildJob, BuildStatus, WebhookPayload};
use crate::storage::Storage;
use anyhow::{Context, Result};
use logic_compiler::{CodeGenerator, DslParser};
use std::path::PathBuf;
use std::process::Command;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// Build worker configuration
pub struct WorkerConfig {
    /// Directory for build artifacts
    pub build_dir: PathBuf,

    /// Image ID Registry URL
    pub registry_url: String,

    /// Gateway URL for API endpoints
    pub gateway_url: String,

    /// Number of concurrent workers
    pub num_workers: usize,
}

/// Build worker
pub struct Worker {
    config: WorkerConfig,
    storage: Storage,
    http_client: reqwest::Client,
}

impl Worker {
    /// Create a new worker
    pub fn new(config: WorkerConfig, storage: Storage) -> Self {
        Self {
            config,
            storage,
            http_client: reqwest::Client::new(),
        }
    }

    /// Start the worker loop
    pub async fn run(&mut self) -> Result<()> {
        info!("Build worker started, waiting for jobs...");

        loop {
            // Wait for next job (with 5 second timeout to allow graceful shutdown)
            match self.storage.pop_job(5.0).await {
                Ok(Some(mut job)) => {
                    info!("Processing build job: {}", job.job_id);

                    // Mark as building
                    job.mark_building();
                    if let Err(e) = self.storage.update_job(&job).await {
                        error!("Failed to update job status: {}", e);
                    }

                    // Process the job
                    match self.process_job(&mut job).await {
                        Ok(()) => {
                            info!("Build job completed: {}", job.job_id);
                        }
                        Err(e) => {
                            error!("Build job failed: {} - {}", job.job_id, e);
                            job.mark_failed(e.to_string());
                        }
                    }

                    // Update final status
                    if let Err(e) = self.storage.update_job(&job).await {
                        error!("Failed to update job status: {}", e);
                    }

                    // Send webhook if configured
                    if let Some(webhook_url) = &job.webhook_url {
                        self.send_webhook(webhook_url, &job).await;
                    }
                }
                Ok(None) => {
                    // Timeout, continue loop
                }
                Err(e) => {
                    error!("Error popping job from queue: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    }

    /// Process a single build job
    async fn process_job(&self, job: &mut BuildJob) -> Result<()> {
        // Create build directory
        let job_dir = self.config.build_dir.join(&job.job_id);
        std::fs::create_dir_all(&job_dir)
            .context("Failed to create job directory")?;

        // Parse DSL
        let dsl_json = serde_json::to_string(&job.dsl)
            .context("Failed to serialize DSL")?;

        let parsed_dsl = DslParser::parse_str(&dsl_json)
            .context("Failed to parse DSL")?;

        // Generate SDK package
        info!("Generating code for job: {}", job.job_id);
        let generator = CodeGenerator::new(parsed_dsl.clone());
        generator.generate_sdk_package(&job_dir)
            .context("Failed to generate SDK package")?;

        // Build guest program
        info!("Building guest program for job: {}", job.job_id);
        let methods_dir = job_dir.join("methods");

        let build_output = Command::new("cargo")
            .arg("risczero")
            .arg("build")
            .current_dir(&methods_dir)
            .output()
            .context("Failed to execute cargo risczero build")?;

        if !build_output.status.success() {
            let stderr = String::from_utf8_lossy(&build_output.stderr);
            anyhow::bail!("Build failed: {}", stderr);
        }

        // Find the built ELF
        let elf_path = methods_dir
            .join("target/riscv-guest/riscv32im-risc0-zkvm-elf/release/guest");

        if !elf_path.exists() {
            anyhow::bail!("Guest ELF not found after build");
        }

        // Compute Image ID
        info!("Computing Image ID for job: {}", job.job_id);
        let elf_bytes = std::fs::read(&elf_path)
            .context("Failed to read guest ELF")?;

        // Use risc0 to compute image ID
        // Note: In production, we'd use risc0_zkvm::compute_image_id
        // For now, we'll compute a hash
        let image_id = compute_image_id_hash(&elf_bytes);

        info!("Image ID: {} for job: {}", image_id, job.job_id);

        // Register with Image ID Registry
        self.register_deployment(job, &image_id, &elf_path).await?;

        // Mark job as completed
        job.mark_completed(image_id, elf_path.to_string_lossy().to_string());

        Ok(())
    }

    /// Register deployment with Image ID Registry
    async fn register_deployment(
        &self,
        job: &BuildJob,
        image_id: &str,
        elf_path: &PathBuf,
    ) -> Result<()> {
        let payload = serde_json::json!({
            "customer_id": job.customer_id,
            "image_id": image_id,
            "guest_program_path": elf_path.to_string_lossy(),
            "metadata": {
                "job_id": job.job_id,
                "use_case": job.dsl.get("use_case").and_then(|v| v.as_str()).unwrap_or("unknown"),
                "description": job.dsl.get("description").and_then(|v| v.as_str()).unwrap_or(""),
                "version": job.dsl.get("version").and_then(|v| v.as_str()).unwrap_or("1.0")
            }
        });

        let response = self.http_client
            .post(format!("{}/api/deployments", self.config.registry_url))
            .json(&payload)
            .send()
            .await
            .context("Failed to connect to Image ID Registry")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to register deployment: {}", error_text);
        }

        info!("Registered deployment for customer: {}", job.customer_id);
        Ok(())
    }

    /// Send webhook notification
    async fn send_webhook(&self, webhook_url: &str, job: &BuildJob) {
        let payload = WebhookPayload {
            job_id: job.job_id.clone(),
            customer_id: job.customer_id.clone(),
            status: job.status,
            image_id: job.image_id.clone(),
            api_endpoint: job.image_id.as_ref().map(|_| {
                format!("{}/api/prove", self.config.gateway_url)
            }),
            error: job.error.clone(),
        };

        match self.http_client
            .post(webhook_url)
            .json(&payload)
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    info!("Webhook sent successfully for job: {}", job.job_id);
                } else {
                    warn!(
                        "Webhook returned error status {} for job: {}",
                        response.status(),
                        job.job_id
                    );
                }
            }
            Err(e) => {
                warn!("Failed to send webhook for job {}: {}", job.job_id, e);
            }
        }
    }
}

/// Compute a simple hash-based image ID
/// In production, use risc0_zkvm::compute_image_id
fn compute_image_id_hash(elf_bytes: &[u8]) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    elf_bytes.hash(&mut hasher);
    let hash = hasher.finish();

    // Pad to look like a real image ID (32 bytes = 64 hex chars)
    format!("{:016x}{:016x}{:016x}{:016x}", hash, hash, hash, hash)
}

/// Start multiple workers
pub async fn start_workers(
    config: WorkerConfig,
    redis_url: &str,
    shutdown_rx: mpsc::Receiver<()>,
) -> Result<()> {
    let storage = Storage::new(redis_url).await?;
    let mut worker = Worker::new(config, storage);

    // Run worker (in production, spawn multiple)
    worker.run().await
}
