//! End-to-end tests for code generation

use logic_compiler::{CodeGenerator, DslParser};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_generate_age_verification_guest_program() {
    // Parse the age verification DSL
    let dsl = DslParser::parse_file("../../docs/examples/age-verification-simple.json")
        .expect("Failed to parse age verification DSL");

    // Create code generator
    let generator = CodeGenerator::new(dsl);

    // Generate guest program
    let code = generator.generate().expect("Failed to generate code");

    // Verify essential components are present
    assert!(code.contains("#![no_main]"), "Missing no_main attribute");
    assert!(
        code.contains("use risc0_zkvm::guest::env"),
        "Missing risc0 import"
    );
    assert!(code.contains("PrivateInputs"), "Missing PrivateInputs type");
    assert!(code.contains("PublicParams"), "Missing PublicParams type");
    assert!(code.contains("Outputs"), "Missing Outputs type");
    assert!(code.contains("fn main()"), "Missing main function");
    assert!(code.contains("env::read()"), "Missing env::read");
    assert!(code.contains("env::commit"), "Missing env::commit");
    assert!(
        code.contains("validate_all"),
        "Missing validate_all function"
    );
    assert!(
        code.contains("calculate_age"),
        "Missing age calculation helper"
    );
    assert!(
        code.contains("compliance_result"),
        "Missing compliance result"
    );

    // Verify age verification specific code
    assert!(code.contains("date_of_birth"), "Missing DOB field");
    assert!(code.contains("min_age"), "Missing min_age parameter");

    println!("Generated code:\n{}", code);
}

#[test]
fn test_generate_pharma_guest_program() {
    // Parse the pharma rules DSL
    let dsl = DslParser::parse_file("../../docs/examples/pharma-rules.json")
        .expect("Failed to parse pharma DSL");

    let generator = CodeGenerator::new(dsl);
    let code = generator.generate().expect("Failed to generate code");

    // Verify signature verification code
    assert!(
        code.contains("prescriber_signature"),
        "Missing signature field"
    );
    assert!(
        code.contains("verify_signature_placeholder"),
        "Missing signature verification"
    );
    assert!(code.contains("ed25519"), "Missing algorithm");

    // Verify range check code
    assert!(code.contains("quantity"), "Missing quantity field");
    assert!(code.contains("min_value"), "Missing min value check");
    assert!(code.contains("max_value"), "Missing max value check");

    // Verify age verification
    assert!(code.contains("patient_dob"), "Missing patient DOB");
    assert!(code.contains("calculate_age"), "Missing age calculation");

    println!("Generated pharma code length: {} bytes", code.len());
}

#[test]
fn test_generate_shipping_guest_program() {
    // Parse the shipping rules DSL
    let dsl = DslParser::parse_file("../../docs/examples/shipping-rules.json")
        .expect("Failed to parse shipping DSL");

    let generator = CodeGenerator::new(dsl);
    let code = generator.generate().expect("Failed to generate code");

    // Verify blacklist check
    assert!(
        code.contains("destination_country"),
        "Missing destination field"
    );
    assert!(
        code.contains("blacklist") && code.contains("contains"),
        "Missing blacklist check"
    );

    // Verify array intersection check
    assert!(
        code.contains("has_intersection"),
        "Missing intersection check"
    );
    assert!(code.contains("prohibited"), "Missing prohibited items");

    println!("Generated shipping code length: {} bytes", code.len());
}

#[test]
fn test_generate_to_file() {
    let dsl = DslParser::parse_file("../../docs/examples/age-verification-simple.json")
        .expect("Failed to parse DSL");

    let generator = CodeGenerator::new(dsl);

    // Create temp directory
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_path = temp_dir.path().join("guest_main.rs");

    // Generate to file
    generator
        .generate_to_file(&output_path)
        .expect("Failed to generate to file");

    // Verify file exists and has content
    assert!(output_path.exists(), "Output file not created");
    let content = fs::read_to_string(&output_path).expect("Failed to read output file");
    assert!(content.contains("#![no_main]"), "File content invalid");
    assert!(content.len() > 500, "Generated code too short");

    println!("Generated file size: {} bytes", content.len());
}

