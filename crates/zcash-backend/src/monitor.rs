//! Blockchain monitoring module
//!
//! Continuously polls the Zcash node for new blocks and processes payments.

use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};

use crate::config::Config;
use crate::mock_node::MockNode;
use crate::parser::Parser;
use crate::storage::Storage;

/// Blockchain monitor
pub struct Monitor {
    /// Mock Zcash node client
    node: Arc<MockNode>,

    /// Transaction parser
    parser: Parser,

    /// Storage client
    storage: Storage,

    /// Configuration
    config: Config,

    /// Last processed block height
    last_processed_height: u32,
}

impl Monitor {
    /// Create a new monitor
    pub async fn new(config: Config) -> Result<Self> {
        let node = Arc::new(MockNode::new(config.payment_address.clone()));
        let parser = Parser::new(config.payment_address.clone());
        let mut storage = Storage::new(&config.redis_url).await?;

        // Get the latest block height from storage, or start from current chain height
        let last_processed_height = storage
            .get_latest_block_height()
            .await?
            .unwrap_or_else(|| {
                info!("No previous block height found, will start from current chain height");
                0
            });

        info!(
            "Monitor initialized, starting from block height {}",
            last_processed_height
        );

        Ok(Self {
            node,
            parser,
            storage,
            config,
            last_processed_height,
        })
    }

    /// Start the monitoring loop
    ///
    /// This runs indefinitely, polling for new blocks at the configured interval.
    pub async fn start(mut self) -> Result<()> {
        info!(
            "Starting blockchain monitor (polling every {} seconds)",
            self.config.polling_interval_secs
        );

        loop {
            if let Err(e) = self.poll_once().await {
                error!("Error polling blockchain: {:#}", e);
                // Continue despite errors - don't crash the monitor
            }

            // Wait before next poll
            sleep(Duration::from_secs(self.config.polling_interval_secs)).await;

            // In mock mode, advance the chain to simulate new blocks
            if self.config.mock_mode {
                self.node.advance_chain().await;
            }
        }
    }

    /// Poll for new blocks once
    async fn poll_once(&mut self) -> Result<()> {
        // Get current blockchain height
        let current_height = self.node.get_block_count().await?;

        if current_height <= self.last_processed_height {
            info!(
                "No new blocks (current: {}, last processed: {})",
                current_height, self.last_processed_height
            );
            return Ok(());
        }

        info!(
            "Processing blocks {} to {}",
            self.last_processed_height + 1,
            current_height
        );

        // Process each new block
        for height in (self.last_processed_height + 1)..=current_height {
            self.process_block(height).await?;
        }

        self.last_processed_height = current_height;

        Ok(())
    }

    /// Process a single block
    async fn process_block(&mut self, height: u32) -> Result<()> {
        info!("Processing block {}", height);

        // Fetch block from node
        let block = match self.node.get_block(height).await? {
            Some(block) => block,
            None => {
                warn!("Block {} not found, skipping", height);
                return Ok(());
            }
        };

        // Parse block to extract payments
        let payments = self.parser.parse_block(&block)?;

        if payments.is_empty() {
            info!("No payments found in block {}", height);
            return Ok(());
        }

        info!(
            "Found {} payment(s) in block {}",
            payments.len(),
            height
        );

        // Store each payment
        for payment in payments {
            match self.storage.insert_payment(&payment).await {
                Ok(true) => {
                    info!(
                        "Stored payment: {} ZEC from tx {}",
                        payment.amount as f64 / 100_000_000.0,
                        payment.tx_id
                    );
                }
                Ok(false) => {
                    warn!(
                        "Payment already exists: {}",
                        payment.nullifier.to_hex()
                    );
                }
                Err(e) => {
                    error!("Failed to store payment: {:#}", e);
                    // Continue processing other payments
                }
            }
        }

        Ok(())
    }

    /// Get a reference to the mock node (for testing)
    #[cfg(test)]
    pub fn node(&self) -> Arc<MockNode> {
        Arc::clone(&self.node)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires Redis
    async fn test_monitor_processes_blocks() {
        // Set up test config
        std::env::set_var("REDIS_URL", "redis://localhost:6379");
        std::env::set_var("MOCK_MODE", "true");
        std::env::set_var("PAYMENT_ADDRESS", "test_address");

        let config = Config::from_env().unwrap();
        let mut monitor = Monitor::new(config).await.unwrap();

        // Process a single poll
        monitor.poll_once().await.unwrap();

        // Verify last_processed_height was updated
        assert!(monitor.last_processed_height > 0);
    }
}
