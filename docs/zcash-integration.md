# Zcash Integration Guide

## Table of Contents
1. [Zcash Fundamentals](#zcash-fundamentals)
2. [Orchard Protocol Overview](#orchard-protocol-overview)
3. [Payment Verification Flow](#payment-verification-flow)
4. [Integration with RISC Zero](#integration-with-risc-zero)
5. [Current Implementation](#current-implementation)
6. [Implementation Roadmap](#implementation-roadmap)
7. [Code Examples](#code-examples)
8. [References](#references)

---

## Zcash Fundamentals

### Transparent vs Shielded Pools

**Transparent Pool:**
- Unshielded and non-private (similar to Bitcoin)
- Addresses start with "t"
- All transaction details are public
- Supports multi-signature transactions

**Shielded Pools (Sapling and Orchard):**
- Privacy-preserving using zero-knowledge proofs
- Each protocol has a separate anonymity set
- All value belongs to one of: transparent pool, Sprout pool, Sapling pool, or Orchard pool
- Funds can move between pools at minimal cost

### What is a "Note"?

A **note** represents value in the shielded pool. In Orchard, a note has the structure:

```
Note = (addr, v, ρ, ψ, rcm)
```

Where:
- `addr`: Recipient's shielded payment address
- `v`: Value (amount in zatoshis)
- `ρ` (rho): Unique value derived from nullifier of spent note in same action
- `ψ` (psi): Sender-controlled randomness (derived from ρ and rseed)
- `rcm`: Randomness for note commitment

**Note Commitment:** Each note has a cryptographically associated commitment using Sinsemilla hash:
```
cm = NoteCommitOrchard(addr, v, ρ, ψ, rcm)
```

This commitment is added to the global commitment tree.

### What is a "Nullifier"?

A **nullifier** is a unique identifier that prevents double-spending. In Orchard:

```
nf = ExtractP([(Fnk(ρ) + ψ) mod p]G + cm)
```

Where:
- `Fnk`: Keyed pseudorandom function (Poseidon) using nullifier deriving key `nk`
- `ρ`: Ensures uniqueness
- `ψ`: Privacy blinding factor
- `G`: Fixed independent base point
- `cm`: Note commitment
- `ExtractP`: Extracts x-coordinate (32 bytes)

**Key Properties:**
- Nullifier deterministically derives from values in the note commitment
- Ensures **only one nullifier per note**
- Prevents double-spending while maintaining privacy
- Safe to reveal publicly (doesn't link to specific note due to blinding)

**Derivation Chain:**
```
Spending Key
    ↓ BLAKE2b
Nullifier Deriving Key (nk)
    ↓ Poseidon(nk, note data)
Nullifier
```

### What is a "Commitment Tree"?

The **commitment tree** is a global Merkle tree tracking all note commitments.

**Structure:**
- **Fixed depth:** 32 levels
- **Global and incremental:** Single tree, commitments appended from blocks sequentially
- **Hash function:** Sinsemilla (Orchard) / Bowe-Hopwood Pedersen (Sapling)
- **Uncommitted value:** 2 (sentinel value)

**Anchor:** The Merkle root of a commitment tree at a block boundary.
- Uniquely identifies the tree state
- Serves as public input for proofs
- Valid anchors correspond to global tree state at block boundaries

---

## Orchard Protocol Overview

### Sapling vs Orchard

**Sapling (2018):**
- Addresses begin with "zs"
- Uses Jubjub elliptic curve
- Separate descriptions for each input/output
- Bowe-Hopwood Pedersen hash
- Required trusted setup

**Orchard (Latest - Recommended):**
- Unified addresses (no standalone Orchard addresses)
- Pallas/Vesta curves (recursion-friendly)
- **No trusted setup** required
- Each **Action** handles spend + output simultaneously
- Sinsemilla hash function
- Arity-hiding without proof size bloat

**Key Innovation:** Orchard combines spending and outputs into unified "Actions" - more efficient than Sapling's separate descriptions.

### Key Hierarchy

```
Spending Key (sk) - Most privileged, enables spending
    ↓ derive
Nullifier Deriving Key (nk) - Used for nullifiers
    ↓ derive
Full Viewing Key (fvk) - See all transactions
    ↓ derive
Incoming Viewing Key (ivk) - See only incoming
    ↓ derive
Payment Address - Public identifier
```

**Key Roles:**
- **Spending Key:** Private key for spending notes, deriving nullifiers
- **Viewing Keys:** Read-only access for auditing without spending authority
- **Address:** Public identifier for receiving payments

---

## Payment Verification Flow

### How Zcash Payments Work

```
┌─────────────────────────────────────────────────────┐
│  User Spending a Shielded Note                      │
│                                                      │
│  User possesses:                                    │
│  • Spending key (private)                           │
│  • Note details (addr, v, ρ, ψ, rcm)               │
│  • Merkle path proving note exists in tree          │
│  • Current anchor (merkle root)                     │
└─────────────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────┐
│  Create Orchard Action                              │
│  1. Derive nullifier from spending key + note       │
│  2. Create new output note (if transferring)        │
│  3. Construct value commitment                      │
│  4. Generate zkSNARK proof                          │
└─────────────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────┐
│  Broadcast Transaction                              │
│  • Reveals: nullifier, new note commitment(s)       │
│  • Includes: zkSNARK proof, signature               │
│  • Public: anchor, value commitments                │
└─────────────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────┐
│  Blockchain Consensus Verifies                      │
│  ✓ zkSNARK proof is valid                           │
│  ✓ Nullifier not in nullifier set (no replay)      │
│  ✓ Anchor is valid historical tree state            │
│  ✓ Value balance checks out                         │
│  ✓ Signature is valid                               │
└─────────────────────────────────────────────────────┘
```

### What's Public vs Private

**PUBLIC (on blockchain):**
- Transaction ID
- Nullifiers (reveals *some* note was spent, but not which)
- Note commitments (new outputs)
- Value commitments (homomorphic)
- zkSNARK proof
- Anchor (merkle root)
- Signature

**PRIVATE (hidden by ZK proof):**
- Sender address
- Recipient address
- Transaction amount
- Which specific note in tree was spent
- Spending key
- Note details (v, ρ, ψ, rcm)
- Merkle path

### The zkSNARK Proof

The Orchard zkSNARK proves (without revealing private data):

1. **Input validity:** Merkle path from note commitment to anchor is valid
2. **Spending authority:** Prover knows spending key for the note
3. **Nullifier correctness:** Nullifier correctly derived from spending key and note
4. **Value balance:** Input values sum to output values (+ fees)
5. **Signature linkage:** Keys cryptographically linked to transaction signature

**Circuit Inputs:**
- **Private:** Spending key, note details, merkle path, randomness
- **Public:** Anchor, nullifier, value commitment, note commitment (output)

---

## Integration with RISC Zero

### Architecture Overview

Khafi Gateway uses **RISC Zero** to create a second layer of zero-knowledge proofs that combine:
1. **Zcash payment proof** (proves a note was spent)
2. **Business logic validation** (proves custom compliance rules)

```
┌──────────────────────────────────────────────────────────┐
│                  Khafi Gateway Flow                       │
└──────────────────────────────────────────────────────────┘
                          │
          ┌───────────────┴───────────────┐
          │                               │
          ▼                               ▼
┌──────────────────┐           ┌──────────────────┐
│  Zcash Orchard   │           │  Business Logic  │
│  zkSNARK         │           │  Validation      │
│  (on-chain)      │           │  (in RISC Zero)  │
└──────────────────┘           └──────────────────┘
          │                               │
          │     Combined in RISC Zero     │
          └───────────────┬───────────────┘
                          │
                          ▼
            ┌──────────────────────────┐
            │  Single Proof Verifies:  │
            │  1. Zcash payment valid  │
            │  2. Business rules met   │
            │  3. Nullifier unique     │
            └──────────────────────────┘
```

### Why Use RISC Zero + Zcash?

**Zcash zkSNARK alone:**
- Proves: "I spent a valid note"
- Does NOT prove: Custom business logic

**RISC Zero zkVM:**
- Can verify Zcash proof
- Can execute arbitrary business logic
- Combines both into single proof

**Benefits:**
- **Privacy:** Spending key never leaves zkVM
- **Flexibility:** Any business rules via Logic Compiler
- **Efficiency:** One proof for payment + compliance
- **Composability:** Zcash verification is standard, business logic is customizable

### Data Flow

```
┌────────────────────────────────────────────────────────┐
│  1. User's Wallet/SDK                                  │
│     • Has: spending key, note data                     │
│     • Fetches: current anchor from zcash-backend       │
│     • Builds: Merkle path from wallet's local tree     │
└────────────────────────────────────────────────────────┘
                          │
                          ▼ ZcashInputs + BusinessInputs
┌────────────────────────────────────────────────────────┐
│  2. RISC Zero Guest Program (zkVM)                     │
│  ┌──────────────────────────────────────────────────┐  │
│  │  verify_zcash_payment():                         │  │
│  │   • Deserialize Orchard note                     │  │
│  │   • Compute note commitment                      │  │
│  │   • Verify Merkle path against anchor            │  │
│  │   • Derive nullifier using Poseidon hash         │  │
│  │   • Validate spending authority                  │  │
│  └──────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────┐  │
│  │  execute_business_logic():                       │  │
│  │   • Custom validation rules (from Logic Compiler)│  │
│  │   • Age verification, signature checks, etc.     │  │
│  └──────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────┐  │
│  │  env::commit(GuestOutputs):                      │  │
│  │   • nullifier (for replay prevention)            │  │
│  │   • compliance_result (bool)                     │  │
│  │   • metadata (optional attestations)             │  │
│  └──────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────┘
                          │
                          ▼ Receipt (proof + journal)
┌────────────────────────────────────────────────────────┐
│  3. zk-verification-service (Envoy ExtAuth)            │
│     • Verify RISC Zero proof cryptographically         │
│     • Check nullifier uniqueness in Redis              │
│     • Extract compliance_result from journal           │
│     • Allow/deny API request based on compliance       │
└────────────────────────────────────────────────────────┘
```

### Input/Output Schema

**ZcashInputs** (from `crates/common/src/inputs.rs`):
```rust
pub struct ZcashInputs {
    pub spending_key: Vec<u8>,    // Spending key (32 bytes, PRIVATE)
    pub note: Vec<u8>,            // Serialized Orchard note (PRIVATE)
    pub merkle_path: Vec<u8>,     // Merkle path (~1KB, PRIVATE)
    pub merkle_root: [u8; 32],    // Anchor from blockchain (PUBLIC)
}
```

**GuestOutputs** (committed to journal):
```rust
pub struct GuestOutputs {
    pub nullifier: Nullifier,          // [u8; 32] - prevents replay
    pub compliance_result: bool,        // Did validation pass?
    pub metadata: Vec<u8>,             // Optional attestations
}
```

---

## Current Implementation

### Status: Placeholder ⏳

The Zcash verification is currently **mocked** with placeholders to prove the architecture works.

**Current code** (`crates/methods/guest/src/main.rs`):
```rust
fn verify_zcash_payment(inputs: &ZcashInputs) -> Nullifier {
    // TODO: Implement actual Zcash Orchard verification
    // For now, placeholder: derive nullifier from spending key
    let mut nullifier_bytes = [0u8; 32];
    if inputs.spending_key.len() >= 32 {
        nullifier_bytes.copy_from_slice(&inputs.spending_key[0..32]);
    }
    Nullifier::new(nullifier_bytes)
}
```

**What's Missing:**
1. ❌ Orchard note deserialization
2. ❌ Merkle path verification using Sinsemilla
3. ❌ Proper nullifier derivation using Poseidon hash
4. ❌ Spending authority verification
5. ❌ Orchard crate integration in guest dependencies

**What Works:**
- ✅ Correct type definitions (ZcashInputs, GuestOutputs)
- ✅ RISC Zero integration complete
- ✅ Proof generation and verification pipeline
- ✅ Nullifier replay prevention via Redis
- ✅ Architecture ready for real Zcash implementation

### Available Crates

Workspace dependencies (already configured):
```toml
zcash_primitives = "0.26.1"   # Core types, merkle trees
orchard = "0.11"               # Orchard protocol implementation
zcash_client_backend = "0.21"  # Wallet/client functions
```

**Key modules:**
- `orchard::Note` - Note structure
- `orchard::tree::MerklePath` - Merkle proof types
- `orchard::keys::SpendingKey` - Key management
- `orchard::primitives::redpallas` - Signature scheme
- `zcash_primitives::merkle_tree` - Merkle tree operations

---

## Implementation Roadmap

### Phase 4: Zcash Integration (Future)

**Three Implementation Approaches:**

#### Option A: Full Orchard (Production-Ready)
**Timeline:** 3-4 weeks
**Effort:** High
**Authenticity:** ✅ Full Zcash compatibility

**Tasks:**
1. Add `orchard` crate to guest dependencies (check `no_std` support)
2. Implement Sinsemilla merkle hash verification
3. Implement Poseidon hash for nullifier derivation
4. Add Pallas/Vesta curve operations for zkVM
5. Build SDK serialization utilities for Orchard types
6. Connect zcash-backend to Zebra testnet node
7. End-to-end testing with real Orchard notes

**Challenges:**
- Orchard crate may not support `no_std` (required for zkVM)
- Cryptographic operations may be slow in zkVM
- Complex serialization between SDK and guest

#### Option B: Simplified (MVP/Demo)
**Timeline:** 1 week
**Effort:** Low
**Authenticity:** ⚠️ Simulated for demonstration

**Tasks:**
1. Mock merkle verification (simple hash comparison)
2. Simple nullifier derivation (SHA256 instead of Poseidon)
3. Simplified note structure
4. Document clearly as "demo mode"

**Use case:** Proving the SaaS concept without full Zcash complexity

#### Option C: Hybrid (Recommended)
**Timeline:** 2 weeks
**Effort:** Medium
**Authenticity:** ✅ Core verification authentic

**Tasks:**
1. Implement real Sinsemilla merkle path verification
2. Implement real Poseidon nullifier derivation
3. Use simplified note structure initially
4. Clear upgrade path to full Orchard

**Balance:** Authenticity where it matters, pragmatism where it doesn't

### Recommended Path

**For SaaS/MVP:** Option B (Simplified)
- Focus on Logic Compiler (unique differentiator)
- Zcash verification is placeholder
- Upgrade to full Orchard post-MVP

**For Production:** Option C (Hybrid) → Option A (Full)
- Start with hybrid to prove architecture
- Incrementally upgrade to full Orchard
- Maintain backward compatibility

---

## Code Examples

### Example 1: Merkle Path Verification (Simplified)

```rust
// In methods/guest/src/main.rs

fn verify_merkle_path(
    commitment: [u8; 32],
    path: &[[u8; 32]],  // 32 siblings for 32-level tree
    expected_root: [u8; 32],
) -> bool {
    let mut current = commitment;

    for sibling in path.iter() {
        // Simplified: just SHA256 hash
        // Real Orchard uses Sinsemilla
        let mut hasher = Sha256::new();
        hasher.update(&current);
        hasher.update(sibling);
        current = hasher.finalize().into();
    }

    current == expected_root
}
```

### Example 2: Full Orchard Verification (Target)

```rust
// Future implementation with orchard crate

use orchard::{Note, keys::SpendingKey, tree::MerklePath};

fn verify_zcash_payment(inputs: &ZcashInputs) -> Result<Nullifier> {
    // 1. Deserialize note
    let note = Note::read(&mut &inputs.note[..])?;

    // 2. Compute note commitment using Sinsemilla
    let commitment = note.commitment();

    // 3. Deserialize and verify merkle path
    let merkle_path = MerklePath::from_bytes(&inputs.merkle_path)?;
    let computed_root = merkle_path.root(commitment);

    // Verify against public anchor
    if computed_root.to_bytes() != inputs.merkle_root {
        return Err("Merkle proof verification failed");
    }

    // 4. Derive nullifier using Poseidon hash
    let spending_key = SpendingKey::from_bytes(&inputs.spending_key)?;
    let fvk = FullViewingKey::from(&spending_key);
    let nullifier = note.nullifier(&fvk);

    Ok(Nullifier::new(nullifier.to_bytes()))
}
```

### Example 3: SDK Integration

```rust
// In sdk-template/src/zcash_client.rs

pub struct ZcashClient {
    backend_url: String,
}

impl ZcashClient {
    pub async fn prepare_zcash_inputs(
        &self,
        spending_key: &SpendingKey,
        note: &Note,
        note_position: u64,
    ) -> Result<ZcashInputs> {
        // 1. Fetch current anchor from zcash-backend
        let anchor_response = self
            .get_commitment_tree_root()
            .await?;

        // 2. Build merkle path from wallet's local tree
        // (In real implementation, wallet maintains local tree)
        let merkle_path = self.build_merkle_path(note_position)?;

        // 3. Serialize for guest program
        Ok(ZcashInputs {
            spending_key: spending_key.to_bytes().to_vec(),
            note: serialize_note(note),
            merkle_path: serialize_merkle_path(&merkle_path),
            merkle_root: anchor_response.root,
        })
    }

    async fn get_commitment_tree_root(&self) -> Result<AnchorResponse> {
        let response = reqwest::get(
            format!("{}/commitment-tree/root", self.backend_url)
        ).await?;

        response.json().await
    }
}
```

---

## References

### Documentation
- [Zcash Protocol Specification](https://zips.z.cash/protocol/protocol.pdf)
- [Orchard Book](https://zcash.github.io/orchard/)
- [RISC Zero Documentation](https://dev.risczero.com/)

### Crates
- [`orchard`](https://docs.rs/orchard/latest/orchard/)
- [`zcash_primitives`](https://docs.rs/zcash_primitives/latest/zcash_primitives/)
- [`zcash_client_backend`](https://docs.rs/zcash_client_backend/latest/zcash_client_backend/)

### Related Docs
- [RISC Zero Setup](./risc0-setup-complete.md)
- [Host vs Guest Code](./host-vs-guest-code.md)
- [Implementation Plan](./implementation-plan.md)
- [Zcash Verification Flow](./zcash-verification-flow.md) (detailed technical)

---

*Last Updated: 2025-11-16*
*Status: Zcash integration pending - currently using placeholders*
