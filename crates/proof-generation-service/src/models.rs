//! Data models for Proof Generation Service

use serde::{Deserialize, Serialize};

/// Request to generate a proof
#[derive(Debug, Deserialize)]
pub struct GenerateProofRequest {
    /// Customer identifier
    pub customer_id: String,

    /// Private inputs (will be serialized and passed to guest program)
    pub private_inputs: serde_json::Value,

    /// Public parameters (will be serialized and passed to guest program)
    pub public_params: serde_json::Value,
}

/// Response from proof generation
#[derive(Debug, Serialize)]
pub struct GenerateProofResponse {
    /// Whether proof generation succeeded
    pub success: bool,

    /// Generated proof (hex-encoded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof: Option<String>,

    /// Image ID used for this proof
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_id: Option<String>,

    /// Public outputs from the guest program
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<serde_json::Value>,

    /// Error message if generation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Guest program deployment
#[derive(Debug, Clone)]
pub struct GuestProgram {
    /// Customer identifier
    pub customer_id: String,

    /// RISC Zero Image ID
    pub image_id: String,

    /// Path to the ELF file
    pub elf_path: String,

    /// Loaded ELF binary
    pub elf_binary: Vec<u8>,
}

impl GuestProgram {
    /// Load a guest program from disk
    pub fn load(customer_id: String, image_id: String, elf_path: String) -> anyhow::Result<Self> {
        let elf_binary = std::fs::read(&elf_path)?;
        Ok(Self {
            customer_id,
            image_id,
            elf_path,
            elf_binary,
        })
    }
}
