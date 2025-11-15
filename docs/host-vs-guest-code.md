# RISC Zero: Host vs Guest Code in Khafi Gateway

## Quick Answer

In our **khafi-gateway** project:

- **HOST CODE** = `crates/sdk-template/src/prover.rs` ← **This is the equivalent of zgw-rz/host/**
- **GUEST CODE** = `crates/methods/guest/src/main.rs`

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                      HOST CODE                              │
│  (Runs on normal CPU - does the proving)                    │
│                                                              │
│  Location: crates/sdk-template/src/prover.rs                │
│                                                              │
│  What it does:                                              │
│  1. Prepare inputs                                          │
│  2. Create ExecutorEnv with inputs                          │
│  3. Call prover.prove(env, GUEST_ELF)                       │
│  4. Get receipt back                                        │
│  5. Extract journal (outputs)                               │
│  6. Verify proofs                                           │
└─────────────────────────────────────────────────────────────┘
                            │
                            │ Runs guest program in zkVM
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                     GUEST CODE                              │
│  (Runs inside zkVM - gets proven)                          │
│                                                              │
│  Location: crates/methods/guest/src/main.rs                 │
│                                                              │
│  What it does:                                              │
│  1. env::read() to get inputs from host                    │
│  2. Verify Zcash payment                                    │
│  3. Execute business logic                                  │
│  4. env::commit(&outputs) to write results                 │
│                                                              │
│  Compiled to: target/.../guest.bin (352 KB ELF)            │
└─────────────────────────────────────────────────────────────┘
```

## Side-by-Side Code Comparison

### Reference (zgw-rz) - Simple Example

**HOST CODE:** `zgw-rz/host/src/main.rs`
```rust
use methods::{METHOD_ELF, METHOD_ID};
use risc0_zkvm::{default_prover, ExecutorEnv};

fn main() {
    // 1. Prepare input
    let input: u32 = 15 * u32::pow(2, 27) + 1;

    // 2. Create environment with input
    let env = ExecutorEnv::builder()
        .write(&input)
        .unwrap()
        .build()
        .unwrap();

    // 3. Get prover
    let prover = default_prover();

    // 4. Generate proof
    let prove_info = prover
        .prove(env, METHOD_ELF)
        .unwrap();

    // 5. Extract receipt
    let receipt = prove_info.receipt;

    // 6. Extract output from journal
    let output: u32 = receipt.journal.decode().unwrap();

    // 7. Verify proof
    receipt.verify(METHOD_ID).unwrap();
}
```

**GUEST CODE:** `zgw-rz/methods/guest/src/main.rs`
```rust
use risc0_zkvm::guest::env;

fn main() {
    // Read input from host
    let input: u32 = env::read();

    // Do computation
    let result = input * 2;

    // Write output to journal
    env::commit(&result);
}
```

---

### Our Project (khafi-gateway) - Production Example

**HOST CODE:** `crates/sdk-template/src/prover.rs`
```rust
use khafi_common::{GuestInputs, GuestOutputs, Receipt, Result};
use risc0_zkvm::{default_prover, ExecutorEnv};

/// Generate a RISC Zero proof
pub fn generate_proof(
    inputs: GuestInputs,
    guest_binary: &[u8],
    image_id: [u8; 32],
) -> Result<Receipt> {
    // 1. Create environment with inputs
    let env = ExecutorEnv::builder()
        .write(&inputs)?                    // Write GuestInputs
        .build()?;

    // 2. Get prover
    let prover = default_prover();

    // 3. Generate proof
    let prove_info = prover
        .prove(env, guest_binary)?;         // Use GUEST_ELF

    // 4. Extract receipt
    let risc0_receipt = prove_info.receipt;

    // 5. Serialize and wrap receipt
    let receipt_bytes = bincode::serde::encode_to_vec(&risc0_receipt, ...)?;
    Ok(Receipt::new(receipt_bytes, image_id))
}

/// Extract outputs from journal
pub fn extract_outputs(receipt: &Receipt) -> Result<GuestOutputs> {
    // 1. Deserialize RISC Zero receipt
    let (risc0_receipt, _) = bincode::serde::decode_from_slice(&receipt.inner, ...)?;

    // 2. Get journal bytes
    let journal_bytes = risc0_receipt.journal.bytes;

    // 3. Deserialize GuestOutputs
    let (outputs, _) = bincode::serde::decode_from_slice(&journal_bytes, ...)?;

    Ok(outputs)
}
```

**GUEST CODE:** `crates/methods/guest/src/main.rs`
```rust
use risc0_zkvm::guest::env;
use khafi_common::{GuestInputs, GuestOutputs, Nullifier};

risc0_zkvm::guest::entry!(main);

fn main() {
    // 1. Read inputs from host
    let inputs: GuestInputs = env::read();

    // 2. Verify Zcash payment
    let nullifier = verify_zcash_payment(&inputs.zcash);

    // 3. Execute business logic
    let compliance_result = execute_business_logic(&inputs.business);

    // 4. Create outputs
    let outputs = GuestOutputs {
        nullifier,
        compliance_result,
        metadata: vec![],
    };

    // 5. Write to journal (public output)
    env::commit(&outputs);
}
```

## Key Responsibilities

### HOST CODE (sdk-template/prover.rs)

| Responsibility | Code Location |
|----------------|---------------|
| **Prepare inputs** | `let inputs = GuestInputs { ... }` |
| **Create executor env** | `ExecutorEnv::builder().write(&inputs)` |
| **Load guest ELF** | Uses `GUEST_ELF` from methods crate |
| **Run prover** | `default_prover().prove(env, GUEST_ELF)` |
| **Extract receipt** | `prove_info.receipt` |
| **Decode journal** | `bincode::decode_from_slice(journal_bytes)` |
| **Verify proofs** | `receipt.verify(image_id)` |

### GUEST CODE (methods/guest/src/main.rs)

| Responsibility | Code Location |
|----------------|---------------|
| **Read inputs** | `env::read()` |
| **Verify Zcash** | `verify_zcash_payment(&inputs.zcash)` |
| **Business logic** | `execute_business_logic(&inputs.business)` |
| **Create outputs** | `GuestOutputs { nullifier, compliance_result, ... }` |
| **Write to journal** | `env::commit(&outputs)` |

## Data Flow

```
USER/APPLICATION
      │
      │ Calls SDK
      ▼
┌─────────────────────────────────────────┐
│  KhafiSDK::generate_proof()             │  ← User-facing API
│  (sdk-template/src/lib.rs)              │
└─────────────────────────────────────────┘
      │
      │ Calls
      ▼
┌─────────────────────────────────────────┐
│  prover::generate_proof()               │  ← HOST CODE (this is the equivalent of zgw-rz/host/)
│  (sdk-template/src/prover.rs)           │
│                                          │
│  1. ExecutorEnv::builder()              │
│     .write(&GuestInputs)                │
│  2. default_prover()                    │
│  3. .prove(env, GUEST_ELF)  ────────────┼──┐
└─────────────────────────────────────────┘  │
                                              │ Runs in zkVM
                                              ▼
                                    ┌─────────────────────────┐
                                    │  guest::main()          │  ← GUEST CODE
                                    │  (methods/guest/)       │
                                    │                         │
                                    │  1. env::read()         │
                                    │  2. verify_zcash()      │
                                    │  3. business_logic()    │
                                    │  4. env::commit()       │
                                    └─────────────────────────┘
                                              │
                                              │ Returns receipt with journal
                                              ▼
┌─────────────────────────────────────────┐
│  Receipt { inner, image_id }            │
│                                          │
│  receipt.journal contains GuestOutputs  │
└─────────────────────────────────────────┘
```

## Where to Find Each Part

### In Our Project

```
crates/
├── sdk-template/              ← HOST CODE
│   ├── src/lib.rs             • High-level SDK API
│   └── src/prover.rs          • ⭐ CORE HOST CODE (equiv of zgw-rz/host/)
│                                - ExecutorEnv::builder()
│                                - default_prover().prove()
│                                - Journal extraction
│
└── methods/                   ← Build system + GUEST CODE
    ├── build.rs               • Builds guest program
    ├── src/lib.rs             • Exports GUEST_ELF & GUEST_ID
    └── guest/                 ← GUEST CODE
        └── src/main.rs        • ⭐ CORE GUEST CODE
                                 - env::read()
                                 - Verification logic
                                 - env::commit()
```

### In Reference Project

```
zgw-rz/
├── host/
│   └── src/main.rs            ← HOST CODE (simple example)
│
└── methods/
    ├── build.rs
    ├── src/lib.rs
    └── guest/
        └── src/main.rs        ← GUEST CODE (simple example)
```

## Common Confusion Points

### ❓ "Why is the host code in sdk-template and not in a 'host' directory?"

**Answer:** Because our project is more complex:
- **zgw-rz** = Simple standalone example (one host main.rs)
- **khafi-gateway** = Full SDK/library (host code is distributed across the SDK crate)

The host code is **the SDK itself** - it's what users import and call.

### ❓ "Can I move host code to a separate 'host' crate?"

**Answer:** You could, but it's not necessary. The current structure is better for a library/SDK:
```rust
// Users import the SDK (which contains the host code)
use sdk_template::KhafiSDK;

let sdk = KhafiSDK::new(...);
let receipt = sdk.generate_proof(...).await?;  // This calls the host code
```

### ❓ "Where is methods::GUEST_ELF used?"

**Answer:** In the host code (`prover.rs`):
```rust
use methods::GUEST_ELF;

// Host code passes GUEST_ELF to the prover
prover.prove(env, GUEST_ELF)
```

## Summary

| Aspect | Reference (zgw-rz) | Our Project (khafi-gateway) |
|--------|-------------------|----------------------------|
| **Host Code** | `host/src/main.rs` | `sdk-template/src/prover.rs` |
| **Guest Code** | `methods/guest/src/main.rs` | `methods/guest/src/main.rs` |
| **Purpose** | Standalone example | Production SDK/library |
| **Complexity** | Simple (u32 input/output) | Complex (GuestInputs/Outputs) |
| **User-facing** | CLI tool | Library API |

**Bottom Line:**
- `crates/sdk-template/src/prover.rs` = HOST CODE ✅
- `crates/methods/guest/src/main.rs` = GUEST CODE ✅

They're just organized differently because we're building a library/SDK, not a simple standalone tool.
