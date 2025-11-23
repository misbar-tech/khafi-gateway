//! Input builders for type-safe construction of proof inputs
//!
//! The Logic Compiler will generate custom versions of these builders
//! tailored to each customer's use case.

use anyhow::Result;
use khafi_common::{BusinessInputs, ZcashInputs};

/// Builder for Zcash payment inputs
///
/// This helps construct the universal Zcash payment data needed for all proofs
pub struct ZcashInputsBuilder {
    spending_key: Option<Vec<u8>>,
    note: Option<Vec<u8>>,
    merkle_path: Option<Vec<u8>>,
    merkle_root: Option<[u8; 32]>,
}

impl ZcashInputsBuilder {
    pub fn new() -> Self {
        Self {
            spending_key: None,
            note: None,
            merkle_path: None,
            merkle_root: None,
        }
    }

    pub fn spending_key(mut self, key: Vec<u8>) -> Self {
        self.spending_key = Some(key);
        self
    }

    pub fn note(mut self, note: Vec<u8>) -> Self {
        self.note = Some(note);
        self
    }

    pub fn merkle_path(mut self, path: Vec<u8>) -> Self {
        self.merkle_path = Some(path);
        self
    }

    pub fn merkle_root(mut self, root: [u8; 32]) -> Self {
        self.merkle_root = Some(root);
        self
    }

    pub fn build(self) -> Result<ZcashInputs> {
        Ok(ZcashInputs {
            spending_key: self
                .spending_key
                .ok_or_else(|| anyhow::anyhow!("Missing spending_key"))?,
            note: self.note.ok_or_else(|| anyhow::anyhow!("Missing note"))?,
            merkle_path: self
                .merkle_path
                .ok_or_else(|| anyhow::anyhow!("Missing merkle_path"))?,
            merkle_root: self
                .merkle_root
                .ok_or_else(|| anyhow::anyhow!("Missing merkle_root"))?,
        })
    }
}

impl Default for ZcashInputsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for business-specific inputs
///
/// **This will be CUSTOMIZED by the Logic Compiler**
///
/// For example, for a pharma use case, the compiler would generate:
/// ```ignore
/// pub struct PharmaInputsBuilder {
///     prescription: Option<Prescription>,
///     patient_dob: Option<Date>,
///     max_quantity: u32,
///     min_age: u32,
/// }
///
/// impl PharmaInputsBuilder {
///     pub fn prescription(mut self, p: Prescription) -> Self { ... }
///     pub fn patient_dob(mut self, dob: Date) -> Self { ... }
///     pub fn build(self) -> Result<BusinessInputs> {
///         // Serialize prescription and dob as private_data
///         // Serialize limits as public_params
///         ...
///     }
/// }
/// ```
pub struct BusinessInputsBuilder {
    private_data: Vec<u8>,
    public_params: Vec<u8>,
}

impl BusinessInputsBuilder {
    pub fn new() -> Self {
        Self {
            private_data: vec![],
            public_params: vec![],
        }
    }

    /// Set the private data (will be encrypted/serialized business data)
    pub fn private_data(mut self, data: Vec<u8>) -> Self {
        self.private_data = data;
        self
    }

    /// Set the public parameters (validation rules, thresholds, etc.)
    pub fn public_params(mut self, params: Vec<u8>) -> Self {
        self.public_params = params;
        self
    }

    pub fn build(self) -> BusinessInputs {
        BusinessInputs {
            private_data: self.private_data,
            public_params: self.public_params,
        }
    }
}

impl Default for BusinessInputsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zcash_inputs_builder() {
        let inputs = ZcashInputsBuilder::new()
            .spending_key(vec![1, 2, 3])
            .note(vec![4, 5, 6])
            .merkle_path(vec![7, 8, 9])
            .merkle_root([0u8; 32])
            .build()
            .unwrap();

        assert_eq!(inputs.spending_key, vec![1, 2, 3]);
        assert_eq!(inputs.note, vec![4, 5, 6]);
    }

    #[test]
    fn test_business_inputs_builder() {
        let inputs = BusinessInputsBuilder::new()
            .private_data(vec![1, 2, 3])
            .public_params(vec![4, 5, 6])
            .build();

        assert_eq!(inputs.private_data, vec![1, 2, 3]);
        assert_eq!(inputs.public_params, vec![4, 5, 6]);
    }

    #[test]
    fn test_missing_required_field() {
        let result = ZcashInputsBuilder::new()
            .spending_key(vec![1, 2, 3])
            // Missing note, merkle_path, merkle_root
            .build();

        assert!(result.is_err());
    }
}
