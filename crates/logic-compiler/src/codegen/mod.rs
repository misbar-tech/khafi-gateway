//! Code generation for RISC Zero guest programs from DSL
//!
//! This module transforms BusinessRulesDSL into Rust code that runs in the zkVM.

pub mod guest_template;
pub mod type_gen;
pub mod validation_gen;

use crate::dsl::BusinessRulesDSL;
use anyhow::{Context, Result};
use std::path::Path;

/// Main code generator that orchestrates guest program creation
pub struct CodeGenerator {
    dsl: BusinessRulesDSL,
}

impl CodeGenerator {
    /// Create a new code generator from a DSL specification
    pub fn new(dsl: BusinessRulesDSL) -> Self {
        Self { dsl }
    }

    /// Generate complete guest program source code
    ///
    /// Returns Rust source code as a String
    pub fn generate(&self) -> Result<String> {
        // Generate type definitions
        let types = type_gen::generate_types(&self.dsl)?;

        // Generate validation logic
        let validations = validation_gen::generate_validations(&self.dsl)?;

        // Combine into guest program
        let guest_code = guest_template::create_guest_program(&self.dsl, &types, &validations)?;

        Ok(guest_code)
    }

    /// Generate and write guest program to a file
    pub fn generate_to_file<P: AsRef<Path>>(&self, output_path: P) -> Result<()> {
        let code = self.generate()?;
        std::fs::write(output_path.as_ref(), code).with_context(|| {
            format!(
                "Failed to write guest program to {}",
                output_path.as_ref().display()
            )
        })?;
        Ok(())
    }

    /// Generate complete SDK package with guest program and build configuration
    pub fn generate_sdk_package<P: AsRef<Path>>(&self, output_dir: P) -> Result<()> {
        let output_dir = output_dir.as_ref();

        // Create directory structure
        std::fs::create_dir_all(output_dir.join("methods/guest/src"))?;

        // Generate guest program
        let guest_code = self.generate()?;
        std::fs::write(output_dir.join("methods/guest/src/main.rs"), guest_code)?;

        // Generate Cargo.toml for guest
        let guest_cargo = self.generate_guest_cargo_toml()?;
        std::fs::write(output_dir.join("methods/guest/Cargo.toml"), guest_cargo)?;

        // Generate build.rs for methods crate
        let build_script = self.generate_build_script()?;
        std::fs::write(output_dir.join("methods/build.rs"), build_script)?;

        // Generate methods/Cargo.toml
        let methods_cargo = self.generate_methods_cargo_toml()?;
        std::fs::write(output_dir.join("methods/Cargo.toml"), methods_cargo)?;

        Ok(())
    }

    fn generate_guest_cargo_toml(&self) -> Result<String> {
        Ok(format!(
            r#"[package]
name = "{}-guest"
version = "0.1.0"
edition = "2021"

[workspace]

[dependencies]
risc0-zkvm = {{ version = "1.0", default-features = false, features = ["std"] }}
serde = {{ version = "1.0", default-features = false, features = ["derive"] }}

[patch.crates-io]
# Optimization for zkVM
sha2 = {{ git = "https://github.com/risc0/RustCrypto-hashes", tag = "sha2-v0.10.6-risczero.0" }}
"#,
            self.dsl.use_case
        ))
    }

    fn generate_build_script(&self) -> Result<String> {
        Ok(r#"fn main() {
    risc0_build::embed_methods();
}
"#
        .to_string())
    }

    fn generate_methods_cargo_toml(&self) -> Result<String> {
        Ok(format!(
            r#"[package]
name = "{}-methods"
version = "0.1.0"
edition = "2021"

[build-dependencies]
risc0-build = "1.0"

[dependencies]
risc0-zkvm = "1.0"

[package.metadata.risc0]
methods = ["guest"]
"#,
            self.dsl.use_case
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DslParser;

    #[test]
    fn test_generate_age_verification() {
        let dsl = DslParser::parse_file("../../docs/examples/age-verification-simple.json")
            .expect("Failed to parse DSL");

        let generator = CodeGenerator::new(dsl);
        let code = generator.generate().expect("Failed to generate code");

        // Verify code contains expected elements
        assert!(code.contains("risc0_zkvm"));
        assert!(code.contains("env::read"));
        assert!(code.contains("env::commit"));
        assert!(code.contains("main()"));
    }
}
