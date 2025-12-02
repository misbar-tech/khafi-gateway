//! ZK Verification Service
//!
//! gRPC service that verifies RISC Zero proofs and checks nullifiers.
//! Implements Envoy's ExtAuth interface with optional payment verification.

pub mod config;
pub mod nullifier;
pub mod payment;
pub mod service;
