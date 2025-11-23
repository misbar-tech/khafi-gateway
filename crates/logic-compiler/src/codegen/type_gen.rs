//! Type generation - converts DSL schemas to Rust struct definitions

use crate::dsl::{BusinessRulesDSL, InputSchema, ParamSchema};
use anyhow::Result;
use proc_macro2::TokenStream;
use quote::quote;

/// Generate Rust type definitions from DSL schemas
pub fn generate_types(dsl: &BusinessRulesDSL) -> Result<String> {
    let private_inputs = generate_private_inputs(&dsl.private_inputs)?;
    let public_params = generate_public_params(&dsl.public_params)?;
    let outputs = generate_outputs(dsl)?;

    let combined = quote! {
        use serde::{Deserialize, Serialize};

        #private_inputs

        #public_params

        #outputs
    };

    Ok(format_code(&combined))
}

/// Generate private inputs struct
fn generate_private_inputs(schema: &InputSchema) -> Result<TokenStream> {
    match schema {
        InputSchema::Object(obj) => {
            let fields = generate_fields(&obj.fields);
            Ok(quote! {
                /// Private inputs (hidden in the proof)
                #[derive(Debug, Clone, Serialize, Deserialize)]
                pub struct PrivateInputs {
                    #(#fields),*
                }
            })
        }
        InputSchema::Map(map) => {
            // Generate a struct for each named input
            let structs: Vec<TokenStream> = map
                .iter()
                .map(|(name, obj)| {
                    let struct_name = format_ident(&to_pascal_case(name));
                    let fields = generate_fields(&obj.fields);
                    quote! {
                        #[derive(Debug, Clone, Serialize, Deserialize)]
                        pub struct #struct_name {
                            #(#fields),*
                        }
                    }
                })
                .collect();

            // Create a wrapper struct
            let field_defs: Vec<TokenStream> = map
                .keys()
                .map(|name| {
                    let field_name = format_ident(&to_snake_case(name));
                    let field_type = format_ident(&to_pascal_case(name));
                    quote! { pub #field_name: #field_type }
                })
                .collect();

            Ok(quote! {
                #(#structs)*

                /// Private inputs (hidden in the proof)
                #[derive(Debug, Clone, Serialize, Deserialize)]
                pub struct PrivateInputs {
                    #(#field_defs),*
                }
            })
        }
    }
}

/// Generate public params struct
fn generate_public_params(schema: &ParamSchema) -> Result<TokenStream> {
    match schema {
        ParamSchema::Map(map) => {
            let fields: Vec<TokenStream> = map
                .iter()
                .map(|(name, type_str)| {
                    let field_name = format_ident(&to_snake_case(name));
                    let field_type = map_type_string(type_str);
                    quote! { pub #field_name: #field_type }
                })
                .collect();

            Ok(quote! {
                /// Public parameters (visible to verifier)
                #[derive(Debug, Clone, Serialize, Deserialize)]
                pub struct PublicParams {
                    #(#fields),*
                }
            })
        }
        ParamSchema::Object(obj) => {
            let fields = generate_fields(&obj.fields);
            Ok(quote! {
                /// Public parameters (visible to verifier)
                #[derive(Debug, Clone, Serialize, Deserialize)]
                pub struct PublicParams {
                    #(#fields),*
                }
            })
        }
    }
}

/// Generate outputs struct
fn generate_outputs(dsl: &BusinessRulesDSL) -> Result<TokenStream> {
    let additional_fields: Vec<TokenStream> = dsl
        .outputs
        .additional
        .iter()
        .map(|(name, type_str)| {
            let field_name = format_ident(&to_snake_case(name));
            let field_type = map_type_string(type_str);
            quote! { pub #field_name: #field_type }
        })
        .collect();

    Ok(quote! {
        /// Outputs from the verification (public)
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct Outputs {
            /// Whether validation passed
            pub compliance_result: bool,
            #(#additional_fields),*
        }
    })
}

/// Generate field definitions from a HashMap
fn generate_fields(fields: &std::collections::HashMap<String, String>) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|(name, type_str)| {
            let field_name = format_ident(&to_snake_case(name));
            let field_type = map_type_string(type_str);
            quote! { pub #field_name: #field_type }
        })
        .collect()
}

/// Map DSL type strings to Rust types
fn map_type_string(type_str: &str) -> TokenStream {
    match type_str {
        "string" => quote! { String },
        "u32" => quote! { u32 },
        "u64" => quote! { u64 },
        "i32" => quote! { i32 },
        "i64" => quote! { i64 },
        "bool" => quote! { bool },
        "bytes" => quote! { Vec<u8> },
        "array<string>" | "array[string]" => quote! { Vec<String> },
        "array<u32>" | "array[u32]" => quote! { Vec<u32> },
        "array<u64>" | "array[u64]" => quote! { Vec<u64> },
        _ => {
            // Default to String for unknown types
            eprintln!("Warning: Unknown type '{}', defaulting to String", type_str);
            quote! { String }
        }
    }
}

/// Convert string to PascalCase
fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
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

/// Format TokenStream as formatted Rust code
fn format_code(tokens: &TokenStream) -> String {
    let code = tokens.to_string();
    // Basic formatting - in production you'd use rustfmt
    code.replace(" ; ", ";\n")
        .replace(" { ", " {\n    ")
        .replace(" } ", "\n}\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::ObjectSchema;
    use std::collections::HashMap;

    #[test]
    fn test_map_type_string() {
        let tokens = map_type_string("u32");
        assert_eq!(tokens.to_string(), "u32");

        let tokens = map_type_string("string");
        assert_eq!(tokens.to_string(), "String");

        let tokens = map_type_string("bytes");
        assert_eq!(tokens.to_string(), "Vec < u8 >");
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("user_data"), "UserData");
        assert_eq!(to_pascal_case("foo"), "Foo");
        assert_eq!(to_pascal_case("some_long_name"), "SomeLongName");
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("UserData"), "userdata");
        assert_eq!(to_snake_case("some-name"), "some_name");
        assert_eq!(to_snake_case("min_age"), "min_age");
    }

    #[test]
    fn test_generate_simple_types() {
        let mut fields = HashMap::new();
        fields.insert("date_of_birth".to_string(), "string".to_string());
        fields.insert("user_id".to_string(), "string".to_string());

        let obj = ObjectSchema {
            type_name: "object".to_string(),
            fields,
        };

        let schema = InputSchema::Object(obj);
        let result = generate_private_inputs(&schema).unwrap();
        let code = result.to_string();

        assert!(code.contains("PrivateInputs"));
        assert!(code.contains("date_of_birth"));
        assert!(code.contains("user_id"));
    }
}
