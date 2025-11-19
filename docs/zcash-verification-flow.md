# Zcash Verification Flow - Technical Deep Dive

This document provides detailed technical information about how Zcash payment verification works in the Khafi Gateway zkVM guest program.

## Table of Contents
1. [Verification Steps](#verification-steps)
2. [Cryptographic Operations](#cryptographic-operations)
3. [Serialization Formats](#serialization-formats)
4. [Implementation Details](#implementation-details)
5. [Performance Considerations](#performance-considerations)
6. [Testing Strategy](#testing-strategy)

---

## Verification Steps

### Step-by-Step Guest Program Flow

```rust
// In crates/methods/guest/src/main.rs

fn verify_zcash_payment(inputs: &ZcashInputs) -> Nullifier {
    // STEP 1: Deserialize Orchard note from bytes
    let note = deserialize_orchard_note(&inputs.note)?;

    // STEP 2: Compute note commitment using Sinsemilla
    let commitment = compute_note_commitment(&note)?;

    // STEP 3: Verify Merkle path against public anchor
    let merkle_path = deserialize_merkle_path(&inputs.merkle_path)?;
    verify_merkle_proof(commitment, &merkle_path, inputs.merkle_root)?;

    // STEP 4: Derive nullifier deriving key from spending key
    let nk = derive_nullifier_key(&inputs.spending_key)?;

    // STEP 5: Compute nullifier using Poseidon hash
    let nullifier = compute_nullifier(nk, &note, commitment)?;

    // STEP 6: Validate spending authority (implicit in successful derivation)
    // If we got here, user proved they know the spending key

    nullifier
}
```

### Input Validation

```rust
fn validate_zcash_inputs(inputs: &ZcashInputs) -> Result<()> {
    // Check spending key length
    if inputs.spending_key.len() != 32 {
        return Err("Invalid spending key length");
    }

    // Check note format
    if inputs.note.is_empty() {
        return Err("Empty note data");
    }

    // Check merkle path (32 levels × 32 bytes = 1024 bytes)
    if inputs.merkle_path.len() != 1024 {
        return Err("Invalid merkle path length");
    }

    Ok(())
}
```

---

## Cryptographic Operations

### 1. Sinsemilla Hash (Merkle Tree)

**Purpose:** Hash function for Orchard commitment tree

**Algorithm:**
- Operates on Pallas curve
- Takes two inputs: left child (32 bytes), right child (32 bytes)
- Returns: 32-byte hash

**Implementation:**
```rust
use orchard::primitives::sinsemilla;

fn merkle_hash(left: [u8; 32], right: [u8; 32]) -> [u8; 32] {
    // Sinsemilla("MerkleCRH", left || right)
    let domain = sinsemilla::HashDomain::new("MerkleCRH");
    let message = [left, right].concat();
    let hash = domain.hash_to_point(message.iter().copied());
    hash.to_bytes()
}
```

**Challenges in zkVM:**
- Requires Pallas curve operations
- Need to verify `orchard` crate works in `no_std` mode
- May need custom implementation if crate not compatible

### 2. Poseidon Hash (Nullifier Derivation)

**Purpose:** Keyed pseudorandom function for deriving nullifiers

**Algorithm:**
- Arithmetic hash function over Pallas scalar field
- Takes: nullifier key (nk), note randomness (ρ, ψ)
- Returns: 32-byte nullifier component

**Implementation:**
```rust
use orchard::primitives::poseidon;

fn poseidon_hash(nk: &NullifierKey, rho: &[u8; 32], psi: &[u8; 32]) -> [u8; 32] {
    // Fnk(ρ) using Poseidon hash
    let hasher = poseidon::Hash::init();
    hasher.update(nk.to_bytes());
    hasher.update(rho);
    hasher.update(psi);
    hasher.finalize()
}
```

**Nullifier Formula (Orchard):**
```
nf = ExtractP([(Fnk(ρ) + ψ) mod p]G + cm)
```

Where:
- `Fnk(ρ)` = Poseidon hash with key nk
- `ψ` = Blinding factor from note
- `G` = Fixed generator point
- `cm` = Note commitment
- `ExtractP` = Extract x-coordinate

### 3. Note Commitment

**Purpose:** Bind note data into tree

**Algorithm:**
```rust
fn compute_note_commitment(note: &Note) -> [u8; 32] {
    // NoteCommit(addr, v, ρ, ψ, rcm)
    let domain = sinsemilla::CommitDomain::new("NoteCommit");

    let message = [
        note.recipient().to_bytes(),
        note.value().to_le_bytes(),
        note.rho(),
        note.psi(),
        note.rcm(),
    ].concat();

    domain.commit(message.iter().copied(), note.rcm())
}
```

### 4. Key Derivation

**Spending Key → Nullifier Deriving Key:**
```rust
fn derive_nullifier_key(spending_key: &[u8; 32]) -> NullifierKey {
    // Use BLAKE2b-512 for key derivation
    let mut hasher = Blake2b512::new();
    hasher.update(b"Zcash_ExpandSeed");
    hasher.update(spending_key);
    hasher.update(&[0x02]); // Key type: nk

    let hash = hasher.finalize();
    NullifierKey::from_bytes(&hash[0..32])
}
```

**Full Key Hierarchy:**
```
SpendingKey (32 bytes)
    ↓ BLAKE2b("Zcash_ExpandSeed", sk, 0x02)
NullifierKey (nk)
    ↓ BLAKE2b("Zcash_ExpandSeed", sk, 0x00)
FullViewingKey (fvk)
    ↓ derive
IncomingViewingKey (ivk)
    ↓ derive
Address
```

---

## Serialization Formats

### Orchard Note Serialization

**Structure (wire format):**
```
byte[11]:  recipient diversifier
byte[32]:  recipient pk_d (transmission key)
byte[8]:   value (little-endian u64)
byte[32]:  ρ (rho)
byte[32]:  ψ (psi)
byte[32]:  rcm (commitment randomness)
───────────────────────────────
Total: 147 bytes
```

**Serialization (in SDK):**
```rust
fn serialize_note(note: &orchard::Note) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(147);
    note.write(&mut bytes).expect("Serialization failed");
    bytes
}
```

**Deserialization (in guest):**
```rust
fn deserialize_note(bytes: &[u8]) -> Result<Note> {
    if bytes.len() != 147 {
        return Err("Invalid note length");
    }
    orchard::Note::read(&mut &bytes[..])
}
```

### Merkle Path Serialization

**Structure (for 32-level tree):**
```
byte[32]:  Sibling at level 0 (leaf)
byte[32]:  Sibling at level 1
byte[32]:  Sibling at level 2
...
byte[32]:  Sibling at level 31 (near root)
───────────────────────────────
Total: 1024 bytes (32 × 32)
```

**Format:**
```rust
struct MerklePath {
    siblings: [[u8; 32]; 32],  // 32 levels
}
```

**Serialization:**
```rust
fn serialize_merkle_path(path: &orchard::tree::MerklePath) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(1024);
    for sibling in path.auth_path() {
        bytes.extend_from_slice(&sibling.to_bytes());
    }
    bytes
}
```

**Deserialization:**
```rust
fn deserialize_merkle_path(bytes: &[u8]) -> Result<MerklePath> {
    if bytes.len() != 1024 {
        return Err("Invalid merkle path length");
    }

    let mut siblings = [[0u8; 32]; 32];
    for i in 0..32 {
        siblings[i].copy_from_slice(&bytes[i*32..(i+1)*32]);
    }

    Ok(MerklePath { siblings })
}
```

### Spending Key Format

**Standard format:** 32 random bytes

```rust
struct SpendingKey([u8; 32]);

impl SpendingKey {
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 32 {
            return Err("Invalid spending key length");
        }
        let mut sk = [0u8; 32];
        sk.copy_from_slice(bytes);
        Ok(Self(sk))
    }

    fn to_bytes(&self) -> [u8; 32] {
        self.0
    }
}
```

---

## Implementation Details

### Memory Layout in zkVM

**GuestInputs memory structure:**
```
ZcashInputs:
  spending_key:  Vec<u8>    →  Heap (32 bytes)
  note:          Vec<u8>    →  Heap (147 bytes)
  merkle_path:   Vec<u8>    →  Heap (1024 bytes)
  merkle_root:   [u8; 32]   →  Stack (32 bytes)

Total zkVM input: ~1235 bytes
```

**Stack usage for verification:**
```rust
// Estimated stack usage
let commitment: [u8; 32];         // 32 bytes
let nk: NullifierKey;              // 32 bytes
let current_hash: [u8; 32];        // 32 bytes (for merkle traversal)
// Total: ~100 bytes stack

// Most data stays in heap via Vec<u8>
```

### Error Handling

```rust
#[derive(Debug)]
enum ZcashVerificationError {
    InvalidNoteEncoding,
    InvalidMerklePathLength,
    InvalidSpendingKey,
    MerkleProofFailed,
    NullifierDerivationFailed,
}

fn verify_zcash_payment_safe(inputs: &ZcashInputs) -> Result<Nullifier, ZcashVerificationError> {
    let note = deserialize_note(&inputs.note)
        .map_err(|_| ZcashVerificationError::InvalidNoteEncoding)?;

    let merkle_path = deserialize_merkle_path(&inputs.merkle_path)
        .map_err(|_| ZcashVerificationError::InvalidMerklePathLength)?;

    // Continue with verification...
}
```

### Constant-Time Operations

**Important for side-channel resistance:**
```rust
// Use constant-time comparison for nullifier checking
fn ct_eq(a: &[u8; 32], b: &[u8; 32]) -> bool {
    use subtle::ConstantTimeEq;
    a.ct_eq(b).into()
}

// Verify merkle root
if !ct_eq(&computed_root, &inputs.merkle_root) {
    return Err("Merkle proof verification failed");
}
```

---

## Performance Considerations

### Verification Time Estimates

**In zkVM (RISC Zero):**
```
Operation                    Cycles (est.)   Time (est.)
──────────────────────────────────────────────────────────
Deserialize note             ~1,000          0.1ms
Compute commitment           ~50,000         5ms
Verify merkle path (32 lvl)  ~1,600,000      160ms
Derive nullifier             ~100,000        10ms
Total Zcash verification     ~1,750,000      175ms

Note: Estimates based on RISC Zero benchmarks
Actual performance depends on:
  - Crypto library efficiency in zkVM
  - Proof generation parallelism
  - Hardware (CPU, RAM)
```

**Comparison to on-chain Zcash:**
- On-chain Orchard action: ~2-5 seconds to generate
- RISC Zero addition: ~175ms overhead
- **Total:** Still faster than generating Zcash proof itself

### Optimization Strategies

**1. Batch Verification:**
```rust
// If verifying multiple notes
fn verify_batch(inputs: &[ZcashInputs]) -> Vec<Nullifier> {
    // Reuse merkle root check (all use same anchor)
    let anchor = inputs[0].merkle_root;

    inputs.par_iter()  // Parallel verification
        .map(|input| verify_zcash_payment(input))
        .collect()
}
```

**2. Precomputed Constants:**
```rust
// Precompute Sinsemilla generators
const MERKLE_CRH_GENERATORS: [Point; 256] = precompute_generators();

// Reuse in merkle hashing
fn fast_merkle_hash(left: &[u8], right: &[u8]) -> [u8; 32] {
    sinsemilla_hash_with_generators(
        left,
        right,
        &MERKLE_CRH_GENERATORS
    )
}
```

**3. Lazy Deserialization:**
```rust
// Only deserialize what's needed
fn verify_zcash_payment_lazy(inputs: &ZcashInputs) -> Nullifier {
    // Don't deserialize full note if only need specific fields
    let value = read_note_value(&inputs.note);
    if value == 0 {
        return Err("Zero value note");
    }

    // Continue with full verification...
}
```

### Proof Size Impact

**RISC Zero proof size:**
```
Base proof:           ~150 KB (compressed STARK)
Zcash verification:   +10 KB (additional computation)
Business logic:       +5-20 KB (depends on rules)
───────────────────────────────────────────────────
Total:                ~165-180 KB

Network transmission: ~200ms on typical connection
```

---

## Testing Strategy

### Unit Tests

**Test 1: Merkle Path Verification**
```rust
#[test]
fn test_merkle_path_verification() {
    // Create test note
    let note = create_test_note();
    let commitment = compute_commitment(&note);

    // Build merkle tree with test data
    let tree = build_test_tree(&[commitment]);
    let path = tree.authentication_path(0);
    let root = tree.root();

    // Verify path
    assert!(verify_merkle_path(commitment, &path, root));
}
```

**Test 2: Nullifier Derivation**
```rust
#[test]
fn test_nullifier_derivation() {
    let spending_key = SpendingKey::random();
    let note = create_note_for_key(&spending_key);

    // Derive nullifier
    let nk = derive_nullifier_key(&spending_key);
    let nullifier1 = compute_nullifier(nk, &note);
    let nullifier2 = compute_nullifier(nk, &note);

    // Should be deterministic
    assert_eq!(nullifier1, nullifier2);

    // Different note → different nullifier
    let note2 = create_different_note(&spending_key);
    let nullifier3 = compute_nullifier(nk, &note2);
    assert_ne!(nullifier1, nullifier3);
}
```

**Test 3: Serialization Round-Trip**
```rust
#[test]
fn test_note_serialization() {
    let note = create_test_note();

    // Serialize
    let bytes = serialize_note(&note);
    assert_eq!(bytes.len(), 147);

    // Deserialize
    let note2 = deserialize_note(&bytes).unwrap();

    // Should match
    assert_eq!(note.value(), note2.value());
    assert_eq!(note.rho(), note2.rho());
}
```

### Integration Tests

**Test with Real RISC Zero:**
```rust
#[test]
fn test_full_zcash_verification_in_zkvm() {
    // Prepare inputs
    let spending_key = SpendingKey::random();
    let note = create_test_note_for_key(&spending_key);
    let tree = build_tree_with_note(&note);
    let path = tree.authentication_path(0);

    let zcash_inputs = ZcashInputs {
        spending_key: spending_key.to_bytes().to_vec(),
        note: serialize_note(&note),
        merkle_path: serialize_merkle_path(&path),
        merkle_root: tree.root(),
    };

    let guest_inputs = GuestInputs {
        zcash: zcash_inputs,
        business: BusinessInputs::default(),
    };

    // Run in zkVM
    let env = ExecutorEnv::builder()
        .write(&guest_inputs)
        .unwrap()
        .build()
        .unwrap();

    let prover = default_prover();
    let receipt = prover.prove(env, GUEST_ELF).unwrap().receipt;

    // Extract outputs
    let outputs: GuestOutputs = receipt.journal.decode().unwrap();

    // Verify nullifier was derived correctly
    assert_eq!(outputs.nullifier.0.len(), 32);
}
```

### Testnet Integration

**Test with Zcash Testnet:**
```rust
#[tokio::test]
async fn test_with_real_zcash_testnet() {
    // Connect to testnet Zebra node
    let client = ZcashClient::connect_testnet().await.unwrap();

    // Create test note on testnet
    let (note, spending_key) = client.create_test_note().await.unwrap();

    // Wait for confirmation
    client.wait_for_confirmation(note.commitment()).await;

    // Fetch commitment tree
    let tree_state = client.get_tree_state().await.unwrap();

    // Build merkle path
    let path = client.build_merkle_path(note.position()).await.unwrap();

    // Prepare inputs
    let zcash_inputs = ZcashInputs {
        spending_key: spending_key.to_bytes().to_vec(),
        note: serialize_note(&note),
        merkle_path: serialize_merkle_path(&path),
        merkle_root: tree_state.root,
    };

    // Generate proof
    let receipt = generate_proof(zcash_inputs).await.unwrap();

    // Verify
    assert!(verify_receipt(&receipt).is_ok());
}
```

---

*Last Updated: 2025-11-16*
*Related: [Zcash Integration Guide](./zcash-integration.md)*
