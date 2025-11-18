# Two-Proof Architecture Design

**Status:** Proposed
**Date:** 2025-11-18
**Author:** Architecture Team

## Table of Contents
1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Component Breakdown](#component-breakdown)
4. [API Specifications](#api-specifications)
5. [Trust Model & Security](#trust-model--security)
6. [Implementation Plan](#implementation-plan)
7. [Comparison: Single vs Two-Proof](#comparison-single-vs-two-proof)
8. [Client SDK Examples](#client-sdk-examples)
9. [Deployment Guide](#deployment-guide)

---

## Overview

### The Problem

The current single-proof architecture combines both Zcash payment verification and business logic verification in one zkVM proof. This creates several challenges:

1. **Platform Limitations:** RISC0's prover requires native code (can't run in browsers/mobile)
2. **Privacy Model Confusion:** Unclear who should hold spending keys vs business data
3. **Flexibility:** Can't verify payment and business logic independently

### The Solution

**Split verification into TWO separate zero-knowledge proofs:**

1. **Proof 1: Zcash Payment Proof**
   - Verifies user has valid Zcash payment
   - Derives nullifier from spending key
   - Generated client-side (non-custodial)

2. **Proof 2: Business Logic Proof**
   - Verifies compliance with business rules (prescription valid, manifest legal, etc.)
   - Uses private customer data
   - Generated client-side (user controls private data)

Both proofs are verified independently by khafi-gateway before allowing API access.

---

## Architecture

### High-Level Flow

```
┌────────────────────────────────────────────────────────────────────┐
│ User's Device (Browser/Mobile/Backend)                             │
│                                                                     │
│ User Controls:                                                      │
│  ✓ Zcash spending key (controls money)                            │
│  ✓ Business private data (prescription, shipping manifest, etc.)   │
└─────────────────┬───────────────────────────────────────────────────┘
                  │
                  │ Step 1: Generate both proofs
                  │ POST /v1/generate-proofs
                  │ { zcash_inputs, business_inputs }
                  ↓
┌─────────────────────────────────────────────────────────────────────┐
│ Customer's Infrastructure                                           │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────┐      │
│  │ Proving Service (NEW)                                    │      │
│  │  - Stateless proof generation                            │      │
│  │  - Does NOT store user secrets                           │      │
│  │                                                           │      │
│  │  ┌──────────────────┐    ┌──────────────────┐           │      │
│  │  │ Zcash Prover     │    │ Business Prover  │           │      │
│  │  │ (guest-zcash)    │    │ (guest-business) │           │      │
│  │  │                  │    │                  │           │      │
│  │  │ Image ID: 0xAAA  │    │ Image ID: 0xBBB  │           │      │
│  │  └──────────────────┘    └──────────────────┘           │      │
│  │                                                           │      │
│  │  Returns:                                                 │      │
│  │  {                                                        │      │
│  │    "zcash_proof": "0x...",                               │      │
│  │    "business_proof": "0x...",                            │      │
│  │    "nullifier": "0x...",                                 │      │
│  │    "image_ids": { "zcash": "0xAAA", "business": "0xBBB" }│      │
│  │  }                                                        │      │
│  └──────────────────────────────────────────────────────────┘      │
│                  │                                                  │
│                  │ Step 2: Client receives both proofs              │
│                  │ (Proofs never logged, user keeps them)           │
│                  │                                                  │
│                  ↓                                                  │
│  ┌───────────────────────────────────────────────┐                 │
│  │ Client calls protected API:                   │                 │
│  │   GET /api/prescription                       │                 │
│  │   Headers:                                    │                 │
│  │     x-zk-receipt-payment: <zcash_proof>       │                 │
│  │     x-zk-receipt-business: <business_proof>   │                 │
│  │     x-zk-nullifier: <nullifier>               │                 │
│  └───────────────┬───────────────────────────────┘                 │
│                  │                                                  │
│                  │ Step 3: Gateway verifies BOTH proofs             │
│                  ↓                                                  │
│  ┌──────────────────────────────────────────────────────┐          │
│  │ khafi-gateway (Envoy + zk-verification-service)      │          │
│  │                                                       │          │
│  │  Verification Steps:                                 │          │
│  │  1. ✓ Zcash payment proof valid?                    │          │
│  │     - Verify cryptographic proof against 0xAAA       │          │
│  │     - Extract nullifier from journal                 │          │
│  │  2. ✓ Business logic proof valid?                   │          │
│  │     - Verify cryptographic proof against 0xBBB       │          │
│  │     - Extract compliance_result from journal         │          │
│  │  3. ✓ Nullifier not replayed? (Redis check)         │          │
│  │  4. ✓ Compliance result == true?                    │          │
│  │                                                       │          │
│  │  If ALL pass → route to protected API               │          │
│  │  If ANY fail → return 403 Forbidden                 │          │
│  └───────────────────┬───────────────────────────────────┘          │
│                      │                                              │
│                      │ Step 4: Access granted                       │
│                      ↓                                              │
│  ┌──────────────────────────────────────┐                          │
│  │ Protected API                        │                          │
│  │ (pharmacy inventory, shipping, etc.) │                          │
│  └──────────────────────────────────────┘                          │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Component Breakdown

### 1. Guest Programs (zkVM Code)

#### Guest Program 1: Zcash Payment Verifier

**Location:** `crates/methods/guest-zcash/src/main.rs`

```rust
#![no_main]

use risc0_zkvm::guest::env;
use khafi_common::{ZcashInputs, ZcashOutputs, Nullifier};

risc0_zkvm::guest::entry!(main);

fn main() {
    // Read Zcash payment inputs
    let inputs: ZcashInputs = env::read();

    // Verify payment:
    // 1. Check merkle proof (note exists in commitment tree)
    // 2. Verify spending key ownership
    // 3. Derive nullifier
    let nullifier = verify_zcash_payment(
        &inputs.spending_key,
        &inputs.note,
        &inputs.merkle_path,
        &inputs.merkle_root,
    );

    // Commit public outputs
    let outputs = ZcashOutputs {
        nullifier,
        payment_verified: true,
    };

    env::commit(&outputs);
}

fn verify_zcash_payment(
    spending_key: &[u8],
    note: &[u8],
    merkle_path: &[u8],
    merkle_root: &[u8; 32],
) -> Nullifier {
    // TODO: Implement actual Zcash Orchard verification
    // Using zcash_primitives and orchard crates:
    // 1. Deserialize note and merkle path
    // 2. Verify merkle path against public root
    // 3. Derive nullifier from spending key and note
    // 4. Return nullifier

    // Placeholder implementation
    let mut nullifier_bytes = [0u8; 32];
    if spending_key.len() >= 32 {
        nullifier_bytes.copy_from_slice(&spending_key[0..32]);
    }
    Nullifier::new(nullifier_bytes)
}
```

**Image ID:** `ZCASH_IMAGE_ID` (generated at build time)

---

#### Guest Program 2: Business Logic Verifier

**Location:** `crates/methods/guest-business/src/main.rs`

```rust
#![no_main]

use risc0_zkvm::guest::env;
use khafi_common::{BusinessInputs, BusinessOutputs};

risc0_zkvm::guest::entry!(main);

fn main() {
    // Read business inputs
    let inputs: BusinessInputs = env::read();

    // Execute custom business logic validation
    // This function will be REPLACED by the Logic Compiler
    // with customer-specific verification code
    let compliance_result = execute_business_logic(
        &inputs.private_data,
        &inputs.public_params,
    );

    // Commit public outputs
    let outputs = BusinessOutputs {
        compliance_result,
        metadata: vec![],
    };

    env::commit(&outputs);
}

// This function will be replaced by Logic Compiler
fn execute_business_logic(
    private_data: &[u8],
    public_params: &[u8],
) -> bool {
    // Examples of what this becomes:

    // PHARMA:
    // let prescription: Prescription = deserialize(private_data);
    // let params: PharmaParams = deserialize(public_params);
    // return prescription.quantity <= params.max_quantity
    //     && prescription.patient_age >= params.min_age
    //     && verify_doctor_signature(&prescription);

    // SHIPPING:
    // let manifest: Manifest = deserialize(private_data);
    // let params: ShippingParams = deserialize(public_params);
    // return !params.sanctioned_countries.contains(&manifest.destination)
    //     && manifest.weight <= params.max_weight;

    // Placeholder
    !private_data.is_empty() && !public_params.is_empty()
}
```

**Image ID:** `BUSINESS_IMAGE_ID` (unique per customer, generated by Logic Compiler)

---

### 2. Proving Service

**Location:** `crates/proving-service/`

**Purpose:** HTTP API that generates both ZK proofs using RISC0

**Key Properties:**
- ✓ Stateless (doesn't store user data)
- ✓ Deployed in customer's infrastructure
- ✓ Can be replaced with Bonsai for production

#### Directory Structure

```
crates/proving-service/
├── Cargo.toml
├── Dockerfile
└── src/
    ├── main.rs              # HTTP server (Axum)
    ├── zcash_prover.rs      # Zcash proof generation
    ├── business_prover.rs   # Business proof generation
    └── config.rs            # Configuration
```

#### Main API Endpoint

**POST /v1/generate-proofs**

Request:
```json
{
  "zcash_inputs": {
    "spending_key": "0x...",  // 32 bytes hex
    "note": "0x...",          // Orchard note bytes
    "merkle_path": "0x...",   // Merkle proof path
    "merkle_root": "0x..."    // Current commitment tree root
  },
  "business_inputs": {
    "private_data": {
      // Customer-specific structure
      // Pharma example:
      "prescription_id": "RX12345",
      "patient_dob": "1990-01-01",
      "medication": "...",
      "quantity": 30
    },
    "public_params": {
      "max_quantity": 90,
      "min_age": 18,
      "pharmacy_id": "PH789"
    }
  }
}
```

Response:
```json
{
  "zcash_proof": "0x...",        // Serialized Receipt (hex)
  "business_proof": "0x...",     // Serialized Receipt (hex)
  "nullifier": "0x...",          // Nullifier from Zcash proof
  "image_ids": {
    "zcash": "0x...",           // ZCASH_IMAGE_ID
    "business": "0x..."         // BUSINESS_IMAGE_ID
  },
  "outputs": {
    "payment_verified": true,
    "compliance_result": true,
    "metadata": {}
  }
}
```

Error Response:
```json
{
  "error": "proof_generation_failed",
  "message": "Invalid merkle proof",
  "code": 400
}
```

---

### 3. ZK Verification Service (Updated)

**Location:** `crates/zk-verification-service/`

**Changes:** Now verifies TWO proofs instead of one

#### Updated ExtAuth Check

```rust
// src/service.rs
async fn check(
    &self,
    request: Request<CheckRequest>,
) -> Result<Response<CheckResponse>, Status> {
    let req = request.into_inner();

    // Extract THREE headers
    let zcash_proof_hex = req.headers.get("x-zk-receipt-payment")
        .ok_or_else(|| Status::unauthenticated("Missing x-zk-receipt-payment"))?;

    let business_proof_hex = req.headers.get("x-zk-receipt-business")
        .ok_or_else(|| Status::unauthenticated("Missing x-zk-receipt-business"))?;

    let nullifier_hex = req.headers.get("x-zk-nullifier")
        .ok_or_else(|| Status::unauthenticated("Missing x-zk-nullifier"))?;

    // Parse nullifier
    let nullifier = Nullifier::from_hex(nullifier_hex)
        .map_err(|e| Status::invalid_argument(format!("Invalid nullifier: {}", e)))?;

    // Check nullifier replay FIRST (before expensive verification)
    let is_new = self.nullifier_checker.check_and_set(&nullifier).await
        .map_err(|e| Status::unavailable(format!("Redis error: {}", e)))?;

    if !is_new {
        return Ok(Response::new(CheckResponse {
            status: StatusCode::Unauthenticated as i32,
            message: "Nullifier replay detected".to_string(),
            metadata: Default::default(),
        }));
    }

    // Verify Zcash payment proof
    let zcash_outputs = self.verify_zcash_proof(zcash_proof_hex).await?;

    // Verify business logic proof
    let business_outputs = self.verify_business_proof(business_proof_hex).await?;

    // Cross-check: nullifier from proof matches header
    if zcash_outputs.nullifier != nullifier {
        return Ok(Response::new(CheckResponse {
            status: StatusCode::PermissionDenied as i32,
            message: "Nullifier mismatch".to_string(),
            metadata: Default::default(),
        }));
    }

    // Check compliance result
    if !business_outputs.compliance_result {
        return Ok(Response::new(CheckResponse {
            status: StatusCode::PermissionDenied as i32,
            message: "Business logic validation failed".to_string(),
            metadata: Default::default(),
        }));
    }

    // All checks passed
    Ok(Response::new(CheckResponse {
        status: StatusCode::Ok as i32,
        message: "Both proofs verified successfully".to_string(),
        metadata: Default::default(),
    }))
}

// Helper: Verify Zcash proof
async fn verify_zcash_proof(&self, proof_hex: &str) -> Result<ZcashOutputs, Status> {
    let proof_bytes = hex::decode(proof_hex)
        .map_err(|e| Status::invalid_argument(format!("Invalid proof hex: {}", e)))?;

    let (receipt, _): (Receipt, usize) = bincode::serde::decode_from_slice(
        &proof_bytes,
        bincode::config::standard(),
    ).map_err(|e| Status::invalid_argument(format!("Decode error: {}", e)))?;

    // Verify and decode in one step
    receipt.verify_and_decode(&self.config.zcash_image_id)
        .map_err(|e| Status::permission_denied(format!("Zcash proof invalid: {}", e)))
}

// Helper: Verify Business proof
async fn verify_business_proof(&self, proof_hex: &str) -> Result<BusinessOutputs, Status> {
    let proof_bytes = hex::decode(proof_hex)
        .map_err(|e| Status::invalid_argument(format!("Invalid proof hex: {}", e)))?;

    let (receipt, _): (Receipt, usize) = bincode::serde::decode_from_slice(
        &proof_bytes,
        bincode::config::standard(),
    ).map_err(|e| Status::invalid_argument(format!("Decode error: {}", e)))?;

    // Verify and decode in one step
    receipt.verify_and_decode(&self.config.business_image_id)
        .map_err(|e| Status::permission_denied(format!("Business proof invalid: {}", e)))
}
```

#### Updated Configuration

```rust
// src/config.rs
pub struct Config {
    pub redis_url: String,
    pub zcash_image_id: [u8; 32],      // Image ID for Zcash verifier
    pub business_image_id: [u8; 32],   // Image ID for business verifier
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            zcash_image_id: load_image_id("ZCASH_IMAGE_ID"),
            business_image_id: load_image_id("BUSINESS_IMAGE_ID"),
        }
    }
}

fn load_image_id(env_var: &str) -> [u8; 32] {
    let hex_str = std::env::var(env_var)
        .expect(&format!("{} must be set", env_var));

    let bytes = hex::decode(hex_str)
        .expect(&format!("Invalid hex for {}", env_var));

    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    arr
}
```

---

### 4. Envoy Configuration (Updated)

**Location:** `envoy/envoy.yaml`

**No changes needed!** ExtAuth filter automatically forwards all headers to the verification service.

---

## API Specifications

### Proving Service API

#### POST /v1/generate-proofs

**Description:** Generate both Zcash and business logic ZK proofs

**Request Headers:**
- `Content-Type: application/json`

**Request Body:**
```typescript
interface ProofRequest {
  zcash_inputs: {
    spending_key: string;      // hex-encoded 32 bytes
    note: string;              // hex-encoded Orchard note
    merkle_path: string;       // hex-encoded merkle proof
    merkle_root: string;       // hex-encoded 32 bytes
  };
  business_inputs: {
    private_data: object;      // Customer-specific structure
    public_params: object;     // Customer-specific parameters
  };
}
```

**Response (Success - 200 OK):**
```typescript
interface ProofResponse {
  zcash_proof: string;         // hex-encoded Receipt
  business_proof: string;      // hex-encoded Receipt
  nullifier: string;           // hex-encoded 32 bytes
  image_ids: {
    zcash: string;            // hex-encoded 32 bytes
    business: string;         // hex-encoded 32 bytes
  };
  outputs: {
    payment_verified: boolean;
    compliance_result: boolean;
    metadata?: object;
  };
}
```

**Error Responses:**

400 Bad Request:
```json
{
  "error": "invalid_input",
  "message": "Missing spending_key",
  "field": "zcash_inputs.spending_key"
}
```

500 Internal Server Error:
```json
{
  "error": "proof_generation_failed",
  "message": "RISC0 prover error: ...",
  "retry_after": 5
}
```

---

#### GET /health

**Description:** Health check endpoint

**Response (200 OK):**
```json
{
  "status": "healthy",
  "services": {
    "zcash_prover": "ok",
    "business_prover": "ok"
  },
  "version": "0.1.0"
}
```

---

### Protected API Headers

When calling protected APIs behind khafi-gateway, include:

```http
GET /api/prescription HTTP/1.1
Host: customer.com
x-zk-receipt-payment: 0x1234abcd...     # Zcash proof
x-zk-receipt-business: 0x5678ef01...    # Business proof
x-zk-nullifier: 0xabcd1234...           # Nullifier
Content-Type: application/json
```

**Gateway validates:**
1. Both proofs are valid cryptographic proofs
2. Proofs match expected Image IDs
3. Nullifier from Zcash proof matches header
4. Nullifier hasn't been used before (replay check)
5. Business logic compliance result is true

**If all pass:** Request forwarded to protected API
**If any fail:** 403 Forbidden returned immediately

---

## Trust Model & Security

### Threat Model

#### What We Protect Against:

1. ✅ **Replay Attacks**
   - Nullifier stored in Redis
   - Duplicate nullifiers rejected
   - Each payment can only be used once

2. ✅ **Invalid Payments**
   - Zcash proof cryptographically verifies spending key ownership
   - Merkle proof ensures note exists in commitment tree
   - Can't fake a payment

3. ✅ **Business Logic Bypass**
   - Business proof verifies compliance rules
   - Can't skip validation (gateway checks both proofs)
   - Rules enforced in zkVM (can't tamper)

4. ✅ **Proof Forgery**
   - RISC0 zkVM provides cryptographic security
   - Can't create valid proof without valid inputs
   - Image ID prevents proof substitution

#### What Users Must Trust:

1. **Proving Service (if customer-hosted)**
   - Users send secrets (spending key, private data) to generate proofs
   - Service is STATELESS (doesn't store data)
   - Customer can audit the code
   - **Mitigation:** Use Bonsai instead (industry-standard, no customer code)

2. **Customer's Infrastructure**
   - Proving Service and Gateway run in customer's environment
   - Customer could theoretically log secrets
   - **Mitigation:** Open-source code + audit logs + compliance

3. **RISC0 zkVM**
   - Trust RISC0's cryptographic implementation
   - Industry-standard, audited by Quantstamp
   - Open-source

#### Attack Scenarios:

**Scenario 1: User sends fake payment proof**
- ❌ Blocked: Gateway verifies proof against `ZCASH_IMAGE_ID`
- Invalid proof → 403 Forbidden

**Scenario 2: User reuses same proof twice**
- ❌ Blocked: Nullifier stored in Redis
- Second request → 403 "Nullifier replay detected"

**Scenario 3: User passes business validation but not payment**
- ❌ Blocked: Gateway checks BOTH proofs
- Missing or invalid Zcash proof → 403 Forbidden

**Scenario 4: Malicious customer modifies guest program**
- ❌ Detected: Image ID changes when code changes
- Gateway rejects proof (doesn't match expected Image ID)

**Scenario 5: User tries to skip proving service**
- ❌ Impossible: Can't generate valid proof without RISC0
- Browser/mobile can't run zkVM locally

---

### Privacy Guarantees

| Data | Visibility | Stored? | Who Sees It |
|------|-----------|---------|-------------|
| Spending Key | Proving Service only | ❌ No | Proving Service (stateless) |
| Private Data | Proving Service only | ❌ No | Proving Service (stateless) |
| Zcash Proof | Gateway + Protected API | ❌ No | Gateway (verifies), API (optional) |
| Business Proof | Gateway + Protected API | ❌ No | Gateway (verifies), API (optional) |
| Nullifier | Gateway | ✅ Yes (Redis) | Gateway (replay prevention) |
| Compliance Result | Gateway + Protected API | ❌ No | Extracted from proof journal |

**Key Point:** The gateway NEVER sees spending keys or private data. It only sees cryptographic proofs.

---

## Implementation Plan

### Phase 1: Split Guest Programs ✅ Ready

**Goal:** Create separate Zcash and Business guest programs

**Tasks:**
1. Create `crates/methods/guest-zcash/`
   - Copy current guest program structure
   - Remove business logic validation
   - Focus only on Zcash verification
   - Update `Cargo.toml` to generate `ZCASH_IMAGE_ID`

2. Create `crates/methods/guest-business/`
   - Copy current guest program structure
   - Remove Zcash verification
   - Focus only on business logic
   - Update `Cargo.toml` to generate `BUSINESS_IMAGE_ID`

3. Update `crates/common/src/inputs.rs`
   - Add `ZcashOutputs` struct
   - Add `BusinessOutputs` struct
   - Keep existing `GuestInputs` (now used separately)

4. Update `crates/methods/build.rs`
   - Build both guest programs
   - Export both Image IDs

**Success Criteria:**
- Both guest programs compile
- Both generate separate Image IDs
- Unit tests pass

---

### Phase 2: Implement Proving Service 🔨 Next

**Goal:** HTTP API for proof generation

**Tasks:**
1. Create `crates/proving-service/` crate
   - Add Axum dependencies
   - Set up HTTP server

2. Implement proof generation endpoints
   - `POST /v1/generate-proofs`
   - `GET /health`

3. Add error handling
   - Invalid input validation
   - RISC0 error handling
   - Rate limiting (optional)

4. Write tests
   - Unit tests for each prover
   - Integration test for full flow

5. Create Dockerfile
   - Multi-stage build
   - Optimize for size

**Success Criteria:**
- API returns both proofs
- Proofs are valid (verifiable)
- Performance: <5s per proof on dev machine

---

### Phase 3: Update Gateway Verification 🔨 Next

**Goal:** Verify both proofs in ExtAuth filter

**Tasks:**
1. Update `zk-verification-service/src/config.rs`
   - Add `business_image_id` field
   - Load from environment

2. Update `zk-verification-service/src/service.rs`
   - Extract both proof headers
   - Verify Zcash proof
   - Verify business proof
   - Cross-check nullifier
   - Check compliance result

3. Update error messages
   - Distinguish Zcash vs business failures
   - Add detailed logging

4. Write tests
   - Test with both valid proofs
   - Test with one invalid proof
   - Test nullifier mismatch
   - Test replay attack

**Success Criteria:**
- Gateway accepts only when both proofs valid
- Replay attacks blocked
- Proper error messages

---

### Phase 4: Client SDK (Browser/Mobile) 📱 Future

**Goal:** Make it easy for any platform to use Khafi

**Tasks:**
1. JavaScript/TypeScript SDK
   - `generateProofs()` function
   - `callProtectedAPI()` helper
   - TypeScript types

2. Example applications
   - React web app
   - React Native mobile app
   - Node.js backend

3. Documentation
   - Quick start guide
   - API reference
   - Common patterns

**Success Criteria:**
- Browser can generate and use proofs
- Mobile app works on iOS/Android
- Documentation complete

---

### Phase 5: Logic Compiler Integration 🤖 Future

**Goal:** Generate custom business guest programs from JSON DSL

**Tasks:**
1. Update Logic Compiler to generate guest-business code
2. Build system for per-customer Image IDs
3. Registry of customer Image IDs in gateway

(Details in separate ADR)

---

### Phase 6: Production Hardening 🔐 Future

**Goal:** Production-ready deployment

**Tasks:**
1. Replace local prover with Bonsai (optional)
2. Add monitoring and metrics
3. Security audit
4. Load testing
5. Documentation

---

## Comparison: Single vs Two-Proof

| Aspect | Single Proof (Current) | Two-Proof (Proposed) |
|--------|----------------------|---------------------|
| **Proof Generation** | One zkVM execution | Two zkVM executions (parallel) |
| **Proving Time** | ~3-5s | ~6-10s (but can parallelize) |
| **Proof Size** | ~200-300KB | ~400-600KB (2x) |
| **Bandwidth** | Lower (1 proof) | Higher (2 proofs) |
| **Platform Support** | Requires native code | **Same** (both need prover) |
| **Flexibility** | Combined verification only | Can verify independently |
| **Image IDs** | 1 per customer | 2 per customer (Zcash is universal) |
| **Gateway Complexity** | Lower (1 verification) | Higher (2 verifications + cross-check) |
| **Privacy** | Both secrets to prover | **Same** (both secrets to prover) |
| **Security** | Same cryptographic guarantees | **Same** cryptographic guarantees |
| **Use Cases** | Payment + business must pair | Can verify payment without business |
| **Future: Separate Provers** | Not possible | Zcash prover separate from business |

### Why Two-Proof Is Better:

1. **Modularity:** Can upgrade Zcash verifier without touching business logic
2. **Reusability:** Same Zcash verifier for all customers (only business varies)
3. **Debugging:** Can test payment and business verification independently
4. **Flexibility:** Could support payment-only or business-only modes in future
5. **Clarity:** Clean separation of concerns (payment vs compliance)

### Tradeoffs:

1. ⚠️ **Performance:** 2x proof generation time and size
   - **Mitigation:** Parallelize proving, use Bonsai
2. ⚠️ **Complexity:** More moving parts
   - **Mitigation:** Clear architecture, good docs
3. ⚠️ **Bandwidth:** 2x proof data in headers
   - **Mitigation:** ~500KB total is acceptable for most use cases

**Recommendation:** Proceed with two-proof architecture for long-term flexibility.

---

## Client SDK Examples

### JavaScript/TypeScript (Browser)

```typescript
// khafi-sdk.ts
export class KhafiSDK {
  constructor(
    private provingServiceUrl: string,
    private gatewayUrl: string
  ) {}

  async generateProofs(
    zcashInputs: ZcashInputs,
    businessInputs: BusinessInputs
  ): Promise<ProofResponse> {
    const response = await fetch(
      `${this.provingServiceUrl}/v1/generate-proofs`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          zcash_inputs: zcashInputs,
          business_inputs: businessInputs,
        }),
      }
    );

    if (!response.ok) {
      throw new Error(`Proof generation failed: ${response.statusText}`);
    }

    return response.json();
  }

  async callProtectedAPI(
    proofs: ProofResponse,
    endpoint: string,
    options?: RequestInit
  ): Promise<Response> {
    return fetch(`${this.gatewayUrl}${endpoint}`, {
      ...options,
      headers: {
        ...options?.headers,
        'x-zk-receipt-payment': proofs.zcash_proof,
        'x-zk-receipt-business': proofs.business_proof,
        'x-zk-nullifier': proofs.nullifier,
      },
    });
  }
}

// Usage example
const sdk = new KhafiSDK(
  'https://proving.example.com',
  'https://api.example.com'
);

// Generate proofs
const proofs = await sdk.generateProofs(
  {
    spending_key: '0x...',
    note: '0x...',
    merkle_path: '0x...',
    merkle_root: '0x...',
  },
  {
    private_data: {
      prescription_id: 'RX12345',
      quantity: 30,
    },
    public_params: {
      max_quantity: 90,
      min_age: 18,
    },
  }
);

// Call protected API
const response = await sdk.callProtectedAPI(proofs, '/api/prescription');
const data = await response.json();
console.log('Prescription approved:', data);
```

---

### React Example

```tsx
// PrescriptionForm.tsx
import { KhafiSDK } from './khafi-sdk';

function PrescriptionForm() {
  const [loading, setLoading] = useState(false);
  const sdk = new KhafiSDK(
    process.env.REACT_APP_PROVING_URL!,
    process.env.REACT_APP_GATEWAY_URL!
  );

  async function handleSubmit(formData: any) {
    setLoading(true);
    try {
      // Step 1: Generate proofs
      const proofs = await sdk.generateProofs(
        formData.zcashPayment,
        formData.prescription
      );

      // Step 2: Call protected API
      const response = await sdk.callProtectedAPI(
        proofs,
        '/api/prescriptions',
        {
          method: 'POST',
          body: JSON.stringify(formData.details),
        }
      );

      if (response.ok) {
        alert('Prescription submitted successfully!');
      } else {
        alert('Verification failed: ' + (await response.text()));
      }
    } catch (error) {
      alert('Error: ' + error.message);
    } finally {
      setLoading(false);
    }
  }

  return (
    <form onSubmit={handleSubmit}>
      {/* Form fields */}
      <button type="submit" disabled={loading}>
        {loading ? 'Generating proof...' : 'Submit Prescription'}
      </button>
    </form>
  );
}
```

---

### Python Backend Example

```python
# khafi_sdk.py
import requests
from typing import Dict, Any

class KhafiSDK:
    def __init__(self, proving_url: str, gateway_url: str):
        self.proving_url = proving_url
        self.gateway_url = gateway_url

    def generate_proofs(
        self,
        zcash_inputs: Dict[str, str],
        business_inputs: Dict[str, Any]
    ) -> Dict[str, str]:
        response = requests.post(
            f"{self.proving_url}/v1/generate-proofs",
            json={
                "zcash_inputs": zcash_inputs,
                "business_inputs": business_inputs
            }
        )
        response.raise_for_status()
        return response.json()

    def call_protected_api(
        self,
        proofs: Dict[str, str],
        endpoint: str,
        method: str = "GET",
        **kwargs
    ):
        headers = {
            "x-zk-receipt-payment": proofs["zcash_proof"],
            "x-zk-receipt-business": proofs["business_proof"],
            "x-zk-nullifier": proofs["nullifier"],
            **kwargs.get("headers", {})
        }

        return requests.request(
            method,
            f"{self.gateway_url}{endpoint}",
            headers=headers,
            **{k: v for k, v in kwargs.items() if k != "headers"}
        )

# Usage
sdk = KhafiSDK(
    proving_url="https://proving.example.com",
    gateway_url="https://api.example.com"
)

proofs = sdk.generate_proofs(
    zcash_inputs={
        "spending_key": "0x...",
        "note": "0x...",
        "merkle_path": "0x...",
        "merkle_root": "0x..."
    },
    business_inputs={
        "private_data": {"prescription_id": "RX12345"},
        "public_params": {"max_quantity": 90}
    }
)

response = sdk.call_protected_api(proofs, "/api/prescriptions")
print(response.json())
```

---

## Deployment Guide

### Docker Compose Configuration

```yaml
# docker-compose.yml
version: '3.8'

services:
  # Redis for nullifier storage
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis-data:/data
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 5s
      timeout: 3s
      retries: 5
    networks:
      - khafi-network

  # NEW: Proving Service
  proving-service:
    build:
      context: .
      dockerfile: crates/proving-service/Dockerfile
    ports:
      - "8082:8082"
    environment:
      - RUST_LOG=info
      - ZCASH_IMAGE_ID=${ZCASH_IMAGE_ID}
      - BUSINESS_IMAGE_ID=${BUSINESS_IMAGE_ID}
    networks:
      - khafi-network
    deploy:
      resources:
        limits:
          cpus: '4'
          memory: 8G

  # ZK Verification Service (updated to verify both proofs)
  zk-verification-service:
    build:
      context: .
      dockerfile: crates/zk-verification-service/Dockerfile
    ports:
      - "50051:50051"
    environment:
      - REDIS_URL=redis://redis:6379
      - ZCASH_IMAGE_ID=${ZCASH_IMAGE_ID}
      - BUSINESS_IMAGE_ID=${BUSINESS_IMAGE_ID}
      - RUST_LOG=info
    depends_on:
      redis:
        condition: service_healthy
    networks:
      - khafi-network

  # Zcash Backend
  zcash-backend:
    build:
      context: .
      dockerfile: crates/zcash-backend/Dockerfile
    ports:
      - "8081:8081"
    environment:
      - REDIS_URL=redis://redis:6379
      - RUST_LOG=info
    depends_on:
      redis:
        condition: service_healthy
    networks:
      - khafi-network

  # Envoy Gateway
  envoy:
    build:
      context: ./envoy
      dockerfile: Dockerfile.envoy
    ports:
      - "8080:8080"
      - "9901:9901"
    depends_on:
      - zk-verification-service
    networks:
      - khafi-network

  # Mock upstream (for testing)
  mock-upstream:
    build:
      context: ./mock-upstream
    ports:
      - "8083:8083"
    networks:
      - khafi-network

volumes:
  redis-data:

networks:
  khafi-network:
    driver: bridge
```

### Environment Variables

```bash
# .env
REDIS_URL=redis://redis:6379
RUST_LOG=info

# Image IDs (generated at build time)
ZCASH_IMAGE_ID=0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef
BUSINESS_IMAGE_ID=0xfedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321
```

### Build and Run

```bash
# Build all services
docker-compose build

# Start all services
docker-compose up -d

# View logs
docker-compose logs -f proving-service
docker-compose logs -f zk-verification-service

# Health checks
curl http://localhost:8082/health  # Proving service
curl http://localhost:9901/ready   # Envoy gateway
curl http://localhost:8081/health  # Zcash backend

# Stop all services
docker-compose down
```

### Testing the Stack

```bash
# 1. Generate proofs
curl -X POST http://localhost:8082/v1/generate-proofs \
  -H "Content-Type: application/json" \
  -d '{
    "zcash_inputs": {
      "spending_key": "0x0000000000000000000000000000000000000000000000000000000000000001",
      "note": "0x...",
      "merkle_path": "0x...",
      "merkle_root": "0x..."
    },
    "business_inputs": {
      "private_data": {"test": "data"},
      "public_params": {"test": "params"}
    }
  }' > proofs.json

