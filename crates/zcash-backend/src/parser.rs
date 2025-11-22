//! Transaction parsing module
//!
//! Extracts payment information from Zcash transactions.
//! Simplified for mock mode - can be extended for real Orchard parsing.

use anyhow::{Context, Result};
use khafi_common::Nullifier;
use crate::mock_node::{MockBlock, MockTransaction, MockAction};
use crate::storage::ReceivedPayment;
use tracing::debug;

/// Parser for extracting payments from blocks
pub struct Parser {
    /// The payment address we're monitoring for
    payment_address: String,
}

impl Parser {
    /// Create a new parser
    pub fn new(payment_address: String) -> Self {
        Self { payment_address }
    }

    /// Parse a block and extract payments to our address
    pub fn parse_block(&self, block: &MockBlock) -> Result<Vec<ReceivedPayment>> {
        let mut payments = Vec::new();

        debug!(
            "Parsing block {} with {} transactions",
            block.height,
            block.transactions.len()
        );

        for tx in &block.transactions {
            if let Some(payment) = self.parse_transaction(tx, block.height)? {
                payments.push(payment);
            }
        }

        debug!("Found {} payments in block {}", payments.len(), block.height);

        Ok(payments)
    }

    /// Parse a single transaction and extract payment if it's to our address
    fn parse_transaction(
        &self,
        tx: &MockTransaction,
        block_height: u32,
    ) -> Result<Option<ReceivedPayment>> {
        // Check each action in the transaction
        for action in &tx.actions {
            if action.is_our_payment {
                // This is a payment to our address
                let payment = self.parse_action(action, tx, block_height)?;
                return Ok(Some(payment));
            }
        }

        Ok(None)
    }

    /// Parse an action into a payment record
    fn parse_action(
        &self,
        action: &MockAction,
        tx: &MockTransaction,
        block_height: u32,
    ) -> Result<ReceivedPayment> {
        // Decode nullifier from hex
        let nullifier_bytes = hex::decode(&action.nullifier)
            .context("Failed to decode nullifier hex")?;

        if nullifier_bytes.len() != 32 {
            anyhow::bail!("Nullifier must be 32 bytes, got {}", nullifier_bytes.len());
        }

        let mut nullifier_array = [0u8; 32];
        nullifier_array.copy_from_slice(&nullifier_bytes);
        let nullifier = Nullifier::new(nullifier_array);

        debug!(
            "Parsed payment: nullifier={}, amount={}, tx={}",
            action.nullifier, action.amount, tx.txid
        );

        Ok(ReceivedPayment::new(
            nullifier,
            action.amount,
            tx.txid.clone(),
            block_height,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock_node::MockNode;

    #[tokio::test]
    async fn test_parse_block_with_payment() {
        let mock_node = MockNode::new("test_address".to_string());
        let parser = Parser::new("test_address".to_string());

        // Block 100000 should have a payment
        let block = mock_node.get_block(100000).await.unwrap().unwrap();
        let payments = parser.parse_block(&block).unwrap();

        assert_eq!(payments.len(), 1);
        assert_eq!(payments[0].block_height, 100000);
        assert!(payments[0].amount > 0);
    }

    #[tokio::test]
    async fn test_parse_block_without_payment() {
        let mock_node = MockNode::new("test_address".to_string());
        let parser = Parser::new("test_address".to_string());

        // Block 99999 should NOT have a payment (not divisible by 10)
        let block = mock_node.get_block(99999).await.unwrap().unwrap();
        let payments = parser.parse_block(&block).unwrap();

        assert_eq!(payments.len(), 0);
    }

    #[tokio::test]
    async fn test_nullifier_parsing() {
        let mock_node = MockNode::new("test_address".to_string());
        let parser = Parser::new("test_address".to_string());

        let block = mock_node.get_block(100000).await.unwrap().unwrap();
        let payments = parser.parse_block(&block).unwrap();

        // Verify nullifier is 32 bytes
        let payment = &payments[0];
        assert_eq!(payment.nullifier.as_bytes().len(), 32);
    }
}
