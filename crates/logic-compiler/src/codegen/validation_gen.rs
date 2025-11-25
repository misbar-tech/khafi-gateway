//! Validation logic generation - converts DSL validation rules to Rust code

use crate::dsl::{BusinessRulesDSL, ValidationRule};
use anyhow::Result;
use proc_macro2::TokenStream;
use quote::quote;

/// Generate validation logic from DSL rules
pub fn generate_validations(dsl: &BusinessRulesDSL) -> Result<String> {
    let validation_checks: Vec<TokenStream> = dsl
        .validation_rules
        .iter()
        .enumerate()
        .map(|(idx, rule)| generate_validation_rule(rule, idx))
        .collect();

    let combined = quote! {
        /// Perform all validation checks
        fn validate_all(
            private_inputs: &PrivateInputs,
            public_params: &PublicParams,
        ) -> bool {
            #(#validation_checks)*
            true
        }
    };

    // Format the generated code
    format_rust_code(&combined.to_string())
}

/// Generate code for a single validation rule
fn generate_validation_rule(rule: &ValidationRule, _idx: usize) -> TokenStream {
    let check = match rule {
        ValidationRule::SignatureCheck {
            description,
            field,
            algorithm,
            public_key_param,
            message_fields,
        } => {
            let _desc = description;
            let field_ident = format_ident(&to_snake_case(field));
            let pubkey_ident = format_ident(&to_snake_case(public_key_param));
            let algo = algorithm;

            // Generate message concatenation
            let message_field_idents: Vec<_> = message_fields
                .iter()
                .map(|f| format_ident(&to_snake_case(f)))
                .collect();

            quote! {
                // Validation #idx: #desc
                {
                    // TODO: Implement #algo signature verification
                    // For now, this is a placeholder that assumes signature is valid
                    // In production, this would use ed25519_dalek, k256, or rsa crates

                    // Concatenate message fields
                    let mut message = Vec::new();
                    #(
                        message.extend_from_slice(
                            private_inputs.#message_field_idents.as_bytes()
                        );
                    )*

                    // Verify signature
                    let signature = &private_inputs.#field_ident;
                    let public_key = &public_params.#pubkey_ident;

                    // Placeholder - replace with actual verification
                    let signature_valid = verify_signature_placeholder(
                        &message,
                        signature,
                        public_key,
                        #algo
                    );

                    if !signature_valid {
                        return false;
                    }
                }
            }
        }

        ValidationRule::RangeCheck {
            description,
            field,
            min,
            max,
            min_param,
            max_param,
        } => {
            let _desc = description;
            let field_ident = format_ident(&to_snake_case(field));

            let min_check = if let Some(min_val) = min {
                quote! { let min_value = #min_val; }
            } else if let Some(min_p) = min_param {
                let min_param_ident = format_ident(&to_snake_case(min_p));
                quote! { let min_value = public_params.#min_param_ident; }
            } else {
                quote! { let min_value = 0; }
            };

            let max_check = if let Some(max_val) = max {
                quote! { let max_value = #max_val; }
            } else if let Some(max_p) = max_param {
                let max_param_ident = format_ident(&to_snake_case(max_p));
                quote! { let max_value = public_params.#max_param_ident; }
            } else {
                quote! { let max_value = u64::MAX; }
            };

            quote! {
                // Validation #idx: #desc
                {
                    #min_check
                    #max_check

                    let value = private_inputs.#field_ident;
                    if value < min_value || value > max_value {
                        return false;
                    }
                }
            }
        }

        ValidationRule::AgeVerification {
            description,
            dob_field,
            min_age,
            min_age_param,
        } => {
            let _desc = description;
            let dob_ident = format_ident(&to_snake_case(dob_field));

            let min_age_code = if let Some(age) = min_age {
                quote! { let min_age = #age; }
            } else if let Some(param) = min_age_param {
                let param_ident = format_ident(&to_snake_case(param));
                quote! { let min_age = public_params.#param_ident; }
            } else {
                quote! { let min_age = 18; }
            };

            quote! {
                // Validation #idx: #desc
                {
                    #min_age_code

                    let dob = &private_inputs.#dob_ident;
                    let age = calculate_age(dob);

                    if age < min_age {
                        return false;
                    }
                }
            }
        }

        ValidationRule::BlacklistCheck {
            description,
            field,
            blacklist_param,
        } => {
            let _desc = description;
            let field_ident = format_ident(&to_snake_case(field));
            let blacklist_ident = format_ident(&to_snake_case(blacklist_param));

            quote! {
                // Validation #idx: #desc
                {
                    let value = &private_inputs.#field_ident;
                    let blacklist = &public_params.#blacklist_ident;

                    if blacklist.contains(value) {
                        return false;
                    }
                }
            }
        }

        ValidationRule::ArrayIntersectionCheck {
            description,
            field,
            prohibited_param,
            must_be_empty,
        } => {
            let _desc = description;
            let field_ident = format_ident(&to_snake_case(field));
            let prohibited_ident = format_ident(&to_snake_case(prohibited_param));

            quote! {
                // Validation #idx: #desc
                {
                    let items = &private_inputs.#field_ident;
                    let prohibited = &public_params.#prohibited_ident;

                    let has_intersection = items.iter()
                        .any(|item| prohibited.contains(item));

                    if #must_be_empty && has_intersection {
                        return false;
                    }
                }
            }
        }

        ValidationRule::Custom { description, code } => {
            let _desc = description;
            // Parse the custom code as a TokenStream
            let custom_code: TokenStream = code.parse().unwrap_or_else(|_| {
                eprintln!("Warning: Failed to parse custom code, using placeholder");
                quote! { true }
            });

            quote! {
                // Validation #idx: #desc (custom)
                {
                    let result = #custom_code;
                    if !result {
                        return false;
                    }
                }
            }
        }
    };

    check
}

/// Generate helper functions needed for validation
pub fn generate_helper_functions() -> String {
    let code = quote! {
        /// Calculate age from date of birth (ISO 8601 format: YYYY-MM-DD)
        fn calculate_age(dob: &str) -> u32 {
            // Parse date of birth
            let parts: Vec<&str> = dob.split('-').collect();
            if parts.len() != 3 {
                return 0;
            }

            let birth_year: u32 = parts[0].parse().unwrap_or(0);
            let birth_month: u32 = parts[1].parse().unwrap_or(1);
            let birth_day: u32 = parts[2].parse().unwrap_or(1);

            // For simplicity, use a fixed "current" date
            // In production, this would be passed as a public parameter
            let current_year: u32 = 2024;
            let current_month: u32 = 1;
            let current_day: u32 = 1;

            let mut age = current_year - birth_year;

            // Adjust if birthday hasn't occurred this year
            if current_month < birth_month ||
               (current_month == birth_month && current_day < birth_day) {
                age -= 1;
            }

            age
        }

        /// Placeholder for signature verification
        /// TODO: Replace with actual cryptographic verification
        fn verify_signature_placeholder(
            _message: &[u8],
            _signature: &[u8],
            _public_key: &[u8],
            algorithm: &str,
        ) -> bool {
            // This is a placeholder - in production this would use:
            // - ed25519_dalek for Ed25519
            // - k256 for ECDSA
            // - rsa for RSA
            match algorithm {
                "ed25519" | "ecdsa" | "rsa" => {
                    // Always return true for now
                    true
                }
                _ => false,
            }
        }
    };

    format_rust_code(&code.to_string()).unwrap_or_else(|_| code.to_string())
}

/// Format Rust code using prettyplease
fn format_rust_code(code: &str) -> Result<String> {
    let parsed = syn::parse_file(code)?;
    Ok(prettyplease::unparse(&parsed))
}

/// Convert string to snake_case
fn to_snake_case(s: &str) -> String {
    s.to_lowercase().replace('-', "_").replace(' ', "_")
}

/// Helper to create ident from string
fn format_ident(s: &str) -> proc_macro2::Ident {
    syn::parse_str(s)
        .unwrap_or_else(|_| syn::parse_str(&format!("r#{}", s)).expect("Failed to create ident"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_range_check() {
        let rule = ValidationRule::RangeCheck {
            description: "Test range".to_string(),
            field: "quantity".to_string(),
            min: Some(1),
            max: Some(100),
            min_param: None,
            max_param: None,
        };

        let code = generate_validation_rule(&rule, 0);
        let code_str = code.to_string();

        assert!(code_str.contains("quantity"));
        assert!(code_str.contains("min_value"));
        assert!(code_str.contains("max_value"));
    }

    #[test]
    fn test_generate_age_verification() {
        let rule = ValidationRule::AgeVerification {
            description: "Check age".to_string(),
            dob_field: "date_of_birth".to_string(),
            min_age: Some(18),
            min_age_param: None,
        };

        let code = generate_validation_rule(&rule, 0);
        let code_str = code.to_string();

        assert!(code_str.contains("date_of_birth"));
        assert!(code_str.contains("calculate_age"));
        assert!(code_str.contains("18"));
    }

    #[test]
    fn test_generate_helper_functions() {
        let helpers = generate_helper_functions();
        assert!(helpers.contains("calculate_age"));
        assert!(helpers.contains("verify_signature_placeholder"));
    }
}
