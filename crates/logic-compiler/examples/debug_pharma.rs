use logic_compiler::codegen::CodeGenerator;
use logic_compiler::DslParser;

fn main() {
    let dsl = DslParser::parse_file("docs/examples/pharma-rules.json")
        .expect("Failed to parse pharma DSL");

    let generator = CodeGenerator::new(dsl);
    let code = generator.generate().expect("Failed to generate code");

    // Extract validate_all function
    let validate_start = code
        .find("fn validate_all")
        .expect("validate_all not found");
    let end_pos = (validate_start + 3000).min(code.len());
    let validate_section = &code[validate_start..end_pos];

    println!("=== VALIDATE_ALL SECTION ===");
    println!("{}", validate_section);

    let sig_pos = validate_section
        .find("prescriber_signature")
        .unwrap_or(usize::MAX);
    let quantity_pos = validate_section.find("quantity").unwrap_or(usize::MAX);
    let dob_pos = validate_section.find("patient_dob").unwrap_or(usize::MAX);

    println!("\n=== POSITIONS ===");
    println!("sig_pos: {}", sig_pos);
    println!("quantity_pos: {}", quantity_pos);
    println!("dob_pos: {}", dob_pos);
}
