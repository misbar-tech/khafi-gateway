use logic_compiler::CodeGenerator;
use logic_compiler::DslParser;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let example = args
        .get(1)
        .map(|s| s.as_str())
        .unwrap_or("age-verification-simple");

    let dsl_path = format!("docs/examples/{}.json", example);

    println!("===========================================");
    println!("  Generated Guest Program");
    println!("  DSL: {}", example);
    println!("===========================================\n");

    let dsl = DslParser::parse_file(&dsl_path).expect(&format!("Failed to parse {}", dsl_path));

    let generator = CodeGenerator::new(dsl);
    let code = generator.generate().expect("Failed to generate code");

    println!("{}", code);
}
