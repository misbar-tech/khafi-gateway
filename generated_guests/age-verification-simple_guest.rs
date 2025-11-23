//! Guest program for: age_verification
//! Simple age verification - proves user is over minimum age without revealing exact age
//!
//! This code was automatically generated from a Business Rules DSL.
//! It runs inside the RISC Zero zkVM to verify business logic while
//! keeping private data hidden.

#![no_main]

use risc0_zkvm::guest::env;

use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserData {
    pub date_of_birth: String,
    pub user_id: String,
}
#[doc = r" Private inputs (hidden in the proof)"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivateInputs {
    pub user_data: UserData,
}
#[doc = r" Public parameters (visible to verifier)"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicParams {
    pub min_age: u32,
}
#[doc = r" Outputs from the verification (public)"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Outputs {
    #[doc = r" Whether validation passed"]
    pub compliance_result: bool,
    pub user_hash: Vec<u8>,
}

#[doc = r" Calculate age from date of birth (ISO 8601 format: YYYY-MM-DD)"]
fn calculate_age(dob: &str) -> u32 {
    let parts: Vec<&str> = dob.split('-').collect();
    if parts.len() != 3 {
        return 0;
    }
    let birth_year: u32 = parts[0].parse().unwrap_or(0);
    let birth_month: u32 = parts[1].parse().unwrap_or(1);
    let birth_day: u32 = parts[2].parse().unwrap_or(1);
    let current_year: u32 = 2024;
    let current_month: u32 = 1;
    let current_day: u32 = 1;
    let mut age = current_year - birth_year;
    if current_month < birth_month || (current_month == birth_month && current_day < birth_day) {
        age -= 1;
    }
    age
}
#[doc = r" Placeholder for signature verification"]
#[doc = r" TODO: Replace with actual cryptographic verification"]
fn verify_signature_placeholder(
    _message: &[u8],
    _signature: &[u8],
    _public_key: &[u8],
    algorithm: &str,
) -> bool {
    match algorithm {
        "ed25519" | "ecdsa" | "rsa" => true,
        _ => false,
    }
}

#[doc = r" Perform all validation checks"]
fn validate_all(private_inputs: &PrivateInputs, public_params: &PublicParams) -> bool {
    {
        let min_age = public_params.min_age;
        let dob = &private_inputs.date_of_birth;
        let age = calculate_age(dob);
        if age < min_age {
            return false;
        }
    }
    true
}

/// Main entry point for the guest program
fn main() {
    // Read private inputs
    let private_inputs: PrivateInputs = env::read();

    // Read public parameters
    let public_params: PublicParams = env::read();

    // Perform all validation checks
    let compliance_result = validate_all(&private_inputs, &public_params);

    // Create output
    let outputs = Outputs {
        compliance_result,
        // TODO: Add any additional output fields from DSL
    };

    // Commit output to the journal (this becomes the public output of the proof)
    env::commit(&outputs);
}
