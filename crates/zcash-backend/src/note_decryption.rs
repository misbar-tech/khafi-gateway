//! Note decryption for real Zcash transactions
//!
//! Decrypts incoming Orchard and Sapling notes using viewing keys
//! to detect payments to our address.

use anyhow::{Context, Result};
use khafi_common::Nullifier;
use orchard::keys::FullViewingKey as OrchardFVK;
// Note: OrchardDomain will be used when implementing full trial decryption
use tracing::{debug, info, warn};

use crate::lightwalletd_client::proto::{CompactBlock, CompactOrchardAction, CompactTx};
use crate::storage::ReceivedPayment;

/// Decrypted note information
#[derive(Debug)]
pub struct DecryptedNote {
    /// Note value in zatoshis
    pub value: u64,
    /// Memo field (512 bytes, contains customer nullifier)
    pub memo: [u8; 512],
    /// Nullifier for this note (computed from note + FVK)
    pub nullifier: [u8; 32],
}

/// Note decryptor for detecting incoming payments
pub struct NoteDecryptor {
    /// Orchard Full Viewing Key (if configured)
    orchard_fvk: Option<OrchardFVK>,
}

impl NoteDecryptor {
    /// Create a new note decryptor from hex-encoded viewing keys
    pub fn new(orchard_fvk_hex: Option<&str>, _sapling_fvk_hex: Option<&str>) -> Result<Self> {
        let orchard_fvk = if let Some(hex) = orchard_fvk_hex {
            let bytes = hex::decode(hex).context("Invalid ORCHARD_FVK hex")?;
            if bytes.len() != 96 {
                anyhow::bail!("ORCHARD_FVK must be 96 bytes, got {}", bytes.len());
            }
            let mut fvk_bytes = [0u8; 96];
            fvk_bytes.copy_from_slice(&bytes);

            let fvk = OrchardFVK::from_bytes(&fvk_bytes);
            if fvk.is_none().into() {
                anyhow::bail!("Invalid Orchard Full Viewing Key");
            }
            Some(fvk.unwrap())
        } else {
            None
        };

        // TODO: Add Sapling FVK parsing when needed

        Ok(Self { orchard_fvk })
    }

    /// Try to decrypt a compact block and extract payments to us
    pub fn decrypt_block(&self, block: &CompactBlock) -> Result<Vec<ReceivedPayment>> {
        let mut payments = Vec::new();
        let height = block.height as u32;

        for tx in &block.vtx {
            if let Some(payment) = self.try_decrypt_tx(tx, height)? {
                payments.push(payment);
            }
        }

        if !payments.is_empty() {
            info!("Found {} payment(s) in block {}", payments.len(), height);
        }

        Ok(payments)
    }

    /// Try to decrypt a transaction and extract payment if it's to us
    fn try_decrypt_tx(&self, tx: &CompactTx, block_height: u32) -> Result<Option<ReceivedPayment>> {
        let tx_hash = hex::encode(&tx.hash);

        // Try Orchard actions first
        if let Some(fvk) = &self.orchard_fvk {
            for (idx, action) in tx.actions.iter().enumerate() {
                if let Some(note) = self.try_decrypt_orchard_action(action, fvk)? {
                    debug!(
                        "Decrypted Orchard note: tx={}, action={}, value={}",
                        tx_hash, idx, note.value
                    );

                    // Extract customer nullifier from memo
                    let customer_nullifier = self.extract_nullifier_from_memo(&note.memo)?;

                    if let Some(nullifier) = customer_nullifier {
                        return Ok(Some(ReceivedPayment::new(
                            nullifier,
                            note.value,
                            tx_hash.clone(),
                            block_height,
                        )));
                    } else {
                        warn!(
                            "Payment detected but memo doesn't contain valid nullifier: tx={}",
                            tx_hash
                        );
                    }
                }
            }
        }

        // TODO: Try Sapling outputs when needed

        Ok(None)
    }

