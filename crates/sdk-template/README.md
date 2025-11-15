# SDK Template

This is the base SDK template that customers integrate into their applications to generate zero-knowledge proofs and interact with the Khafi Gateway.

## Purpose

The SDK Template provides a high-level, type-safe API for:
1. **Proof Generation**: Create zk-STARK proofs combining Zcash payment + business logic
2. **Zcash Integration**: Fetch commitment tree roots, manage nullifiers
3. **Gateway Communication**: Submit proofs to the gateway and call protected APIs

## Architecture

```
┌────────────────────────────────────┐
│     Customer Application           │
│                                    │
│  uses                              │
│    ↓                               │
│  ┌──────────────────────────────┐ │
│  │   Generated Custom SDK       │ │  ← Logic Compiler output
│  │   (based on this template)   │ │
│  │                              │ │
│  │  • Type-safe input builders  │ │
│  │  • Proof generation          │ │
│  │  • API client                │ │
│  └──────────────────────────────┘ │
└────────────────────────────────────┘
           │
           │ HTTP/gRPC
           ↓
┌────────────────────────────────────┐
│   Khafi Infrastructure             │
│                                    │
│  • Zcash Backend (get tree root)   │
│  • Gateway (verify proof)          │
│  • Protected APIs                  │
└────────────────────────────────────┘
```

## Customization by Logic Compiler

The Logic Compiler generates a customer-specific version of this SDK:

### What Gets Customized:

1. **Builder Types** (`builders.rs`):
   - Generic `BusinessInputsBuilder` → Domain-specific builder
   - Example: `PrescriptionBuilder` with fields like `prescriber_id`, `quantity`, `patient_dob`

2. **Embedded Guest Program**:
   - The compiled guest binary (with custom business logic) gets embedded
   - Image ID is hardcoded for this specific use case

3. **Documentation**:
   - Auto-generated docs explaining the specific use case
   - Code examples relevant to the customer's domain

4. **Type Definitions**:
   - Domain-specific types (e.g., `Prescription`, `Manifest`, `KYCDocument`)
   - Serialization/deserialization helpers

### Example: Pharma SDK

**Generated from this template:**

```rust
// Pharma-specific types (generated)
pub struct Prescription {
    pub patient_id: String,
    pub drug_name: String,
    pub quantity: u32,
    pub prescriber_id: String,
    pub signature: Vec<u8>,
}

// Custom builder (generated)
pub struct PharmaInputsBuilder {
    prescription: Option<Prescription>,
    patient_dob: Option<Date>,
}

impl PharmaInputsBuilder {
    pub fn prescription(mut self, p: Prescription) -> Self {
        self.prescription = Some(p);
        self
    }

    pub fn patient_dob(mut self, dob: Date) -> Self {
        self.patient_dob = Some(dob);
        self
    }

    pub fn build(self) -> BusinessInputs {
        // Serialize prescription as private_data
        // Serialize validation params as public_params
        ...
    }
}

// Customer usage:
let sdk = KhafiSDK::new(IMAGE_ID, zcash_url, gateway_url);

let proof = sdk
    .generate_proof(
        zcash_inputs,
        PharmaInputsBuilder::new()
            .prescription(my_prescription)
            .patient_dob(patient_dob)
            .build(),
    )
    .await?;
```

## Modules

- **`lib.rs`**: Main SDK interface (`KhafiSDK` struct)
- **`prover.rs`**: RISC Zero proof generation
- **`zcash_client.rs`**: Communication with Zcash backend
- **`builders.rs`**: Type-safe input builders (customization point)

## Workflow

1. Customer receives generated SDK from Logic Compiler
2. Integrate SDK into their application
3. Use builders to construct inputs:
   ```rust
   let zcash_inputs = ZcashInputsBuilder::new()
       .spending_key(key)
       .note(note)
       .merkle_path(path)
       .merkle_root(root)
       .build()?;

   let business_inputs = CustomBuilder::new()
       .field1(value1)
       .field2(value2)
       .build()?;
   ```
4. Generate proof:
   ```rust
   let receipt = sdk.generate_proof(zcash_inputs, business_inputs).await?;
   ```
5. Submit to gateway:
   ```rust
   let response = sdk.call_api(&receipt, &nullifier, "api/protected").await?;
   ```

## Testing

Run tests:
```bash
cargo test -p sdk-template
```

## Related Crates

- `guest-template`: The zkVM guest program that this SDK invokes
- `logic-compiler`: Generates customized versions of this SDK
- `khafi-common`: Shared types used by both SDK and guest program
