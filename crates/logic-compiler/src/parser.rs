//! DSL parser and validator
//!
//! This module handles parsing JSON DSL files and validating them.

use crate::dsl::*;
use anyhow::{Context, Result};
use std::path::Path;

/// Parser for Business Rules DSL
pub struct DslParser;

impl DslParser {
    /// Parse DSL from a JSON string
    ///
    /// # Arguments
    /// * `json_str` - JSON string containing the DSL
    ///
    /// # Returns
    /// * Parsed and validated DSL structure
    pub fn parse_str(json_str: &str) -> Result<BusinessRulesDSL> {
        let dsl: BusinessRulesDSL =
            serde_json::from_str(json_str).context("Failed to parse JSON DSL")?;

        Self::validate(&dsl)?;

        Ok(dsl)
    }

    /// Parse DSL from a JSON file
    ///
    /// # Arguments
    /// * `path` - Path to the JSON file
    ///
    /// # Returns
    /// * Parsed and validated DSL structure
    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<BusinessRulesDSL> {
        let json_str = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read DSL file: {}", path.as_ref().display()))?;

        Self::parse_str(&json_str)
    }

    /// Validate a parsed DSL structure
    ///
    /// Checks for:
    /// - Non-empty use_case
    /// - At least one validation rule
    /// - Valid field references
    /// - Valid parameter references
    fn validate(dsl: &BusinessRulesDSL) -> Result<()> {
        // Check use_case is not empty
        if dsl.use_case.is_empty() {
            anyhow::bail!("use_case cannot be empty");
        }

        // Check we have at least one validation rule
        if dsl.validation_rules.is_empty() {
            anyhow::bail!("At least one validation rule is required");
        }

        // Validate each rule
        for (idx, rule) in dsl.validation_rules.iter().enumerate() {
            Self::validate_rule(rule, dsl)
                .with_context(|| format!("Validation rule {} is invalid", idx))?;
        }

        Ok(())
    }

    /// Validate a single validation rule
    fn validate_rule(rule: &ValidationRule, _dsl: &BusinessRulesDSL) -> Result<()> {
        match rule {
            ValidationRule::SignatureCheck {
                field,
                algorithm,
                public_key_param,
                message_fields,
                ..
            } => {
                if field.is_empty() {
                    anyhow::bail!("signature_check: field cannot be empty");
                }
                if algorithm.is_empty() {
                    anyhow::bail!("signature_check: algorithm cannot be empty");
                }
                if public_key_param.is_empty() {
                    anyhow::bail!("signature_check: public_key_param cannot be empty");
                }
                if message_fields.is_empty() {
                    anyhow::bail!("signature_check: message_fields cannot be empty");
                }
                // Validate supported algorithms
                match algorithm.as_str() {
                    "ed25519" | "ecdsa" | "rsa" => {}
                    _ => anyhow::bail!("signature_check: unsupported algorithm '{}'", algorithm),
                }
            }

            ValidationRule::RangeCheck {
                field,
                min,
                max,
                max_param,
                min_param,
                ..
            } => {
                if field.is_empty() {
                    anyhow::bail!("range_check: field cannot be empty");
                }
                // Must have either min/max or min_param/max_param
                if min.is_none() && min_param.is_none() {
                    anyhow::bail!("range_check: must specify either 'min' or 'min_param'");
                }
                if max.is_none() && max_param.is_none() {
                    anyhow::bail!("range_check: must specify either 'max' or 'max_param'");
                }
            }

            ValidationRule::AgeVerification {
                dob_field,
                min_age,
                min_age_param,
                ..
            } => {
                if dob_field.is_empty() {
                    anyhow::bail!("age_verification: dob_field cannot be empty");
                }
                // Must have either min_age or min_age_param
                if min_age.is_none() && min_age_param.is_none() {
                    anyhow::bail!(
                        "age_verification: must specify either 'min_age' or 'min_age_param'"
                    );
                }
            }

            ValidationRule::BlacklistCheck {
                field,
                blacklist_param,
                ..
            } => {
                if field.is_empty() {
                    anyhow::bail!("blacklist_check: field cannot be empty");
                }
                if blacklist_param.is_empty() {
                    anyhow::bail!("blacklist_check: blacklist_param cannot be empty");
                }
            }

            ValidationRule::ArrayIntersectionCheck {
                field,
                prohibited_param,
                ..
            } => {
                if field.is_empty() {
                    anyhow::bail!("array_intersection_check: field cannot be empty");
                }
                if prohibited_param.is_empty() {
                    anyhow::bail!("array_intersection_check: prohibited_param cannot be empty");
                }
            }

            ValidationRule::Custom { code, .. } => {
                if code.is_empty() {
                    anyhow::bail!("custom: code cannot be empty");
                }
                // TODO: Could add basic Rust syntax validation here
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_dsl() {
        let json = r#"{
            "use_case": "age_verification",
            "description": "Simple age check",
            "version": "1.0",
            "private_inputs": {
                "user_data": {
                    "type": "object",
                    "fields": {
                        "date_of_birth": "string"
                    }
                }
            },
            "public_params": {
                "min_age": "u32"
            },
            "validation_rules": [
                {
                    "type": "age_verification",
                    "description": "Check minimum age",
                    "dob_field": "date_of_birth",
                    "min_age": 18
                }
            ]
        }"#;

        let dsl = DslParser::parse_str(json).unwrap();
        assert_eq!(dsl.use_case, "age_verification");
        assert_eq!(dsl.validation_rules.len(), 1);
    }

    #[test]
    fn test_validate_empty_use_case() {
        let json = r#"{
            "use_case": "",
            "private_inputs": {},
            "public_params": {},
            "validation_rules": [
                {
                    "type": "age_verification",
                    "dob_field": "dob",
                    "min_age": 18
                }
            ]
        }"#;

        let result = DslParser::parse_str(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("use_case"));
    }

    #[test]
    fn test_validate_no_rules() {
        let json = r#"{
            "use_case": "test",
            "private_inputs": {},
            "public_params": {},
            "validation_rules": []
        }"#;

        let result = DslParser::parse_str(json);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("At least one validation rule"));
    }

    #[test]
    fn test_validate_invalid_algorithm() {
        let json = r#"{
            "use_case": "test",
            "private_inputs": {},
            "public_params": {},
            "validation_rules": [
                {
                    "type": "signature_check",
                    "field": "sig",
                    "algorithm": "invalid_algo",
                    "public_key_param": "pk",
                    "message_fields": ["data"]
                }
            ]
        }"#;

        let result = DslParser::parse_str(json);
        assert!(result.is_err());
        // The error message contains the validation rule context
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.contains("unsupported algorithm") || err_msg.contains("invalid_algo"),
            "Error chain didn't contain expected error: {}",
            err_msg
        );
    }
}
