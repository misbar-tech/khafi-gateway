# Zcash Backend Service - Technical Deep Dive

This document provides detailed technical information about the Zcash Backend service that monitors blockchain payments and verifies them before zkVM execution.

**Important:** The zkVM guest program does NOT perform any Zcash cryptography. All Zcash payment verification happens in the Backend service BEFORE the zkVM runs.

## Table of Contents
1. [Architecture Overview](#architecture-overview)
2. [Blockchain Monitoring](#blockchain-monitoring)
3. [Transaction Parsing](#transaction-parsing)
4. [Payment Database](#payment-database)
5. [Gateway Integration](#gateway-integration)
6. [Performance Considerations](#performance-considerations)
7. [Testing Strategy](#testing-strategy)

---

## Architecture Overview

### System Components

```
┌────────────────────────────────────────────────────────┐
│  User's Zcash Wallet (External)                       │
│  • Creates transaction                                 │
│  • Broadcasts to Zcash network                         │
│  • Spending key NEVER leaves wallet                    │
└────────────────────────────────────────────────────────┘
                          │
                          ▼ Transaction confirmed
┌────────────────────────────────────────────────────────┐
│  Zcash Blockchain                                      │
│  • Confirms transaction                                 │
│  • Publishes nullifier (public output)                 │
│  • Payment received at Khafi's address                 │
└────────────────────────────────────────────────────────┘
                          │
                          ▼ Backend monitors
┌────────────────────────────────────────────────────────┐
│  Zcash Backend Service (Server)                        │
│  ┌──────────────────────────────────────────────────┐  │
│  │  1. Blockchain Monitor                           │  │
│  │     • Connects to Zebra/zcashd node              │  │
│  │     • Subscribes to new blocks                    │  │
│  │     • Filters transactions to our address         │  │
│  └──────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────┐  │
│  │  2. Transaction Parser                           │  │
│  │     • Extracts nullifiers from outputs           │  │
│  │     • Parses amounts, timestamps                  │  │
│  │     • Validates transaction structure             │  │
│  └──────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────┐  │
│  │  3. Payment Database                             │  │
│  │     • Stores received payment nullifiers          │  │
│  │     • Tracks usage (used/unused)                  │  │
│  │     • Provides lookup API                         │  │
│  └──────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────┘
                          │
                          ▼ User makes API request
┌────────────────────────────────────────────────────────┐
│  Khafi Gateway                                         │
│  1. Receive request with nullifier + business data     │
│  2. Check nullifier in payment database                │
│  3. If valid → Run zkVM for business logic             │
│  4. Mark nullifier as used                             │
│  5. Grant/deny API access                              │
└────────────────────────────────────────────────────────┘
```

### Separation of Concerns

**Zcash Backend Responsibilities:**
- ✅ Monitor blockchain for payments
- ✅ Parse Zcash transactions
- ✅ Extract nullifiers (public outputs)
- ✅ Store in payment database
- ✅ Provide lookup API for gateway

**zkVM Responsibilities:**
- ✅ Verify business logic ONLY
- ✅ No Zcash cryptography
- ✅ Input: nullifier (public) + business data (private)
- ✅ Output: compliance result

**Why This Architecture:**
- Spending keys never leave user's wallet
- RISC Zero runs on servers (can't run client-side)
- Clear separation: payment verification vs business logic
- Practical for browser/mobile clients

---

## Blockchain Monitoring

### Connecting to Zcash Node

The Backend service connects to a Zcash node (Zebra or zcashd) to monitor blockchain state.

**Connection Setup:**
```rust
// In crates/zcash-backend/src/node.rs

use zcash_client_backend::data_api::chain::ChainState;
use zcash_primitives::consensus::Network;

pub struct ZcashNode {
    rpc_client: jsonrpc::Client,
    network: Network,
}

impl ZcashNode {
    pub async fn connect(rpc_url: &str, network: Network) -> Result<Self> {
        let rpc_client = jsonrpc::simple_http::SimpleHttpTransport::builder()
            .url(rpc_url)?
            .build();

        Ok(Self {
            rpc_client: jsonrpc::Client::with_transport(rpc_client),
            network,
        })
    }

    pub async fn get_latest_block_height(&self) -> Result<u32> {
        let info: BlockchainInfo = self.rpc_client
            .call("getblockchaininfo", &[])
            .await?;
        Ok(info.blocks)
    }

    pub async fn get_block(&self, height: u32) -> Result<Block> {
        // Get block hash for height
        let hash: String = self.rpc_client
            .call("getblockhash", &[height.into()])
            .await?;

        // Get full block
        let block: Block = self.rpc_client
            .call("getblock", &[hash.into(), 2.into()])  // verbosity = 2 (full tx data)
            .await?;

        Ok(block)
    }
}
```

### Monitoring Loop

**Continuous Block Polling:**
```rust
// In crates/zcash-backend/src/monitor.rs

pub struct BlockchainMonitor {
    node: ZcashNode,
    payment_db: PaymentDatabase,
    our_address: String,
    last_processed_height: u32,
}

impl BlockchainMonitor {
    pub async fn start_monitoring(&mut self) -> Result<()> {
        loop {
            // Get latest block height
            let current_height = self.node.get_latest_block_height().await?;

            // Process any new blocks
            while self.last_processed_height < current_height {
                self.last_processed_height += 1;

                let block = self.node.get_block(self.last_processed_height).await?;
                self.process_block(&block).await?;

                tracing::info!("Processed block {}", self.last_processed_height);
            }

            // Wait before next poll
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }

    async fn process_block(&self, block: &Block) -> Result<()> {
        // Process all transactions in block
        for tx in &block.tx {
            self.process_transaction(tx, block.height, block.time).await?;
        }
        Ok(())
    }
}
```

---

## Transaction Parsing

### Extracting Payments from Transactions

**Parse Orchard Actions:**
```rust
// In crates/zcash-backend/src/parser.rs

use zcash_primitives::transaction::Transaction;
use orchard::note::Nullifier;

impl BlockchainMonitor {
    async fn process_transaction(
        &self,
        tx: &Transaction,
        block_height: u32,
        block_time: u32,
    ) -> Result<()> {
        // Parse Orchard actions (shielded pool)
        if let Some(orchard_bundle) = tx.orchard_bundle() {
            for action in orchard_bundle.actions() {
                // Check if output is to our address
                // (In practice, need viewing key to detect)
                if self.is_payment_to_us(action)? {
                    let payment = ReceivedPayment {
                        nullifier: action.nullifier().to_bytes(),
                        amount: action.value(),  // Requires decryption
                        tx_id: tx.txid().to_string(),
                        block_height,
                        timestamp: block_time,
                        used: false,
                    };

                    self.payment_db.insert(payment).await?;
                    tracing::info!("Recorded payment: {} ZEC", payment.amount);
                }
            }
        }

        Ok(())
    }

    fn is_payment_to_us(&self, action: &Action) -> Result<bool> {
        // Decrypt output note using our incoming viewing key
        // Check if decryption succeeds and address matches
        // This requires our IVK (incoming viewing key)

        // For now, simplified:
        // In production, use orchard::note::Note::decrypt()
        Ok(false)  // TODO: Implement with viewing key
    }
}
```

### Nullifier Extraction

Nullifiers are **public outputs** of Zcash transactions, so no decryption needed:

```rust
fn extract_nullifier(action: &orchard::Action) -> [u8; 32] {
    // Nullifier is publicly visible in transaction
    action.nullifier().to_bytes()
}
```

**Important:** While nullifiers are public, they don't reveal which note was spent or who spent it (due to zero-knowledge proofs).

---

## Payment Database

### Schema Design

**PostgreSQL Schema:**
```sql
CREATE TABLE received_payments (
    nullifier BYTEA PRIMARY KEY,           -- 32 bytes, unique identifier
    amount BIGINT NOT NULL,                 -- zatoshis (1 ZEC = 10^8 zatoshis)
    tx_id TEXT NOT NULL,                    -- Zcash transaction ID
    block_height INTEGER NOT NULL,          -- Block number
    timestamp INTEGER NOT NULL,             -- Unix timestamp
    used BOOLEAN DEFAULT FALSE,             -- Has this payment been used for API access?
    used_at TIMESTAMP,                      -- When was it used?
    created_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_used ON received_payments(used);
CREATE INDEX idx_block_height ON received_payments(block_height);
CREATE INDEX idx_timestamp ON received_payments(timestamp);
```

### Database Operations

**Insert New Payment:**
```rust
// In crates/zcash-backend/src/database.rs

impl PaymentDatabase {
    pub async fn insert(&self, payment: ReceivedPayment) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO received_payments
            (nullifier, amount, tx_id, block_height, timestamp, used)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (nullifier) DO NOTHING
            "#,
            &payment.nullifier[..],
            payment.amount as i64,
            payment.tx_id,
            payment.block_height as i32,
            payment.timestamp as i32,
            payment.used
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get(&self, nullifier: &[u8; 32]) -> Result<Option<ReceivedPayment>> {
        let row = sqlx::query_as!(
            ReceivedPayment,
            r#"
            SELECT nullifier, amount, tx_id, block_height, timestamp, used, used_at
            FROM received_payments
            WHERE nullifier = $1
            "#,
            &nullifier[..]
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    pub async fn mark_used(&self, nullifier: &[u8; 32]) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE received_payments
            SET used = TRUE, used_at = NOW()
            WHERE nullifier = $1
            "#,
            &nullifier[..]
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
```

### Caching Layer

**Redis for Fast Lookups:**
```rust
impl PaymentDatabase {
    pub async fn get_cached(&self, nullifier: &[u8; 32]) -> Result<Option<ReceivedPayment>> {
        let key = format!("payment:{}", hex::encode(nullifier));

        // Try Redis first
        if let Some(cached) = self.redis.get::<_, String>(&key).await.ok() {
            if let Ok(payment) = serde_json::from_str(&cached) {
                return Ok(Some(payment));
            }
        }

        // Fall back to PostgreSQL
        if let Some(payment) = self.get(nullifier).await? {
            // Cache for 1 hour
            let json = serde_json::to_string(&payment)?;
            self.redis.set_ex(&key, json, 3600).await?;
            return Ok(Some(payment));
        }

        Ok(None)
    }
}
```

---

## Gateway Integration

### Payment Verification Before zkVM

**In zk-verification-service:**
```rust
// In crates/zk-verification-service/src/service.rs

async fn verify_and_execute(
    &self,
    nullifier: &Nullifier,
    business_data: Vec<u8>,
) -> Result<Response, Status> {
    // STEP 1: Check payment exists
    let payment = self.payment_db.get_cached(&nullifier.0).await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| Status::unauthenticated("Payment not found"))?;

    // STEP 2: Check not already used
    if payment.used {
        return Err(Status::permission_denied("Payment already used"));
    }

    // STEP 3: Check payment amount (optional)
    const MIN_PAYMENT: i64 = 1_000_000;  // 0.01 ZEC
    if payment.amount < MIN_PAYMENT {
        return Err(Status::permission_denied("Insufficient payment"));
    }

    // STEP 4: Run zkVM for business logic
    let guest_inputs = GuestInputs {
        nullifier: nullifier.clone(),
        business: BusinessInputs {
            private_data: business_data,
            public_params: vec![],
        },
    };

    let proof = self.prover.generate_proof(&guest_inputs).await
        .map_err(|e| Status::internal(format!("Proof generation failed: {}", e)))?;

    let outputs: GuestOutputs = proof.verify_and_decode(&self.image_id)
        .map_err(|e| Status::permission_denied(format!("Proof verification failed: {}", e)))?;

    // STEP 5: Check business logic result
    if !outputs.compliance_result {
        return Err(Status::permission_denied("Business logic validation failed"));
    }

    // STEP 6: Mark payment as used
    self.payment_db.mark_used(&nullifier.0).await
        .map_err(|e| Status::internal(format!("Failed to mark payment used: {}", e)))?;

    // SUCCESS
    Ok(Response::new(CheckResponse {
        status: StatusCode::Ok as i32,
        message: "Verified".to_string(),
    }))
}
```

### API Endpoints

**Zcash Backend REST API:**
```rust
// In crates/zcash-backend/src/api.rs

#[get("/payment/{nullifier}")]
async fn check_payment(
    nullifier: Path<String>,
    db: Data<PaymentDatabase>,
) -> Result<Json<PaymentResponse>> {
    let nullifier_bytes = hex::decode(&nullifier.into_inner())?;
    let mut nullifier = [0u8; 32];
    nullifier.copy_from_slice(&nullifier_bytes);

    let payment = db.get(&nullifier).await?;

    Ok(Json(PaymentResponse {
        exists: payment.is_some(),
        used: payment.map(|p| p.used).unwrap_or(false),
        amount: payment.map(|p| p.amount),
    }))
}
```

---

## Implementation Details

### Handling Blockchain Reorganizations

**Reorg Detection:**
```rust
impl BlockchainMonitor {
    async fn handle_potential_reorg(&mut self, new_height: u32) -> Result<()> {
        // If chain height decreased, reorg likely happened
        if new_height < self.last_processed_height {
            tracing::warn!("Potential reorg detected: height dropped from {} to {}",
                self.last_processed_height, new_height);

            // Roll back database to last safe height
            let safe_height = new_height.saturating_sub(10);  // 10 block safety margin
            self.rollback_to_height(safe_height).await?;
            self.last_processed_height = safe_height;
        }

        Ok(())
    }

    async fn rollback_to_height(&self, height: u32) -> Result<()> {
        // Delete payments from blocks after this height
        sqlx::query!(
            "DELETE FROM received_payments WHERE block_height > $1",
            height as i32
        )
        .execute(&self.db.pool)
        .await?;

        Ok(())
    }
}
```

### Confirmation Depth

**Require Confirmations:**
```rust
pub async fn get_confirmed_payment(
    &self,
    nullifier: &[u8; 32],
    min_confirmations: u32,
) -> Result<Option<ReceivedPayment>> {
    let current_height = self.node.get_latest_block_height().await?;

    if let Some(payment) = self.get(nullifier).await? {
        let confirmations = current_height.saturating_sub(payment.block_height);

        if confirmations >= min_confirmations {
            return Ok(Some(payment));
        }
    }

    Ok(None)
}
```

---

## Serialization Formats

### Payment Record Serialization

**JSON API Format:**
```json
{
  "nullifier": "0x1234...",
  "amount": 10000000,
  "tx_id": "abc123...",
  "block_height": 2500000,
  "timestamp": 1700000000,
  "used": false
}
```

**Binary Format (for database):**
```rust
struct ReceivedPayment {
    nullifier: [u8; 32],    // 32 bytes
    amount: i64,            // 8 bytes
    tx_id: String,          // Variable
    block_height: u32,      // 4 bytes
    timestamp: u32,         // 4 bytes
    used: bool,             // 1 byte
}
```

---

## Performance Considerations

### Blockchain Monitoring Performance

**Block Processing:**
- Zcash blocks: ~75 seconds average block time
- Transactions per block: Varies (average ~10-100)
- Monitor polling interval: 60 seconds (configurable)

**Database Performance:**
- PostgreSQL insert: ~1-5ms per payment
- Redis cached lookup: <1ms
- PostgreSQL lookup (cache miss): ~5-10ms

**Scalability:**
- Single instance: ~1000 payments/minute
- With caching: ~10,000 lookups/second
- Database can scale horizontally (read replicas)

### Gateway Integration Performance

**Payment Verification Overhead:**
```
Total Request Processing Time:
├─ Payment DB lookup: 1-10ms (cached/uncached)
├─ zkVM proof generation: ~500ms-2s (depends on business logic)
├─ Proof verification: ~50ms
├─ Mark nullifier used: 5ms
└─ Total: ~600ms-2.5s
```

**Optimization Strategies:**
1. **Redis caching** - Reduce DB latency to <1ms
2. **Connection pooling** - Reuse DB connections
3. **Batch processing** - Process multiple blocks in parallel
4. **Confirmation depth** - Trade security for speed (fewer confirmations = faster)

---

## Testing Strategy

### Unit Tests

**Test 1: Payment Database Operations**
```rust
#[tokio::test]
async fn test_payment_insert_and_lookup() {
    let db = PaymentDatabase::new_test().await;

    let payment = ReceivedPayment {
        nullifier: [1u8; 32],
        amount: 10_000_000,
        tx_id: "abc123".to_string(),
        block_height: 100,
        timestamp: 1700000000,
        used: false,
    };

    db.insert(&payment).await.unwrap();

    let retrieved = db.get(&payment.nullifier).await.unwrap();
    assert_eq!(retrieved.unwrap().amount, 10_000_000);
}
```

**Test 2: Nullifier Replay Prevention**
```rust
#[tokio::test]
async fn test_mark_used_prevents_reuse() {
    let db = PaymentDatabase::new_test().await;
    let nullifier = [2u8; 32];

    let payment = create_test_payment(nullifier);
    db.insert(&payment).await.unwrap();

    // Mark as used
    db.mark_used(&nullifier).await.unwrap();

    // Check it's marked
    let retrieved = db.get(&nullifier).await.unwrap().unwrap();
    assert!(retrieved.used);
}
```

### Integration Tests

**Test with Testnet:**
```rust
#[tokio::test]
#[ignore]  // Requires testnet connection
async fn test_monitor_testnet_blocks() {
    let node = ZcashNode::connect("http://testnet-node:8232", Network::TestNetwork)
        .await
        .unwrap();

    let height = node.get_latest_block_height().await.unwrap();
    let block = node.get_block(height).await.unwrap();

    assert!(block.transactions.len() > 0);
}
```

### End-to-End Test

**Full Payment Flow:**
```rust
#[tokio::test]
async fn test_full_payment_verification_flow() {
    // Setup
    let backend = ZcashBackend::new_test().await;
    let gateway = Gateway::new_test(backend.clone()).await;

    // Simulate payment received
    let nullifier = [3u8; 32];
    backend.record_payment(ReceivedPayment {
        nullifier,
        amount: 10_000_000,
        ...
    }).await.unwrap();

    // User makes API request
    let result = gateway.verify_and_execute(
        &Nullifier::new(nullifier),
        vec![/* business data */],
    ).await;

    assert!(result.is_ok());

    // Check nullifier marked as used
    let payment = backend.get_payment(&nullifier).await.unwrap();
    assert!(payment.used);

    // Try replay attack
    let replay_result = gateway.verify_and_execute(
        &Nullifier::new(nullifier),
        vec![/* business data */],
    ).await;

    assert!(replay_result.is_err());  // Should be rejected
}
```

---

*Last Updated: 2025-11-19*
*Related: [Zcash Integration Guide](./zcash-integration.md)*
