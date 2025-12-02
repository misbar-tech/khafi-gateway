//! Lightwalletd gRPC client for Zcash blockchain interaction
//!
//! This module provides a client to connect to a lightwalletd instance
//! and retrieve blockchain data for payment verification.

use anyhow::{Context, Result};
use tonic::transport::Channel;
use tracing::{debug, info, warn};

use crate::mock_node::{MockAction, MockBlock, MockTransaction};

// Include the generated protobuf code
pub mod proto {
    tonic::include_proto!("cash.z.wallet.sdk.rpc");
}

use proto::compact_tx_streamer_client::CompactTxStreamerClient;
use proto::{BlockId, BlockRange, ChainSpec, Empty};

/// Lightwalletd client for production use
pub struct LightwalletdClient {
    /// gRPC client
    client: CompactTxStreamerClient<Channel>,

    /// Cached server info
    chain_name: String,
}

impl LightwalletdClient {
    /// Create a new lightwalletd client
    ///
    /// # Arguments
    /// * `endpoint` - The gRPC endpoint (e.g., "https://testnet.lightwalletd.com:9067")
    pub async fn new(endpoint: &str) -> Result<Self> {
        info!("Connecting to lightwalletd at {}", endpoint);

        let channel = Channel::from_shared(endpoint.to_string())
            .context("Invalid endpoint URL")?
            .connect()
            .await
            .context("Failed to connect to lightwalletd")?;

        let mut client = CompactTxStreamerClient::new(channel);

        // Get server info to verify connection
        let info = client
            .get_lightd_info(Empty {})
            .await
            .context("Failed to get lightd info")?
            .into_inner();

        info!(
            "Connected to lightwalletd: version={}, chain={}, height={}",
            info.version, info.chain_name, info.block_height
        );

        Ok(Self {
            client,
            chain_name: info.chain_name,
        })
    }

    /// Get the current blockchain height
    pub async fn get_block_count(&mut self) -> Result<u32> {
        let response = self
            .client
            .get_latest_block(ChainSpec {})
            .await
            .context("Failed to get latest block")?
            .into_inner();

        let height = response.height as u32;
        debug!("Lightwalletd: get_block_count() -> {}", height);

        Ok(height)
    }

    /// Get a block at the specified height
    ///
    /// Returns the block converted to our MockBlock format for compatibility
    /// with the existing parser.
    pub async fn get_block(&mut self, height: u32) -> Result<Option<MockBlock>> {
        let block_id = BlockId {
            height: height as u64,
            hash: vec![],
        };

        let response = match self.client.get_block(block_id).await {
            Ok(response) => response.into_inner(),
            Err(status) if status.code() == tonic::Code::NotFound => {
                return Ok(None);
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to get block: {}", e));
            }
        };

        // Convert CompactBlock to MockBlock format
        let block = self.convert_compact_block(response);

        debug!(
            "Lightwalletd: get_block({}) -> block with {} txs",
            height,
            block.transactions.len()
        );

        Ok(Some(block))
    }

    /// Get a range of blocks (streaming)
    pub async fn get_block_range(&mut self, start: u32, end: u32) -> Result<Vec<MockBlock>> {
        let range = BlockRange {
            start: Some(BlockId {
                height: start as u64,
                hash: vec![],
            }),
            end: Some(BlockId {
                height: end as u64,
                hash: vec![],
            }),
        };

        let mut stream = self
            .client
            .get_block_range(range)
            .await
            .context("Failed to get block range")?
            .into_inner();

        let mut blocks = Vec::new();
        while let Some(compact_block) = stream.message().await? {
            blocks.push(self.convert_compact_block(compact_block));
        }

        info!(
            "Lightwalletd: get_block_range({}, {}) -> {} blocks",
            start,
            end,
            blocks.len()
        );

        Ok(blocks)
    }

    /// Get server chain name
    pub fn chain_name(&self) -> &str {
        &self.chain_name
    }

    /// Convert a CompactBlock from lightwalletd to our MockBlock format
    fn convert_compact_block(&self, block: proto::CompactBlock) -> MockBlock {
        let transactions: Vec<MockTransaction> = block
            .vtx
            .into_iter()
            .map(|tx| self.convert_compact_tx(tx))
            .collect();

        MockBlock {
            height: block.height as u32,
            hash: hex::encode(&block.hash),
            time: block.time as i64,
            transactions,
        }
    }

    /// Convert a CompactTx to our MockTransaction format
    fn convert_compact_tx(&self, tx: proto::CompactTx) -> MockTransaction {
        let mut actions = Vec::new();

        // Add Sapling spends (nullifiers)
        for spend in tx.spends {
            actions.push(MockAction {
                nullifier: hex::encode(&spend.nf),
                amount: 0, // Sapling spends don't have amount in compact format
                is_our_payment: false, // Will be determined by parser
            });
        }

        // Add Orchard actions (nullifiers)
        for action in tx.actions {
            actions.push(MockAction {
                nullifier: hex::encode(&action.nullifier),
                amount: 0, // Amount is encrypted, determined by decryption
                is_our_payment: false, // Will be determined by parser
            });
        }

        MockTransaction {
            txid: hex::encode(&tx.hash),
            actions,
        }
    }
}

/// Enum to choose between mock and real lightwalletd client
pub enum ZcashNode {
    Mock(crate::mock_node::MockNode),
    Lightwalletd(LightwalletdClient),
}

impl ZcashNode {
    /// Get the current blockchain height
    pub async fn get_block_count(&mut self) -> Result<u32> {
        match self {
            ZcashNode::Mock(node) => node.get_block_count().await,
            ZcashNode::Lightwalletd(client) => client.get_block_count().await,
        }
    }

    /// Get a block at the specified height
    pub async fn get_block(&mut self, height: u32) -> Result<Option<MockBlock>> {
        match self {
            ZcashNode::Mock(node) => node.get_block(height).await,
            ZcashNode::Lightwalletd(client) => client.get_block(height).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires running lightwalletd
    async fn test_connect_to_testnet() {
        // Note: This requires access to a lightwalletd instance
        let client = LightwalletdClient::new("http://localhost:9067").await;
        assert!(client.is_ok());
    }
}
