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

Khafi Gateway **separates** Zcash payment from business logic verification:
1. **Zcash payment** - User creates transaction with their wallet (spending key stays private)
2. **Payment verification** - Zcash Backend monitors blockchain, records nullifiers
3. **Business logic** - RISC Zero zkVM verifies custom compliance rules

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
│                  │           │  Server-side     │
│  User's wallet   │           │                  │
└──────────────────┘           └──────────────────┘
          │                               │
          │ Nullifier links payment       │
          │ to API request                │
          └───────────────┬───────────────┘
                          │
                          ▼
            ┌──────────────────────────┐
            │  Two Separate Systems:   │
            │  1. Zcash payment proof  │
            │     (blockchain native)  │
            │  2. Business logic proof │
            │     (RISC Zero)          │
            │  Linked via nullifier    │
            └──────────────────────────┘
```

### Why This Architecture?

**Critical Constraint:** RISC Zero zkVM runs on **servers only** (not browsers/mobile).

**User's Wallet:**
- Creates Zcash transaction locally
- **Spending key NEVER leaves user's device**
- Broadcasts transaction to Zcash network
- Transaction includes payment to Khafi's address

**Zcash Backend (Server):**
- Monitors Khafi's Zcash payment address
- Records received transactions and their nullifiers
- Stores nullifiers in database (Redis/PostgreSQL)

**RISC Zero zkVM (Server):**
- Verifies **business logic ONLY**
- Takes nullifier as public input (links to payment)
- Takes business data as private input
- **NO Zcash cryptography** - payment already verified!

**Benefits:**
- **Privacy:** Spending key never shared, never leaves user's device
- **Security:** Zero-knowledge for business data only
- **Practical:** Works with browser/mobile (no zkVM client-side)
- **Flexibility:** Any business rules via Logic Compiler
- **Composability:** Standard Zcash payment + custom business logic

### Data Flow

```
┌────────────────────────────────────────────────────────┐
│  1. User's Zcash Wallet (Local Device)                │
│     • User creates Zcash transaction                   │
│     • Spending key stays on device (NEVER shared!)     │
│     • Transaction pays Khafi's Zcash address           │
│     • User broadcasts transaction to Zcash network     │
│     • Nullifier is public output of transaction        │
└────────────────────────────────────────────────────────┘
                          │
                          ▼ Transaction confirms on blockchain
┌────────────────────────────────────────────────────────┐
│  2. Zcash Backend (Server - monitors blockchain)       │
│     • Watches Khafi's payment address                  │
│     • Detects incoming transaction                     │
│     • Records nullifier in database                    │
│     • Stores: nullifier, amount, timestamp, tx_id      │
└────────────────────────────────────────────────────────┘
                          │
                          ▼ User makes API request
┌────────────────────────────────────────────────────────┐
│  3. User's Client/SDK                                  │
│     Sends to Khafi Gateway:                            │
│     • Nullifier (from their Zcash transaction)         │
│     • Business private data (age, prescription, etc.)  │
│     • NO spending key - payment already happened!      │
└────────────────────────────────────────────────────────┘
                          │
                          ▼ Nullifier + Business Data
