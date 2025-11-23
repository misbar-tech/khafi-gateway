use logic_compiler::codegen::CodeGenerator;
use logic_compiler::DslParser;

fn main() {
    let dsl = DslParser::parse_file("docs/examples/shipping-rules.json")
        .expect("Failed to parse shipping DSL");

    let generator = CodeGenerator::new(dsl);
    let code = generator.generate().expect("Failed to generate code");

    println!("{}", code);
}
