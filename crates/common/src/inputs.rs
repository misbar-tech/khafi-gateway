use crate::Nullifier;
use serde::{Deserialize, Serialize};

/// ⚠️ DEPRECATED: ZcashInputs is no longer used in the corrected architecture.
///
/// **Why:** RISC Zero zkVM runs on servers (not browsers/mobile), and spending keys
/// cannot be sent to servers without violating user privacy.
///
/// **New Architecture:**
/// - User creates Zcash transaction with their wallet (spending key stays local)
/// - User broadcasts transaction to Zcash network
/// - Zcash Backend monitors blockchain and records nullifiers in database
/// - Gateway checks payment database BEFORE running zkVM
/// - zkVM only receives nullifier + business data (no Zcash cryptography)
///
/// This struct is kept for compatibility but will be removed in future versions.
#[deprecated(
    note = "Use nullifier in GuestInputs directly. Payment verification happens in Zcash Backend, not zkVM."
)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZcashInputs {
    /// DEPRECATED: Spending keys must never leave user's wallet
    pub spending_key: Vec<u8>,
    /// DEPRECATED: Note data not needed in zkVM
    pub note: Vec<u8>,
    /// DEPRECATED: Merkle path verification happens in Zcash Backend
    pub merkle_path: Vec<u8>,
    /// DEPRECATED: Merkle root not needed in zkVM
    pub merkle_root: [u8; 32],
}

/// Business-specific inputs (varies per customer use case)
/// This is where custom validation logic operates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessInputs {
    /// Private data specific to the business use case
    /// Examples:
    /// - Pharma: encrypted prescription with patient_id, drug_name, quantity
    /// - Shipping: encrypted manifest with origin, destination, contents
    /// - Finance: encrypted KYC document with SSN, credit score
    pub private_data: Vec<u8>,

    /// Public validation parameters
    /// Examples:
    /// - Pharma: {"max_quantity": 30, "min_age": 18}
    /// - Shipping: {"sanctioned_countries": ["X", "Y"], "max_weight_kg": 1000}
    /// - Finance: {"min_credit_score": 650}
    pub public_params: Vec<u8>,
}

/// Combined inputs for RISC Zero guest program
///
/// **Architecture:** Payment verification is separated from business logic.
///
/// **Payment Verification (BEFORE zkVM):**
/// - Gateway checks nullifier exists in Zcash Backend payment database
/// - Gateway checks nullifier not already used (replay protection)
/// - Only if payment valid → run zkVM
///
/// **zkVM Guest Program (AFTER payment check):**
/// - Receives nullifier (links request to payment) + business data
/// - Verifies business logic ONLY
/// - NO Zcash cryptography in zkVM!
///
/// This is what gets passed to the guest code for verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestInputs {
    /// DEPRECATED: Use nullifier directly instead
    /// Payment verification happens in Zcash Backend, not in zkVM
    #[deprecated(note = "Payment verification moved to Zcash Backend service")]
    pub zcash: ZcashInputs,

    /// Nullifier from user's Zcash transaction (PUBLIC input)
    /// Links this API request to a specific Zcash payment
    /// Verified against payment database before zkVM execution
    pub nullifier: Nullifier,

    /// Custom: Business logic verification (varies per use case)
    /// This is the ONLY thing the zkVM verifies!
    pub business: BusinessInputs,
}

/// Output from RISC Zero guest program (written to journal)
/// This is what the verifier can read without re-running the proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestOutputs {
    /// Nullifier (passed through from input, NOT derived in zkVM)
    /// Used for replay protection by zk-verification-service
    /// This links the proof to a specific Zcash payment
    pub nullifier: Nullifier,

    /// Did the business validation pass?
    /// true = compliant (grant API access)
    /// false = non-compliant (deny API access)
    pub compliance_result: bool,

    /// Optional metadata about what was verified
    /// This can contain public proof of compliance without revealing private data
    /// Examples:
    /// - "age_verified_over_18" (without revealing actual age)
    /// - "prescription_valid_for_controlled_substance" (without revealing patient/drug)
    /// - "shipment_compliant_with_sanctions" (without revealing contents)
    pub metadata: Vec<u8>,
}

impl GuestOutputs {
    /// Create outputs indicating successful verification
    pub fn success(nullifier: Nullifier) -> Self {
        Self {
            nullifier,
            compliance_result: true,
            metadata: vec![],
        }
    }

    /// Create outputs indicating failed verification
    pub fn failure(nullifier: Nullifier) -> Self {
        Self {
            nullifier,
            compliance_result: false,
            metadata: vec![],
        }
    }

    /// Create outputs with custom metadata
    pub fn with_metadata(nullifier: Nullifier, compliance: bool, metadata: Vec<u8>) -> Self {
        Self {
            nullifier,
            compliance_result: compliance,
            metadata,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guest_outputs_creation() {
        let nullifier = Nullifier::new([1u8; 32]);
        let outputs = GuestOutputs::success(nullifier.clone());
        assert!(outputs.compliance_result);
        assert_eq!(outputs.nullifier, nullifier);
    }

    #[test]
    fn test_serialization() {
        let nullifier = Nullifier::new([1u8; 32]);
        let inputs = GuestInputs {
            zcash: ZcashInputs {
                spending_key: vec![1, 2, 3],
                note: vec![4, 5, 6],
                merkle_path: vec![7, 8, 9],
                merkle_root: [0u8; 32],
            },
            nullifier: nullifier.clone(),
            business: BusinessInputs {
                private_data: vec![10, 11, 12],
                public_params: vec![13, 14, 15],
            },
        };

        let serialized =
            bincode::serde::encode_to_vec(&inputs, bincode::config::standard()).unwrap();
        let (deserialized, _): (GuestInputs, usize) =
            bincode::serde::decode_from_slice(&serialized, bincode::config::standard()).unwrap();

        assert_eq!(inputs.zcash.spending_key, deserialized.zcash.spending_key);
        assert_eq!(
            inputs.business.private_data,
            deserialized.business.private_data
        );
    }
}
