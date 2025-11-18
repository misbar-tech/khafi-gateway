//! Integration tests for logic compiler

use logic_compiler::DslParser;

#[test]
fn test_parse_pharma_example() {
    let dsl = DslParser::parse_file("../../docs/examples/pharma-rules.json")
        .expect("Failed to parse pharma-rules.json");

    assert_eq!(dsl.use_case, "prescription_validation");
    assert_eq!(dsl.validation_rules.len(), 4);
}

#[test]
fn test_parse_shipping_example() {
    let dsl = DslParser::parse_file("../../docs/examples/shipping-rules.json")
        .expect("Failed to parse shipping-rules.json");

    assert_eq!(dsl.use_case, "manifest_compliance");
    assert_eq!(dsl.validation_rules.len(), 5);
}

#[test]
fn test_parse_age_verification_example() {
    let dsl = DslParser::parse_file("../../docs/examples/age-verification-simple.json")
        .expect("Failed to parse age-verification-simple.json");

    assert_eq!(dsl.use_case, "age_verification");
    assert_eq!(dsl.validation_rules.len(), 1);
}