# Extract proofs from response
ZCASH_PROOF=$(jq -r '.zcash_proof' proofs.json)
BUSINESS_PROOF=$(jq -r '.business_proof' proofs.json)
NULLIFIER=$(jq -r '.nullifier' proofs.json)

# 2. Call protected API with proofs
curl -v http://localhost:8080/api/test \
  -H "x-zk-receipt-payment: $ZCASH_PROOF" \
  -H "x-zk-receipt-business: $BUSINESS_PROOF" \
  -H "x-zk-nullifier: $NULLIFIER"

# Should return 200 OK (first time)

# 3. Try replay attack (same nullifier)
curl -v http://localhost:8080/api/test \
  -H "x-zk-receipt-payment: $ZCASH_PROOF" \
  -H "x-zk-receipt-business: $BUSINESS_PROOF" \
  -H "x-zk-nullifier: $NULLIFIER"

# Should return 403 Forbidden (replay detected)
```

---

## Next Steps

1. **Review and approve this architecture document**
2. **Implement Phase 1:** Split guest programs
3. **Implement Phase 2:** Proving Service
4. **Implement Phase 3:** Update Gateway
5. **Create client SDKs** for JavaScript, Python, Rust
6. **Production hardening:** Bonsai integration, monitoring, security audit

---

## References

- [RISC0 Documentation](https://dev.risczero.com/)
- [Zcash Protocol Specification](https://zips.z.cash/)
- [Envoy ExtAuth Filter](https://www.envoyproxy.io/docs/envoy/latest/configuration/http/http_filters/ext_authz_filter)
- [Implementation Plan](../implementation-plan.md)

---

**Document Status:** Proposed
**Last Updated:** 2025-11-18
**Next Review:** After Phase 1 implementation
