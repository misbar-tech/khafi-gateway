// This file includes the generated methods from the build script
// The RISC Zero build system will generate:
// - GUEST_ELF: The compiled guest program binary
// - GUEST_ID: The Image ID (cryptographic hash of the ELF)
include!(concat!(env!("OUT_DIR"), "/methods.rs"));
