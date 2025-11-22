//! Configuration management for Zcash Backend
//!
//! Loads configuration from environment variables with sensible defaults.

use anyhow::{Context, Result};
use std::env;

/// Application configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Redis connection URL
    pub redis_url: String,

    /// API server host
    pub api_host: String,

    /// API server port
    pub api_port: u16,

    /// Blockchain polling interval in seconds
    pub polling_interval_secs: u64,

    /// Whether to use mock Zcash node (for development/testing)
    pub mock_mode: bool,

    /// Zcash node RPC URL (when not in mock mode)
    pub zcash_node_url: Option<String>,

    /// Zcash node RPC username
    pub zcash_node_user: Option<String>,

    /// Zcash node RPC password
    pub zcash_node_password: Option<String>,

    /// Khafi's Zcash payment address
    pub payment_address: String,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        // Load .env file if it exists (for local development)
        dotenv::dotenv().ok();

        let config = Config {
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),

            api_host: env::var("API_HOST")
                .unwrap_or_else(|_| "0.0.0.0".to_string()),

            api_port: env::var("API_PORT")
                .unwrap_or_else(|_| "8081".to_string())
                .parse()
                .context("Invalid API_PORT")?,

            polling_interval_secs: env::var("POLLING_INTERVAL_SECS")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .context("Invalid POLLING_INTERVAL_SECS")?,

            mock_mode: env::var("MOCK_MODE")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .context("Invalid MOCK_MODE (expected true/false)")?,

            zcash_node_url: env::var("ZCASH_NODE_URL").ok(),
            zcash_node_user: env::var("ZCASH_NODE_USER").ok(),
            zcash_node_password: env::var("ZCASH_NODE_PASSWORD").ok(),

            payment_address: env::var("PAYMENT_ADDRESS")
                .unwrap_or_else(|_| "u1test_mock_address".to_string()),
        };

        // Validate configuration
        config.validate()?;

        Ok(config)
    }

    /// Validate configuration
    fn validate(&self) -> Result<()> {
        if self.api_port == 0 {
            anyhow::bail!("API_PORT must be greater than 0");
        }

        if self.polling_interval_secs == 0 {
            anyhow::bail!("POLLING_INTERVAL_SECS must be greater than 0");
        }

        // If not in mock mode, require Zcash node configuration
        if !self.mock_mode {
            if self.zcash_node_url.is_none() {
                anyhow::bail!("ZCASH_NODE_URL is required when MOCK_MODE=false");
            }
        }

        Ok(())
    }

    /// Get the API server address
    pub fn api_address(&self) -> String {
        format!("{}:{}", self.api_host, self.api_port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        // Clear any existing environment variables
        env::remove_var("API_HOST");
        env::remove_var("API_PORT");
        env::remove_var("REDIS_URL");
        env::remove_var("MOCK_MODE");
        env::remove_var("POLLING_INTERVAL_SECS");

        // Set minimal environment for testing
        env::set_var("PAYMENT_ADDRESS", "test_address");

        let config = Config::from_env().expect("Failed to load config");

        assert_eq!(config.redis_url, "redis://localhost:6379");
        assert_eq!(config.api_host, "0.0.0.0");
        assert_eq!(config.api_port, 8081);
        assert_eq!(config.polling_interval_secs, 60);
        assert!(config.mock_mode);
    }

    #[test]
    fn test_api_address() {
        // Clear environment variables from previous tests
        env::remove_var("REDIS_URL");
        env::remove_var("MOCK_MODE");
        env::remove_var("POLLING_INTERVAL_SECS");

        // Set specific values for this test
        env::set_var("API_HOST", "127.0.0.1");
        env::set_var("API_PORT", "9000");
        env::set_var("PAYMENT_ADDRESS", "test_address");

        let config = Config::from_env().unwrap();
        assert_eq!(config.api_address(), "127.0.0.1:9000");
    }
}
