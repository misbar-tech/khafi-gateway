//! Zcash Backend Service
//!
//! Blockchain monitoring and payment verification service for the Khafi Gateway.
//!
//! ## Architecture
//!
//! The Zcash Backend monitors the Zcash blockchain for incoming payments and stores
//! payment nullifiers in Redis. The Gateway queries this service to verify payments
//! before granting API access.
//!
//! **Components:**
//! - `storage`: Redis operations for payment data
//! - `monitor`: Blockchain monitoring loop
//! - `parser`: Transaction parsing and nullifier extraction
//! - `mock_node`: Mock Zcash node for development/testing
//! - `api`: REST API for payment queries
//! - `config`: Configuration management
//!
//! **Data Flow:**
//! 1. User creates Zcash transaction → broadcasts to blockchain
//! 2. Monitor detects payment → Parser extracts nullifier
//! 3. Storage saves payment to Redis
//! 4. Gateway queries API to verify payment
//! 5. Gateway marks nullifier as used after granting access

pub mod api;
pub mod config;
pub mod lightwalletd_client;
pub mod mock_node;
pub mod monitor;
pub mod note_decryption;
pub mod parser;
pub mod storage;

// Re-export commonly used types
pub use config::Config;
pub use monitor::Monitor;
pub use storage::{ReceivedPayment, Storage};