#[test]
fn test_generate_sdk_package() {
    let dsl = DslParser::parse_file("../../docs/examples/age-verification-simple.json")
        .expect("Failed to parse DSL");

    let generator = CodeGenerator::new(dsl);

    // Create temp directory
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Generate full SDK package
    generator
        .generate_sdk_package(temp_dir.path())
        .expect("Failed to generate SDK package");

    // Verify directory structure
    assert!(
        temp_dir.path().join("methods/guest/src/main.rs").exists(),
        "Guest main.rs not created"
    );
    assert!(
        temp_dir.path().join("methods/guest/Cargo.toml").exists(),
        "Guest Cargo.toml not created"
    );
    assert!(
        temp_dir.path().join("methods/build.rs").exists(),
        "build.rs not created"
    );
    assert!(
        temp_dir.path().join("methods/Cargo.toml").exists(),
        "Methods Cargo.toml not created"
    );

    // Verify guest Cargo.toml content
    let guest_cargo = fs::read_to_string(temp_dir.path().join("methods/guest/Cargo.toml"))
        .expect("Failed to read guest Cargo.toml");
    assert!(
        guest_cargo.contains("risc0-zkvm"),
        "Missing risc0-zkvm dependency"
    );
    assert!(
        guest_cargo.contains("age_verification"),
        "Missing use case name"
    );

    // Verify build.rs content
    let build_rs = fs::read_to_string(temp_dir.path().join("methods/build.rs"))
        .expect("Failed to read build.rs");
    assert!(
        build_rs.contains("risc0_build::embed_methods"),
        "Missing embed_methods call"
    );

    // Verify methods Cargo.toml content
    let methods_cargo = fs::read_to_string(temp_dir.path().join("methods/Cargo.toml"))
        .expect("Failed to read methods Cargo.toml");
    assert!(methods_cargo.contains("risc0-build"), "Missing risc0-build");
    assert!(
        methods_cargo.contains("[package.metadata.risc0]"),
        "Missing metadata"
    );

    println!(
        "SDK package generated successfully at {:?}",
        temp_dir.path()
    );
}

#[test]
fn test_type_generation_with_multiple_fields() {
    let dsl = DslParser::parse_file("../../docs/examples/pharma-rules.json")
        .expect("Failed to parse DSL");

    let generator = CodeGenerator::new(dsl);
    let code = generator.generate().expect("Failed to generate code");

    // Verify all prescription fields are present
    assert!(code.contains("drug_name"), "Missing drug_name field");
    assert!(code.contains("quantity"), "Missing quantity field");
    assert!(code.contains("patient_dob"), "Missing patient_dob field");
    assert!(
        code.contains("prescriber_id"),
        "Missing prescriber_id field"
    );
    assert!(
        code.contains("prescriber_signature"),
        "Missing prescriber_signature field"
    );

    // Verify public params
    assert!(code.contains("max_quantity"), "Missing max_quantity param");
    assert!(
        code.contains("prescriber_pubkey"),
        "Missing prescriber_pubkey param"
    );

    // Verify output field
    assert!(
        code.contains("prescription_hash"),
        "Missing prescription_hash output"
    );
}

#[test]
fn test_validation_order_preserved() {
    let dsl = DslParser::parse_file("../../docs/examples/pharma-rules.json")
        .expect("Failed to parse DSL");

    let generator = CodeGenerator::new(dsl);
    let code = generator.generate().expect("Failed to generate code");

    // Extract just the validate_all function to check order within it
    let validate_start = code
        .find("fn validate_all")
        .expect("validate_all not found");
    let validate_section = &code[validate_start..];

    // Find positions of validation checks within validate_all
    // Use more specific markers for each validation type
    let sig_pos = validate_section
        .find("verify_signature_placeholder")
        .unwrap_or(usize::MAX);
    let range_pos = validate_section.find("let min_value").unwrap_or(usize::MAX);
    let age_pos = validate_section.find("calculate_age").unwrap_or(usize::MAX);

    // Verify signature check comes before range check
    assert!(
        sig_pos < range_pos,
        "Validation order not preserved: sig@{} range@{}",
        sig_pos,
        range_pos
    );

    // Verify range check comes before age verification
    assert!(
        range_pos < age_pos,
        "Validation order not preserved: range@{} age@{}",
        range_pos,
        age_pos
    );

    println!(
        "Validation order (within validate_all): sig@{}, range@{}, age@{}",
        sig_pos, range_pos, age_pos
    );
}

#[test]
fn test_code_compiles_syntax() {
    // This test verifies that generated code has valid Rust syntax
    // We can't actually compile it without risc0 target, but we can parse it

    let dsl = DslParser::parse_file("../../docs/examples/age-verification-simple.json")
        .expect("Failed to parse DSL");

    let generator = CodeGenerator::new(dsl);
    let code = generator.generate().expect("Failed to generate code");

    // Try to parse the code as valid Rust syntax
    // This is a basic sanity check
    let parsed = syn::parse_file(&code);
    assert!(
        parsed.is_ok(),
        "Generated code has invalid syntax: {:?}",
        parsed.err()
    );

    println!("Generated code passes syntax validation");
}
