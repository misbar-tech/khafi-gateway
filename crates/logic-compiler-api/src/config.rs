//! Configuration management for Logic Compiler API
//!
//! Loads configuration from environment variables with sensible defaults.

use anyhow::{Context, Result};
use std::env;
use std::path::PathBuf;

/// Application configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// API server host
    pub api_host: String,

    /// API server port
    pub api_port: u16,

    /// Directory where SDK packages are generated
    pub sdk_output_dir: PathBuf,

    /// Directory containing template files
    pub templates_dir: PathBuf,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        // Load .env file if it exists (for local development)
        dotenv::dotenv().ok();

        let config = Config {
            api_host: env::var("API_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),

            api_port: env::var("API_PORT")
                .unwrap_or_else(|_| "8082".to_string())
                .parse()
                .context("Invalid API_PORT")?,

            sdk_output_dir: env::var("SDK_OUTPUT_DIR")
                .unwrap_or_else(|_| "./output/sdks".to_string())
                .into(),

            templates_dir: env::var("TEMPLATES_DIR")
                .unwrap_or_else(|_| "./docs/examples".to_string())
                .into(),
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

        Ok(())
    }

    /// Get the API server address
    pub fn api_address(&self) -> String {
        format!("{}:{}", self.api_host, self.api_port)
    }

    /// Ensure output directories exist
    pub fn ensure_directories(&self) -> Result<()> {
        std::fs::create_dir_all(&self.sdk_output_dir).with_context(|| {
            format!(
                "Failed to create SDK output directory: {}",
                self.sdk_output_dir.display()
            )
        })?;

        // Templates directory might not exist in development, that's ok
        if !self.templates_dir.exists() {
            tracing::warn!(
                "Templates directory does not exist: {}",
                self.templates_dir.display()
            );
        }

        Ok(())
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
        env::remove_var("SDK_OUTPUT_DIR");
        env::remove_var("TEMPLATES_DIR");

        let config = Config::from_env().expect("Failed to load config");

        assert_eq!(config.api_host, "0.0.0.0");
        assert_eq!(config.api_port, 8082);
        assert_eq!(config.sdk_output_dir, PathBuf::from("./output/sdks"));
        assert_eq!(config.templates_dir, PathBuf::from("./docs/examples"));
    }

    #[test]
    fn test_api_address() {
        // Test api_address() method directly without env vars
        let config = Config {
            api_host: "127.0.0.1".to_string(),
            api_port: 9000,
            sdk_output_dir: PathBuf::from("./output"),
            templates_dir: PathBuf::from("./templates"),
        };

        assert_eq!(config.api_address(), "127.0.0.1:9000");
    }

    #[test]
    fn test_validate_invalid_port() {
        let config = Config {
            api_host: "0.0.0.0".to_string(),
            api_port: 0,
            sdk_output_dir: PathBuf::from("./output"),
            templates_dir: PathBuf::from("./templates"),
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("API_PORT must be greater than 0"));
    }
}
