use crate::Nullifier;
use serde::{Deserialize, Serialize};

/// Zcash payment inputs (universal across all customer SDKs)
/// These prove that a valid Zcash shielded payment was made
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZcashInputs {
    /// Private: Spending key (reveals ability to spend the note)
    pub spending_key: Vec<u8>,
    /// Private: Note data (contains value, recipient, etc.)
    pub note: Vec<u8>,
    /// Private: Merkle path proving note is in the commitment tree
    pub merkle_path: Vec<u8>,
    /// Public: Merkle root of the commitment tree (from zcash-backend service)
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
/// This is what gets passed to the guest code for verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestInputs {
    /// Universal: Zcash payment verification
    pub zcash: ZcashInputs,
    /// Custom: Business logic verification
    pub business: BusinessInputs,
}

/// Output from RISC Zero guest program (written to journal)
/// This is what the verifier can read without re-running the proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestOutputs {
    /// Nullifier derived from the Zcash note (prevents replay attacks)
    pub nullifier: Nullifier,

    /// Did the business validation pass?
    /// true = compliant, false = non-compliant
    pub compliance_result: bool,

    /// Optional metadata about what was verified
    /// This can contain public proof of compliance without revealing private data
    /// Example: "prescription_valid_for_patient_over_18" without revealing actual age
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
        let inputs = GuestInputs {
            zcash: ZcashInputs {
                spending_key: vec![1, 2, 3],
                note: vec![4, 5, 6],
                merkle_path: vec![7, 8, 9],
                merkle_root: [0u8; 32],
            },
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
