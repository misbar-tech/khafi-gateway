//! RISC Zero prover integration

use crate::models::GuestProgram;
use anyhow::{Context, Result};
use risc0_zkvm::{default_prover, ExecutorEnv, ProverOpts, VerifierContext};
use tracing::{debug, info};

/// Proof generator
pub struct Prover {
    /// Cached guest programs by customer_id
    programs: std::collections::HashMap<String, GuestProgram>,
}

impl Prover {
    /// Create a new prover
    pub fn new() -> Self {
        Self {
            programs: std::collections::HashMap::new(),
        }
    }

    /// Load a guest program for a customer
    pub fn load_program(&mut self, program: GuestProgram) -> Result<()> {
        info!(
            "Loading guest program for customer: {} (image_id: {})",
            program.customer_id, program.image_id
        );
        self.programs.insert(program.customer_id.clone(), program);
        Ok(())
    }

    /// Generate a proof for a customer's inputs
    pub fn generate_proof(
        &self,
        customer_id: &str,
        private_inputs: &serde_json::Value,
        public_params: &serde_json::Value,
    ) -> Result<ProofResult> {
        // Get the guest program for this customer
        let program = self
            .programs
            .get(customer_id)
            .with_context(|| format!("Guest program not found for customer: {}", customer_id))?;

        info!("Generating proof for customer: {}", customer_id);
        debug!("Private inputs: {:?}", private_inputs);
        debug!("Public params: {:?}", public_params);

        // Prepare inputs for the guest program
        // The guest program expects JSON strings as inputs
        let private_json = serde_json::to_string(private_inputs)?;
        let public_json = serde_json::to_string(public_params)?;

        // Create executor environment
        let env = ExecutorEnv::builder()
            .write(&private_json)?
            .write(&public_json)?
            .build()
            .context("Failed to build executor environment")?;

        // Get the prover
        let prover = default_prover();

        // Prove execution
        let prove_info = prover
            .prove_with_ctx(
                env,
                &VerifierContext::default(),
                &program.elf_binary,
                &ProverOpts::default(),
            )
            .context("Failed to generate proof")?;

        let receipt = prove_info.receipt;

        // Extract journal (public outputs)
        let journal_bytes = receipt.journal.bytes.clone();

        // Try to deserialize journal as JSON Value
        let outputs: serde_json::Value = if journal_bytes.is_empty() {
            serde_json::json!({})
        } else {
            // The guest program commits JSON-serialized outputs
            serde_json::from_slice(&journal_bytes).unwrap_or_else(|_| {
                // Fallback: return as hex if not valid JSON
                serde_json::json!({
                    "raw_journal": hex::encode(&journal_bytes)
                })
            })
        };

        // Serialize receipt using bincode 2.x API
        let proof_bytes = bincode::serde::encode_to_vec(&receipt, bincode::config::standard())?;

        info!(
            "Proof generated successfully for customer: {} ({} bytes)",
            customer_id,
            proof_bytes.len()
        );

        Ok(ProofResult {
            proof: hex::encode(proof_bytes),
            image_id: program.image_id.clone(),
            outputs,
        })
    }

    /// Get the number of loaded programs
    pub fn program_count(&self) -> usize {
        self.programs.len()
    }

    /// Check if a customer has a loaded program
    pub fn has_program(&self, customer_id: &str) -> bool {
        self.programs.contains_key(customer_id)
    }
}

/// Result of proof generation
pub struct ProofResult {
    /// Hex-encoded proof (serialized Receipt)
    pub proof: String,

    /// Image ID used for this proof
    pub image_id: String,

    /// Public outputs from the guest program
    pub outputs: serde_json::Value,
}

impl Default for Prover {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prover_creation() {
        let prover = Prover::new();
        assert_eq!(prover.program_count(), 0);
    }

    #[test]
    fn test_has_program() {
        let mut prover = Prover::new();
        assert!(!prover.has_program("customer-123"));

        let program = GuestProgram {
            customer_id: "customer-123".to_string(),
            image_id: "image-abc".to_string(),
            elf_path: "/path/to/guest.elf".to_string(),
            elf_binary: vec![],
        };

        prover.load_program(program).unwrap();
        assert!(prover.has_program("customer-123"));
        assert_eq!(prover.program_count(), 1);
    }
}
