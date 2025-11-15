use serde::{Deserialize, Serialize};

/// A wrapper around a RISC Zero receipt (proof)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    /// Serialized RISC Zero receipt
    pub inner: Vec<u8>,
    /// Image ID of the guest program that generated this proof
    pub image_id: [u8; 32],
}

impl Receipt {
    /// Create a new receipt
    pub fn new(inner: Vec<u8>, image_id: [u8; 32]) -> Self {
        Self { inner, image_id }
    }

    /// Get the image ID as hex string
    pub fn image_id_hex(&self) -> String {
        hex::encode(self.image_id)
    }

    /// Get the size of the serialized proof
    pub fn proof_size(&self) -> usize {
        self.inner.len()
    }

    /// Verify the RISC Zero proof
    ///
    /// This validates the cryptographic proof and checks that it was generated
    /// by the expected guest program (via Image ID).
    ///
    /// # Arguments
    /// * `expected_image_id` - The Image ID of the expected guest program
    ///
    /// # Returns
    /// * `Ok(())` if verification succeeds
    /// * `Err` if verification fails or Image ID doesn't match
    pub fn verify(&self, expected_image_id: &[u8; 32]) -> crate::Result<()> {
        // Deserialize the RISC Zero receipt
        let (risc0_receipt, _): (risc0_zkvm::Receipt, usize) =
            bincode::serde::decode_from_slice(&self.inner, bincode::config::standard())?;

        // Verify the proof cryptographically
        risc0_receipt
            .verify(*expected_image_id)
            .map_err(|e| crate::Error::InvalidProof(e.to_string()))?;

        Ok(())
    }

    /// Get the journal (public outputs) from the receipt
    ///
    /// The journal contains the data that the guest program wrote via `env::commit()`.
    ///
    /// # Returns
    /// * The journal bytes if successful
    pub fn journal(&self) -> crate::Result<Vec<u8>> {
        let (risc0_receipt, _): (risc0_zkvm::Receipt, usize) =
            bincode::serde::decode_from_slice(&self.inner, bincode::config::standard())?;

        Ok(risc0_receipt.journal.bytes)
    }

    /// Verify the proof and decode the outputs in one step
    ///
    /// This is a convenience method that combines verification and journal extraction.
    ///
    /// # Arguments
    /// * `expected_image_id` - The Image ID of the expected guest program
    ///
    /// # Returns
    /// * The deserialized GuestOutputs if successful
    pub fn verify_and_decode(&self, expected_image_id: &[u8; 32]) -> crate::Result<crate::GuestOutputs> {
        // First verify the proof
        self.verify(expected_image_id)?;

        // Then extract and deserialize the outputs
        let journal_bytes = self.journal()?;
        let (outputs, _): (crate::GuestOutputs, usize) =
            bincode::serde::decode_from_slice(&journal_bytes, bincode::config::standard())?;

        Ok(outputs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_receipt_creation() {
        let receipt = Receipt::new(vec![1, 2, 3], [42u8; 32]);
        assert_eq!(receipt.proof_size(), 3);
        assert_eq!(receipt.image_id[0], 42);
    }
}