    /// Try to decrypt a single Orchard action
    fn try_decrypt_orchard_action(
        &self,
        action: &CompactOrchardAction,
        _fvk: &OrchardFVK,
    ) -> Result<Option<DecryptedNote>> {
        // Compact blocks only contain the first 52 bytes of the ciphertext
        // This is enough to decrypt the note plaintext for trial decryption

        if action.ciphertext.len() < 52 {
            return Ok(None);
        }

        // For compact block trial decryption, we need:
        // - ephemeral_key (32 bytes)
        // - ciphertext (first 52 bytes in compact format)
        // - cmx (note commitment, 32 bytes)

        // The orchard crate's compact decryption API requires the domain
        // and uses the Incoming Viewing Key (IVK) derived from FVK

        // TODO: Implement actual trial decryption using orchard::note_encryption
        // This requires constructing the proper domain and using try_compact_note_decryption

        // For now, return None - we'll implement full decryption next
        debug!(
            "Orchard action: nullifier={}, cmx={}, ephemeral_key={}, ciphertext_len={}",
            hex::encode(&action.nullifier),
            hex::encode(&action.cmx),
            hex::encode(&action.ephemeral_key),
            action.ciphertext.len()
        );

        Ok(None)
    }

    /// Extract customer nullifier from memo field
    ///
    /// Expected memo format:
    /// - First 32 bytes: Customer's nullifier (hex would be 64 chars, but stored as raw bytes)
    /// - Or: "nullifier:" prefix followed by 64 hex characters
    fn extract_nullifier_from_memo(&self, memo: &[u8; 512]) -> Result<Option<Nullifier>> {
        // Check if memo starts with raw 32-byte nullifier (non-zero)
        if memo[0..32].iter().any(|&b| b != 0) {
            let mut nullifier_bytes = [0u8; 32];
            nullifier_bytes.copy_from_slice(&memo[0..32]);
            return Ok(Some(Nullifier::new(nullifier_bytes)));
        }

        // Check for "nullifier:" prefix with hex encoding
        let memo_str = String::from_utf8_lossy(memo);
        let trimmed = memo_str.trim_matches(char::from(0)).trim();

        if let Some(hex_str) = trimmed.strip_prefix("nullifier:") {
            let hex_str = hex_str.trim();
            if hex_str.len() == 64 {
                if let Ok(bytes) = hex::decode(hex_str) {
                    let mut nullifier_bytes = [0u8; 32];
                    nullifier_bytes.copy_from_slice(&bytes);
                    return Ok(Some(Nullifier::new(nullifier_bytes)));
                }
            }
        }

        // Check if memo is just a 64-character hex string
        if trimmed.len() == 64 {
            if let Ok(bytes) = hex::decode(trimmed) {
                let mut nullifier_bytes = [0u8; 32];
                nullifier_bytes.copy_from_slice(&bytes);
                return Ok(Some(Nullifier::new(nullifier_bytes)));
            }
        }

        Ok(None)
    }

    /// Check if we have any viewing keys configured
    pub fn has_viewing_keys(&self) -> bool {
        self.orchard_fvk.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_nullifier_raw_bytes() {
        let decryptor = NoteDecryptor::new(None, None).unwrap();

        let mut memo = [0u8; 512];
        // Put a test nullifier in first 32 bytes
        for i in 0..32 {
            memo[i] = (i + 1) as u8;
        }

        let result = decryptor.extract_nullifier_from_memo(&memo).unwrap();
        assert!(result.is_some());

        let nullifier = result.unwrap();
        assert_eq!(nullifier.as_bytes()[0], 1);
        assert_eq!(nullifier.as_bytes()[31], 32);
    }

    #[test]
    fn test_extract_nullifier_hex_string() {
        let decryptor = NoteDecryptor::new(None, None).unwrap();

        let mut memo = [0u8; 512];
        let hex_nullifier = "0102030405060708091011121314151617181920212223242526272829303132";
        memo[..64].copy_from_slice(hex_nullifier.as_bytes());

        let result = decryptor.extract_nullifier_from_memo(&memo).unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_extract_nullifier_with_prefix() {
        let decryptor = NoteDecryptor::new(None, None).unwrap();

        let mut memo = [0u8; 512];
        let prefixed = "nullifier:0102030405060708091011121314151617181920212223242526272829303132";
        memo[..prefixed.len()].copy_from_slice(prefixed.as_bytes());

        let result = decryptor.extract_nullifier_from_memo(&memo).unwrap();
        assert!(result.is_some());
    }
}
