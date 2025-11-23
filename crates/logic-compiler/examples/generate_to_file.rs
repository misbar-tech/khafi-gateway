use logic_compiler::CodeGenerator;
use logic_compiler::DslParser;
use std::fs;

fn main() {
    let examples = vec!["age-verification-simple", "pharma-rules", "shipping-rules"];

    // Create output directory
    fs::create_dir_all("generated_guests").expect("Failed to create output dir");

    for example in examples {
        let dsl_path = format!("docs/examples/{}.json", example);
        let output_path = format!("generated_guests/{}_guest.rs", example);

        println!("Generating {} -> {}", dsl_path, output_path);

        let dsl = DslParser::parse_file(&dsl_path).expect(&format!("Failed to parse {}", dsl_path));

        let generator = CodeGenerator::new(dsl);
        generator
            .generate_to_file(&output_path)
            .expect("Failed to generate to file");

        println!(
            "  ✓ Generated {} bytes\n",
            fs::metadata(&output_path).unwrap().len()
        );
    }

    println!("All guest programs generated in ./generated_guests/");
    println!("\nView them with:");
    println!("  cat generated_guests/age-verification-simple_guest.rs");
    println!("  cat generated_guests/pharma-rules_guest.rs");
    println!("  cat generated_guests/shipping-rules_guest.rs");
}
