//! RISC Zero prover module
//!
//! This module handles the actual proof generation using RISC Zero.

use khafi_common::{GuestInputs, GuestOutputs, Receipt, Result};
use risc0_zkvm::{default_prover, ExecutorEnv};

/// Generate a RISC Zero proof for the given inputs
///
/// # Arguments
/// * `inputs` - The guest program inputs (Zcash + business data)
/// * `guest_binary` - The compiled guest program ELF binary
/// * `image_id` - The expected Image ID (cryptographic hash of ELF)
///
/// # Returns
/// A Receipt containing the cryptographic proof
pub fn generate_proof(
    inputs: GuestInputs,
    guest_binary: &[u8],
    image_id: [u8; 32],
) -> Result<Receipt> {
    // Step 1: Build the executor environment with inputs
    // Serialize the inputs and write them to the guest environment
    let env = ExecutorEnv::builder()
        .write(&inputs)?
        .build()
        .map_err(|e| khafi_common::Error::RiscZero(e.to_string()))?;

    // Step 2: Get the default prover
    let prover = default_prover();

    // Step 3: Execute the guest program and generate the proof
    // This runs the guest code in the zkVM and produces a receipt
    let prove_info = prover
        .prove(env, guest_binary)
        .map_err(|e| khafi_common::Error::RiscZero(e.to_string()))?;

    // Step 4: Extract the receipt from the prove info
    let risc0_receipt = prove_info.receipt;

    // Step 5: Serialize the RISC Zero receipt for storage/transmission
    let receipt_bytes = bincode::serde::encode_to_vec(&risc0_receipt, bincode::config::standard())?;

    // Step 6: Wrap in our Receipt type
    Ok(Receipt::new(receipt_bytes, image_id))
}

/// Extract outputs from a receipt's journal
///
/// This allows reading the nullifier and compliance result without re-running the proof
///
/// # Arguments
/// * `receipt` - The Receipt containing the proof and journal
///
/// # Returns
/// The GuestOutputs that were written to the journal by the guest program
pub fn extract_outputs(receipt: &Receipt) -> Result<GuestOutputs> {
    // Step 1: Deserialize the inner RISC Zero receipt
    let (risc0_receipt, _): (risc0_zkvm::Receipt, usize) =
        bincode::serde::decode_from_slice(&receipt.inner, bincode::config::standard())?;

    // Step 2: Get the journal (public outputs)
    let journal_bytes = risc0_receipt.journal.bytes;

    // Step 3: Deserialize the GuestOutputs from the journal
    let (outputs, _): (GuestOutputs, usize) =
        bincode::serde::decode_from_slice(&journal_bytes, bincode::config::standard())?;

    Ok(outputs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use khafi_common::{ZcashInputs, BusinessInputs};

    #[test]
    fn test_placeholder_proof_generation() {
        // Note: This test requires the guest program to be built first
        // Run `cargo build -p methods` before running tests
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

        // This will fail until methods are built
        // let receipt = generate_proof(inputs, methods::GUEST_ELF, methods::GUEST_ID).unwrap();
        // assert_eq!(receipt.image_id, methods::GUEST_ID);
    }
}
