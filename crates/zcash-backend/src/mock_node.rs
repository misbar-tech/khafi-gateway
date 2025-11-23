//! Mock Zcash node client for development and testing
//!
//! Simulates a Zcash node without requiring actual blockchain connection.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::debug;

/// Mock block data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockBlock {
    pub height: u32,
    pub hash: String,
    pub time: i64,
    pub transactions: Vec<MockTransaction>,
}

/// Mock transaction data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockTransaction {
    pub txid: String,
    pub actions: Vec<MockAction>,
}

/// Mock Orchard action (simplified)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockAction {
    /// Nullifier (32 bytes as hex)
    pub nullifier: String,

    /// Payment amount in zatoshis
    pub amount: u64,

    /// Whether this payment is to our address
    pub is_our_payment: bool,
}

/// Mock Zcash node client
pub struct MockNode {
    /// Current blockchain height
    current_height: Arc<Mutex<u32>>,

    /// Address to check for payments
    payment_address: String,
}

impl MockNode {
    /// Create a new mock node
    pub fn new(payment_address: String) -> Self {
        Self {
            current_height: Arc::new(Mutex::new(100000)), // Start at height 100000
            payment_address,
        }
    }

    /// Get the current blockchain height
    pub async fn get_block_count(&self) -> Result<u32> {
        let height = *self.current_height.lock().await;
        debug!("Mock node: get_block_count() -> {}", height);
        Ok(height)
    }

    /// Get a block at the specified height
    pub async fn get_block(&self, height: u32) -> Result<Option<MockBlock>> {
        let current = *self.current_height.lock().await;

        if height > current {
            return Ok(None);
        }

        // Generate deterministic mock block
        let block = self.generate_mock_block(height).await;
        debug!(
            "Mock node: get_block({}) -> block with {} txs",
            height,
            block.transactions.len()
        );

        Ok(Some(block))
    }

    /// Advance the blockchain by one block (simulates new block being mined)
    pub async fn advance_chain(&self) {
        let mut height = self.current_height.lock().await;
        *height += 1;
        debug!("Mock node: Advanced to height {}", *height);
    }

    /// Generate a mock block with some test transactions
    async fn generate_mock_block(&self, height: u32) -> MockBlock {
        let mut transactions = vec![];

        // Every 10th block contains a payment to our address
        if height % 10 == 0 {
            transactions.push(self.generate_payment_transaction(height));
        }

        // Every 5th block contains a non-payment transaction
        if height % 5 == 0 {
            transactions.push(self.generate_other_transaction(height));
        }

        MockBlock {
            height,
            hash: format!("mock_block_hash_{:08x}", height),
            time: 1234567890 + (height as i64 * 75), // ~75 seconds per block
            transactions,
        }
    }

    /// Generate a mock transaction with payment to our address
    fn generate_payment_transaction(&self, height: u32) -> MockTransaction {
        // Generate deterministic nullifier based on block height
        let nullifier_bytes = self.generate_nullifier_bytes(height, 0);
        let nullifier_hex = hex::encode(nullifier_bytes);

        MockTransaction {
            txid: format!("mock_payment_tx_{:08x}", height),
            actions: vec![MockAction {
                nullifier: nullifier_hex,
                amount: 10000000 + (height as u64 * 1000), // Variable amount
                is_our_payment: true,
            }],
        }
    }

    /// Generate a mock transaction NOT to our address
    fn generate_other_transaction(&self, height: u32) -> MockTransaction {
        let nullifier_bytes = self.generate_nullifier_bytes(height, 1);
        let nullifier_hex = hex::encode(nullifier_bytes);

        MockTransaction {
            txid: format!("mock_other_tx_{:08x}", height),
            actions: vec![MockAction {
                nullifier: nullifier_hex,
                amount: 5000000,
                is_our_payment: false, // Not to our address
            }],
        }
    }

    /// Generate deterministic 32-byte nullifier
    fn generate_nullifier_bytes(&self, height: u32, index: u8) -> [u8; 32] {
        let mut nullifier = [0u8; 32];

        // Put height in first 4 bytes
        nullifier[0..4].copy_from_slice(&height.to_le_bytes());

        // Put index in 5th byte
        nullifier[4] = index;

        // Fill rest with pattern
        for i in 5..32 {
            nullifier[i] = ((i * 17 + height as usize * 7) % 256) as u8;
        }

        nullifier
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_node_get_block_count() {
        let node = MockNode::new("test_address".to_string());
        let height = node.get_block_count().await.unwrap();
        assert_eq!(height, 100000);
    }

    #[tokio::test]
    async fn test_mock_node_get_block() {
        let node = MockNode::new("test_address".to_string());

        let block = node.get_block(100000).await.unwrap().unwrap();
        assert_eq!(block.height, 100000);
        assert!(!block.transactions.is_empty()); // Height 100000 is divisible by 10
    }

    #[tokio::test]
    async fn test_mock_node_advance_chain() {
        let node = MockNode::new("test_address".to_string());

        let initial_height = node.get_block_count().await.unwrap();
        node.advance_chain().await;
        let new_height = node.get_block_count().await.unwrap();

        assert_eq!(new_height, initial_height + 1);
    }

    #[tokio::test]
    async fn test_payment_generation() {
        let node = MockNode::new("test_address".to_string());

        // Block 100000 should have a payment (divisible by 10)
        let block = node.get_block(100000).await.unwrap().unwrap();
        let has_payment = block
            .transactions
            .iter()
            .flat_map(|tx| &tx.actions)
            .any(|action| action.is_our_payment);

        assert!(has_payment);
    }

    #[tokio::test]
    async fn test_no_payment_for_odd_blocks() {
        let node = MockNode::new("test_address".to_string());

        // Block 99999 should NOT have a payment (not divisible by 10)
        let block = node.get_block(99999).await.unwrap().unwrap();
        let has_payment = block
            .transactions
            .iter()
            .flat_map(|tx| &tx.actions)
            .any(|action| action.is_our_payment);

        assert!(!has_payment);
    }
}