┌────────────────────────────────────────────────────────┐
│  4. Khafi Gateway (Server)                             │
│     • Check: Does nullifier exist in payments DB?      │
│       └─ NO → Reject (user didn't pay)                 │
│     • Check: Has nullifier been used before?           │
│       └─ YES → Reject (replay attack)                  │
│     • If checks pass → Run RISC Zero zkVM              │
└────────────────────────────────────────────────────────┘
                          │
                          ▼
┌────────────────────────────────────────────────────────┐
│  5. RISC Zero Guest Program (zkVM on Server)           │
│  ┌──────────────────────────────────────────────────┐  │
│  │  INPUTS:                                         │  │
│  │   • nullifier (PUBLIC - links to payment)        │  │
│  │   • business_data (PRIVATE - age, etc.)          │  │
│  │   • public_params (PUBLIC - min age, etc.)       │  │
│  └──────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────┐  │
│  │  execute_business_logic():                       │  │
│  │   • Custom validation rules (from Logic Compiler)│  │
│  │   • Age verification, signature checks, etc.     │  │
│  │   • NO Zcash cryptography - just business logic! │  │
│  └──────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────┐  │
│  │  env::commit(GuestOutputs):                      │  │
│  │   • nullifier (same as input)                    │  │
│  │   • compliance_result (bool)                     │  │
│  │   • metadata (optional attestations)             │  │
│  └──────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────┘
                          │
                          ▼ Receipt (proof + journal)
┌────────────────────────────────────────────────────────┐
│  6. zk-verification-service (Envoy ExtAuth)            │
│     • Verify RISC Zero proof cryptographically         │
│     • Mark nullifier as "used" in Redis                │
│     • Extract compliance_result from journal           │
│     • Allow/deny API request based on compliance       │
└────────────────────────────────────────────────────────┘
```

### Input/Output Schema

**GuestInputs** (to RISC Zero zkVM):
```rust
pub struct GuestInputs {
    // Payment identifier (PUBLIC - links API request to Zcash payment)
    pub nullifier: Nullifier,         // [u8; 32] from user's Zcash transaction

    // Business-specific inputs (varies per use case)
    pub business: BusinessInputs {
        pub private_data: Vec<u8>,    // PRIVATE - age, prescription, etc.
        pub public_params: Vec<u8>,   // PUBLIC - min_age, blacklists, etc.
    },
}

// NO ZCASH INPUTS! Payment verification happens before zkVM execution.
```

**GuestOutputs** (committed to journal):
```rust
pub struct GuestOutputs {
    pub nullifier: Nullifier,          // [u8; 32] - same as input, prevents replay
    pub compliance_result: bool,        // Did validation pass?
    pub metadata: Vec<u8>,             // Optional attestations
}
```

**Payment Database Schema** (Redis/PostgreSQL):
```rust
struct ReceivedPayment {
    nullifier: [u8; 32],         // Primary key
    amount: u64,                  // zatoshis received
    tx_id: String,               // Zcash transaction ID
    timestamp: DateTime,          // When received
    used: bool,                  // Has this nullifier been used for API access?
}
```

---

## Current Implementation

### Status: Architecture Defined ✅, Backend Pending ⏳

**What's Implemented:**
- ✅ RISC Zero zkVM guest program structure
- ✅ GuestInputs/GuestOutputs types
- ✅ Business logic verification placeholder
- ✅ Nullifier-based replay protection in zk-verification-service

**What's Pending:**
- ⏳ Zcash Backend service (blockchain monitoring)
- ⏳ Payment database (received nullifiers)
- ⏳ Nullifier lookup before zkVM execution

**Current code** (`crates/methods/guest/src/main.rs`):
```rust
fn main() {
    let inputs: GuestInputs = env::read();

    // NO ZCASH VERIFICATION IN ZKVM!
    // Payment verification happens BEFORE this code runs
    // via nullifier lookup in payment database

    // Guest program ONLY verifies business logic
    let compliance_result = execute_business_logic(&inputs.business);

    let outputs = GuestOutputs {
        nullifier: inputs.nullifier,  // Pass through for replay protection
        compliance_result,
        metadata: vec![],
    };

    env::commit(&outputs);
}
```

**What's Needed for Zcash Backend:**
1. ⏳ Zcash Backend service implementation
   - Monitor blockchain for payments to our address
   - Parse transactions and extract nullifiers
   - Store in payment database
2. ⏳ Payment database schema (Redis/PostgreSQL)
   - Store received payment nullifiers
   - Track usage (prevent replay)
3. ⏳ Zcash node connection
   - Connect to Zebra or zcashd node
   - Query blockchain state
   - Subscribe to new transactions
4. ⏳ Gateway payment verification
   - Check nullifier in database before running zkVM
   - Reject if not found (unpaid) or already used (replay)

**What Works:**
- ✅ RISC Zero zkVM integration complete
- ✅ Guest program structure for business logic only
- ✅ Proof generation and verification pipeline
- ✅ Nullifier-based replay protection in zk-verification-service
- ✅ Correct separation of concerns (payment vs business logic)

### Available Crates for Zcash Backend

Workspace dependencies (for zcash-backend service):
```toml
zcash_primitives = "0.26.1"   # Core types, transaction parsing
orchard = "0.11"               # Orchard protocol types
zcash_client_backend = "0.21"  # Blockchain client functions
```

**Key modules for Backend:**
- `zcash_client_backend::data_api` - Blockchain query API
- `orchard::note::Nullifier` - Nullifier type
- `zcash_primitives::transaction` - Transaction parsing
- `zcash_primitives::consensus` - Network parameters

**NOT needed in guest program** - All Zcash crypto stays in backend!

---

## Implementation Roadmap

### Phase 4: Zcash Backend Service (Future)

**Goal:** Monitor blockchain and verify payments before running zkVM.

**Implementation Steps:**

#### Step 1: Zcash Node Connection
**Timeline:** 1 week
**Effort:** Medium

**Tasks:**
1. Set up Zebra testnet node (or connect to public node)
2. Implement RPC client for blockchain queries
3. Query transaction history for our payment address
4. Parse transactions to extract nullifiers
5. Test connection and transaction parsing

**Dependencies:**
- Zebra node (recommended) or zcashd
- `zcash_client_backend` crate for RPC
- `zcash_primitives` for transaction parsing

#### Step 2: Payment Database
**Timeline:** 3 days
**Effort:** Low

**Tasks:**
1. Design payment database schema:
   ```sql
   CREATE TABLE received_payments (
       nullifier BYTEA PRIMARY KEY,
       amount BIGINT NOT NULL,
       tx_id TEXT NOT NULL,
       timestamp TIMESTAMP NOT NULL,
       used BOOLEAN DEFAULT FALSE
   );
   CREATE INDEX idx_used ON received_payments(used);
   ```
2. Implement database operations (insert, lookup, mark_used)
3. Add Redis caching layer for hot lookups
4. Test concurrent access and race conditions

**Technology:**
- PostgreSQL for persistent storage
- Redis for fast lookups

#### Step 3: Blockchain Monitor Service
**Timeline:** 1 week
**Effort:** Medium

**Tasks:**
1. Implement blockchain polling/subscription
2. Detect new transactions to our address
3. Extract nullifiers from confirmed transactions
4. Store in payment database
5. Handle reorganizations (blockchain reorgs)
6. Add metrics and logging

**Challenges:**
- Handling blockchain reorgs correctly
- Managing confirmation depth requirements
- Real-time vs batch processing trade-offs

#### Step 4: Gateway Integration
**Timeline:** 3 days
**Effort:** Low

**Tasks:**
1. Add payment verification step before zkVM execution:
   ```rust
   fn verify_payment(nullifier: &Nullifier) -> Result<()> {
       // Check nullifier exists in payments DB
       if !payment_db.exists(nullifier)? {
           return Err("Payment not found");
       }
       // Check not already used
       if payment_db.is_used(nullifier)? {
           return Err("Nullifier already used");
       }
       Ok(())
   }
   ```
2. Update zk-verification-service to call payment verification
3. Mark nullifier as used after successful zkVM execution
4. Add proper error handling and user feedback

### Recommended Path

**Phase 1 (MVP):** Simplified Backend
- Manual payment verification (admin UI to add nullifiers)
- Simple database lookup before zkVM
- Focus on Logic Compiler (core differentiator)
- **Timeline:** 3 days

**Phase 2 (Beta):** Automated Monitoring
- Connect to testnet Zebra node
- Automated transaction monitoring
- Real-time payment verification
- **Timeline:** 2 weeks

**Phase 3 (Production):** Full Features
- Mainnet support
- High availability (multiple nodes)
- Confirmation depth requirements
- Reorg handling
- **Timeline:** 1 month

---

## Code Examples

### Example 1: Zcash Backend - Transaction Monitoring

```rust
// In crates/zcash-backend/src/monitor.rs

use zcash_client_backend::data_api::WalletRead;
use zcash_primitives::transaction::Transaction;

pub struct BlockchainMonitor {
    client: ZcashClient,
    payment_db: PaymentDatabase,
    our_address: String,
}

impl BlockchainMonitor {
    pub async fn monitor_transactions(&self) -> Result<()> {
        // Subscribe to new blocks
        let mut block_stream = self.client.subscribe_blocks().await?;

        while let Some(block) = block_stream.next().await {
            // Process all transactions in block
            for tx in block.transactions {
                self.process_transaction(&tx).await?;
            }
        }
        Ok(())
    }

    async fn process_transaction(&self, tx: &Transaction) -> Result<()> {
        // Check if transaction pays our address
        for output in tx.sapling_outputs() {
            if output.address() == self.our_address {
                // Extract nullifier from output
                let nullifier = output.nullifier();

                // Store in payment database
                self.payment_db.insert(ReceivedPayment {
                    nullifier: nullifier.to_bytes(),
                    amount: output.value(),
                    tx_id: tx.txid().to_string(),
                    timestamp: Utc::now(),
                    used: false,
                }).await?;
            }
        }
        Ok(())
    }
}
```

### Example 2: Gateway - Payment Verification Before zkVM

```rust
// In crates/zk-verification-service/src/service.rs

async fn handle_request(&self, nullifier: &Nullifier, business_data: Vec<u8>)
    -> Result<Response>
{
    // STEP 1: Verify payment exists
    let payment = self.payment_db.get(nullifier).await
        .ok_or(Status::unauthenticated("Payment not found"))?;

    // STEP 2: Check not already used
    if payment.used {
        return Err(Status::permission_denied("Nullifier already used"));
    }

    // STEP 3: Run zkVM for business logic ONLY
    let guest_inputs = GuestInputs {
        nullifier: nullifier.clone(),
        business: BusinessInputs {
            private_data: business_data,
            public_params: vec![],  // e.g., min_age, blacklists
        },
    };

    let proof = self.prover.generate_proof(&guest_inputs).await?;
    let outputs: GuestOutputs = proof.verify_and_decode(&self.image_id)?;

    // STEP 4: Check business logic result
    if !outputs.compliance_result {
        return Err(Status::permission_denied("Business logic validation failed"));
    }

    // STEP 5: Mark nullifier as used
    self.payment_db.mark_used(nullifier).await?;

    // SUCCESS - grant API access
    Ok(Response::new(CheckResponse {
        status: StatusCode::Ok as i32,
        message: "Verified".to_string(),
    }))
}
```

### Example 3: zkVM Guest Program - Business Logic Only

```rust
// In methods/guest/src/main.rs

#![no_main]
risc0_zkvm::guest::entry!(main);

use khafi_common::{GuestInputs, GuestOutputs, Nullifier};

fn main() {
    // Read inputs
    let inputs: GuestInputs = env::read();

    // NO ZCASH VERIFICATION HERE!
    // Payment was already verified before this code runs

    // Execute business logic (generated by Logic Compiler)
    let compliance_result = execute_business_logic(&inputs.business);

    // Output results
    let outputs = GuestOutputs {
        nullifier: inputs.nullifier,  // Pass through for replay protection
        compliance_result,
        metadata: vec![],
    };

    env::commit(&outputs);
}

fn execute_business_logic(business: &BusinessInputs) -> bool {
    // Parse private data
    let age = parse_age(&business.private_data);

    // Parse public params
    let min_age = parse_min_age(&business.public_params);

    // Verify age requirement
    age >= min_age
}
```

### Example 4: User's Zcash Wallet Integration

```rust
// User's application code (NOT part of Khafi Gateway)

use zcash_client_backend::keys::UnifiedSpendingKey;

async fn make_api_request_with_payment(
    user_wallet: &ZcashWallet,
    khafi_gateway_url: &str,
    business_data: Vec<u8>,
) -> Result<()> {
    // STEP 1: User creates Zcash transaction (spending key stays local!)
    let tx = user_wallet.create_transaction(
        vec![Output {
            address: KHAFI_PAYMENT_ADDRESS,  // Khafi's Zcash address
            amount: API_CALL_PRICE,          // e.g., 0.001 ZEC
        }]
    )?;

    // STEP 2: Broadcast to Zcash network
    user_wallet.broadcast_transaction(&tx).await?;

    // STEP 3: Extract nullifier from transaction
    let nullifier = tx.nullifier();  // Public output

    // STEP 4: Wait for confirmation (optional)
    user_wallet.wait_for_confirmation(&tx.txid(), 3).await?;

    // STEP 5: Call Khafi API with nullifier + business data
    let response = reqwest::Client::new()
        .post(format!("{}/api/verify", khafi_gateway_url))
        .json(&json!({
            "nullifier": hex::encode(nullifier),
            "business_data": hex::encode(business_data),
        }))
        .send()
        .await?;

    // Gateway will:
    // 1. Check nullifier in payment database
    // 2. Run zkVM to verify business logic
    // 3. Mark nullifier as used
    // 4. Grant API access

    Ok(())
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
