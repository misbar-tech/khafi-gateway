//! Logic Compiler Service
//!
//! This service transforms JSON DSL business rules into custom SDKs.
//! It's the core differentiator that makes Khafi-Gateway a SaaS platform.

pub mod codegen;
pub mod dsl;
pub mod parser;

pub use codegen::CodeGenerator;
pub use dsl::*;
pub use parser::DslParser;
