//! DSL (Domain-Specific Language) schema definitions
//!
//! This module defines the structure of the JSON DSL that customers use
//! to define their business logic validation rules.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Top-level DSL specification for a customer's business logic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessRulesDSL {
    /// Use case identifier (e.g., "prescription_validation", "manifest_compliance")
    pub use_case: String,

    /// Human-readable description of the use case
    #[serde(default)]
    pub description: String,

    /// Schema version for backward compatibility
    #[serde(default = "default_version")]
    pub version: String,

    /// Private input schema (data that remains hidden in the proof)
    pub private_inputs: InputSchema,

    /// Public parameter schema (validation parameters visible to verifier)
    pub public_params: ParamSchema,

    /// Validation rules to enforce
    pub validation_rules: Vec<ValidationRule>,

    /// Output schema (what the proof reveals)
    #[serde(default)]
    pub outputs: OutputSchema,
}

fn default_version() -> String {
    "1.0".to_string()
}

/// Schema for private inputs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InputSchema {
    /// Single object with fields
    Object(ObjectSchema),
    /// Map of named inputs
    Map(HashMap<String, ObjectSchema>),
}

/// Schema for a structured object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectSchema {
    /// Type of the object (currently always "object")
    #[serde(rename = "type")]
    pub type_name: String,

    /// Field definitions
    pub fields: HashMap<String, String>,
}

/// Schema for public parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ParamSchema {
    /// Map of parameter names to types
    Map(HashMap<String, String>),
    /// Object with explicit schema
    Object(ObjectSchema),
}

/// Schema for outputs
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OutputSchema {
    /// Compliance/validation result (always required)
    #[serde(default = "default_bool_type")]
    pub compliance_result: String,

    /// Additional output fields (optional)
    #[serde(flatten)]
    pub additional: HashMap<String, String>,
}

fn default_bool_type() -> String {
    "bool".to_string()
}

/// A validation rule in the DSL
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ValidationRule {
    /// Verify a digital signature
    SignatureCheck {
        /// Human-readable description
        #[serde(default)]
        description: String,

        /// Field containing the signature
        field: String,

        /// Signature algorithm (e.g., "ed25519", "ecdsa")
        algorithm: String,

        /// Parameter name containing public key
        public_key_param: String,

        /// Fields to include in signed message
        message_fields: Vec<String>,
    },

    /// Check if a numeric value is within a range
    RangeCheck {
        /// Human-readable description
        #[serde(default)]
        description: String,

        /// Field to check
        field: String,

        /// Minimum value (optional)
        #[serde(skip_serializing_if = "Option::is_none")]
        min: Option<u64>,

        /// Maximum value (optional, can reference param)
        #[serde(skip_serializing_if = "Option::is_none")]
        max: Option<u64>,

        /// Parameter name for max value
        #[serde(skip_serializing_if = "Option::is_none")]
        max_param: Option<String>,

        /// Parameter name for min value
        #[serde(skip_serializing_if = "Option::is_none")]
        min_param: Option<String>,
    },

    /// Verify age based on date of birth
    AgeVerification {
        /// Human-readable description
        #[serde(default)]
        description: String,

        /// Field containing date of birth (ISO 8601 format)
        dob_field: String,

        /// Minimum age requirement (optional)
        #[serde(skip_serializing_if = "Option::is_none")]
        min_age: Option<u32>,

        /// Parameter name for min age
        #[serde(skip_serializing_if = "Option::is_none")]
        min_age_param: Option<String>,
    },

    /// Check if a value is in a blacklist
    BlacklistCheck {
        /// Human-readable description
        #[serde(default)]
        description: String,

        /// Field to check
        field: String,

        /// Parameter name containing blacklist
        blacklist_param: String,
    },

    /// Check if arrays intersect (for prohibited items)
    ArrayIntersectionCheck {
        /// Human-readable description
        #[serde(default)]
        description: String,

        /// Field containing array to check
        field: String,

        /// Parameter name containing prohibited items
        prohibited_param: String,

        /// If true, intersection must be empty (no prohibited items)
        #[serde(default)]
        must_be_empty: bool,
    },

    /// Custom validation code (advanced)
    Custom {
        /// Human-readable description
        #[serde(default)]
        description: String,

        /// Custom Rust code snippet
        code: String,
    },
}

impl ValidationRule {
    /// Get the human-readable description of this rule
    pub fn description(&self) -> &str {
        match self {
            ValidationRule::SignatureCheck { description, .. } => description,
            ValidationRule::RangeCheck { description, .. } => description,
            ValidationRule::AgeVerification { description, .. } => description,
            ValidationRule::BlacklistCheck { description, .. } => description,
            ValidationRule::ArrayIntersectionCheck { description, .. } => description,
            ValidationRule::Custom { description, .. } => description,
        }
    }

    /// Get a short name for this rule type
    pub fn rule_type(&self) -> &str {
        match self {
            ValidationRule::SignatureCheck { .. } => "signature_check",
            ValidationRule::RangeCheck { .. } => "range_check",
            ValidationRule::AgeVerification { .. } => "age_verification",
            ValidationRule::BlacklistCheck { .. } => "blacklist_check",
            ValidationRule::ArrayIntersectionCheck { .. } => "array_intersection_check",
            ValidationRule::Custom { .. } => "custom",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_rule_description() {
        let rule = ValidationRule::AgeVerification {
            description: "Check age requirement".to_string(),
            dob_field: "dob".to_string(),
            min_age: Some(18),
            min_age_param: None,
        };
        assert_eq!(rule.description(), "Check age requirement");
        assert_eq!(rule.rule_type(), "age_verification");
    }

    #[test]
    fn test_default_version() {
        assert_eq!(default_version(), "1.0");
    }
}
