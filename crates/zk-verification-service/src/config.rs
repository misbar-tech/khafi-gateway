//! Configuration management for ZK Verification Service

use methods::GUEST_ID;

/// Service configuration
#[derive(Clone)]
pub struct Config {
    /// Redis URL for nullifier storage
    pub redis_url: String,

    /// Expected Image ID for proof verification
    pub image_id: [u8; 32],
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        // Get Redis URL from environment or use default
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());

        // Convert GUEST_ID from [u32; 8] to [u8; 32]
        let image_id = image_id_to_bytes(&GUEST_ID);

        Self {
            redis_url,
            image_id,
        }
    }
}

/// Convert RISC Zero Image ID format ([u32; 8]) to bytes ([u8; 32])
fn image_id_to_bytes(id: &[u32; 8]) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    for (i, &word) in id.iter().enumerate() {
        let word_bytes = word.to_le_bytes();
        bytes[i * 4..(i + 1) * 4].copy_from_slice(&word_bytes);
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_env() {
        // Should not panic
        let config = Config::from_env();
        assert_eq!(config.image_id.len(), 32);
    }

    #[test]
    fn test_image_id_conversion() {
        let test_id: [u32; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
        let bytes = image_id_to_bytes(&test_id);
        assert_eq!(bytes.len(), 32);

        // Verify little-endian conversion
        assert_eq!(bytes[0], 1);
        assert_eq!(bytes[1], 0);
        assert_eq!(bytes[2], 0);
        assert_eq!(bytes[3], 0);
    }
}
