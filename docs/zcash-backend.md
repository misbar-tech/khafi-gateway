# Zcash Backend Implementation

## Overview

The Zcash Backend is a service that monitors the Zcash blockchain for incoming payments and stores payment data in Redis. It serves as the bridge between Zcash transactions and the Khafi Gateway's payment verification system.

**Primary Functions:**
1. Connect to a lightwalletd instance (or use mock mode for development)
2. Monitor the blockchain for new blocks
3. Decrypt incoming notes using Full Viewing Keys (FVKs)
4. Extract customer nullifiers from transaction memos
5. Store payment data in Redis for the Gateway to query
6. Expose an API for payment status queries

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Zcash Backend                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   ┌─────────────┐    ┌──────────────┐    ┌─────────────────┐   │
│   │   Monitor   │───▶│ NoteDecrypt  │───▶│     Storage     │   │
│   │  (polling)  │    │  (Orchard)   │    │    (Redis)      │   │
│   └──────┬──────┘    └──────────────┘    └────────┬────────┘   │
│          │                                         │            │
│          ▼                                         ▼            │
│   ┌─────────────┐                         ┌─────────────────┐   │
│   │ Lightwalletd│                         │      API        │   │
│   │   Client    │                         │   (REST/HTTP)   │   │
│   └─────────────┘                         └─────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
          │                                          │
          ▼                                          ▼
   ┌─────────────┐                         ┌─────────────────┐
   │ lightwalletd│                         │  Khafi Gateway  │
   │   (gRPC)    │                         │  (queries API)  │
   └──────┬──────┘                         └─────────────────┘
          │
          ▼
   ┌─────────────┐
   │   zebrad    │
   │  (Zcash)    │
   └─────────────┘
```

---

## Components

### 1. Monitor (`src/monitor.rs`)

The main polling loop that:
- Fetches the current blockchain height
- Processes new blocks sequentially
- Delegates to either mock parser or real note decryptor
- Stores detected payments in Redis
- Updates the chain height tracker

**Configuration:** Polling interval is configurable via `POLLING_INTERVAL_SECS`.

### 2. Lightwalletd Client (`src/lightwalletd_client.rs`)

gRPC client for lightwalletd that provides:
- `get_block_count()` - Current chain height
- `get_block(height)` - Fetch block (converted to MockBlock format)
- `get_compact_block(height)` - Raw compact block for note decryption

**Proto files:** Uses `proto/service.proto` and `proto/compact_formats.proto` from the lightwalletd spec.

### 3. Note Decryptor (`src/note_decryption.rs`)

Decrypts incoming Orchard/Sapling notes using Full Viewing Keys:
- Parses compact blocks from lightwalletd
- Attempts trial decryption on each action
- Extracts customer nullifier from memo field
- Returns `ReceivedPayment` objects for storage

**Memo Format:** The customer's nullifier should be in one of these formats:
- Raw 32-byte nullifier in first 32 bytes of memo
- Hex-encoded 64-character string
- Prefixed format: `nullifier:<64-hex-chars>`

### 4. Parser (`src/parser.rs`)

Mock mode parser that extracts payments from `MockBlock` format. Used only in development/testing mode.

### 5. Mock Node (`src/mock_node.rs`)

Simulates a Zcash node for development:
- Starts at height 100,000
- Generates a payment every 10th block
- Produces deterministic nullifiers based on block height

### 6. Storage (`src/storage.rs`)

Redis operations for payment data:
- Insert/get payments by nullifier
- Mark payments as used
- Track block height
- Get payment statistics

### 7. API (`src/api.rs`)

REST API endpoints for external services:
- `GET /health` - Health check
- `GET /payment/{nullifier}` - Get payment status
- `POST /admin/payment` - Manually insert payment (testing)
- `GET /stats` - Payment statistics

---

## Configuration

Environment variables:

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `REDIS_URL` | No | `redis://localhost:6379` | Redis connection URL |
| `API_HOST` | No | `0.0.0.0` | API server bind address |
| `API_PORT` | No | `8081` | API server port |
| `POLLING_INTERVAL_SECS` | No | `60` | Blockchain polling interval |
| `MOCK_MODE` | No | `true` | Use mock node instead of lightwalletd |
| `LIGHTWALLETD_URL` | If `MOCK_MODE=false` | - | lightwalletd gRPC endpoint |
| `PAYMENT_ADDRESS` | No | `u1test_mock_address` | Zcash payment address to monitor |
| `ORCHARD_FVK` | If `MOCK_MODE=false` | - | 96-byte hex-encoded Orchard Full Viewing Key |
| `SAPLING_FVK` | If `MOCK_MODE=false` | - | Sapling Full Viewing Key (optional) |

### Example `.env` for development (mock mode):
```bash
REDIS_URL=redis://localhost:6379
MOCK_MODE=true
PAYMENT_ADDRESS=u1test_mock_address
POLLING_INTERVAL_SECS=10
```

### Example `.env` for testnet:
```bash
REDIS_URL=redis://localhost:6379
MOCK_MODE=false
LIGHTWALLETD_URL=http://localhost:9067
PAYMENT_ADDRESS=<your-unified-address>
ORCHARD_FVK=<96-byte-hex-encoded-fvk>
POLLING_INTERVAL_SECS=30
```

---

## API Reference

### GET /health

Health check endpoint.

**Response:**
- `200 OK` - Service healthy
- `503 Service Unavailable` - Redis connection failed

### GET /payment/{nullifier}

Get payment status by nullifier (64-character hex string).

**Response:**
```json
{
  "exists": true,
  "used": false,
  "amount": 10000000,
  "block_height": 100000,
  "tx_id": "abc123..."
}
```

### POST /admin/payment

Insert payment manually (for testing).

**Request:**
```json
{
  "nullifier_hex": "0102030405060708091011121314151617181920212223242526272829303132",
  "amount": 10000000,
  "tx_id": "test_tx_123",
  "block_height": 100000
}
```

**Response:**
- `201 Created` - Payment inserted
- `409 Conflict` - Payment already exists

### GET /stats

Get payment statistics.

**Response:**
```json
{
  "total_payments": 10,
  "unused_payments": 5,
  "total_amount_zec": 1.5
}
```

---

## Redis Data Model

### Keys

| Key Pattern | Type | Description |
|-------------|------|-------------|
| `payment:{nullifier_hex}` | Hash | Payment record |
| `payments:all` | Set | All known nullifiers |
| `payments:unused` | Set | Nullifiers not yet used |
| `payments:by_height` | Sorted Set | Nullifiers indexed by block height |
| `chain:block_height` | String | Current chain height |

### Payment Hash Fields

| Field | Type | Description |
|-------|------|-------------|
| `nullifier` | string | 64-char hex nullifier |
| `amount` | string | Amount in zatoshis |
| `tx_id` | string | Transaction ID |
| `block_height` | string | Block height |
| `timestamp` | string | RFC3339 timestamp |
| `used` | string | "true" or "false" |
| `used_at` | string | RFC3339 timestamp (if used) |

---

## Local Development Setup

### Option 1: Mock Mode (No Zcash infrastructure)

```bash
# Start Redis
docker compose up -d redis

# Run in mock mode
MOCK_MODE=true cargo run -p zcash-backend
```

### Option 2: Local Testnet (Full stack)

Use the provided docker-compose for zebrad + lightwalletd:

```bash
# Start Zcash testnet infrastructure
docker compose -f docker-compose.zcash.yaml up -d

# Wait for zebrad to sync past Sapling activation (block 280,000)
docker logs -f zebrad-testnet

# Once synced, run zcash-backend
MOCK_MODE=false \
LIGHTWALLETD_URL=http://localhost:9067 \
ORCHARD_FVK=<your-fvk> \
cargo run -p zcash-backend
```

---

## Implementation Gaps

### 1. Orchard Trial Decryption (CRITICAL)

**Status:** Stubbed out (always returns `None`)

**Location:** `src/note_decryption.rs:116-149`

**Issue:** The `try_decrypt_orchard_action` function currently only logs action data and returns `None`. Full implementation requires:

```rust
// Required implementation:
use orchard::note_encryption::try_compact_note_decryption;

fn try_decrypt_orchard_action(
    &self,
    action: &CompactOrchardAction,
    fvk: &OrchardFVK,
) -> Result<Option<DecryptedNote>> {
    // 1. Construct OrchardDomain from block data
    // 2. Derive Incoming Viewing Key (IVK) from FVK
    // 3. Call try_compact_note_decryption()
    // 4. If success, compute nullifier and extract memo
}
```

**Dependencies:**
- Need access to `orchard::note_encryption::try_compact_note_decryption`
- May need full ciphertext (not just compact 52 bytes) for memo extraction

**Workaround:** For testing, use the `/admin/payment` endpoint to manually insert payments.

### 2. Sapling Note Decryption (NOT IMPLEMENTED)

**Status:** Not implemented

**Location:** `src/note_decryption.rs:110` (TODO comment)

**Issue:** Sapling FVK parsing and note decryption not yet implemented. Lower priority since Orchard is the current pool.

### 3. Full Memo Retrieval

**Status:** Partial

**Issue:** Compact blocks only contain the first 52 bytes of ciphertext, which is enough for trial decryption but NOT for memo extraction. To get the full 512-byte memo:

Options:
1. Use `GetTransaction` RPC to fetch full transaction
2. Use `GetMempoolTx` for unconfirmed transactions
3. Subscribe to mempool stream for real-time detection

### 4. Confirmation Counting

**Status:** Data stored, not exposed

**Issue:** Block height is stored per payment and current chain height is tracked, but no API endpoint exposes confirmation count. The Gateway needs to verify sufficient confirmations before accepting payment.

**Required:** Add confirmation count to `/payment/{nullifier}` response:
```json
{
  "confirmations": 10,
  "confirmed": true
}
```

### 5. Mark Payment Used Endpoint (MISSING)

**Status:** Storage method exists, no API endpoint

**Location:** `src/storage.rs:189` - `mark_used()` method exists

**Issue:** No API endpoint to mark a payment as used. The Gateway needs to call this after granting API access.

**Required:** Add endpoint:
```
POST /payment/{nullifier}/use
```

### 6. Payment Amount Validation

**Status:** Not implemented

**Issue:** The backend stores whatever amount is detected but doesn't validate minimum payment amounts or price tiers.

**Consideration:** This may belong in the Gateway rather than the backend.

### 7. Reorg Handling

**Status:** Not implemented

**Issue:** If a blockchain reorganization occurs, payments may become invalid. Currently no mechanism to:
- Detect reorgs
- Remove orphaned payments
- Re-scan affected blocks

### 8. Rate Limiting / Authentication

**Status:** Not implemented

**Issue:** API endpoints have no authentication or rate limiting. The `/admin/payment` endpoint is particularly sensitive.

**Consideration:** May be handled by Envoy/Gateway layer instead.

---

## Testing

### Unit Tests

```bash
cargo test -p zcash-backend
```

### Integration Tests (require Redis)

```bash
# Start Redis first
docker compose up -d redis

# Run integration tests
cargo test -p zcash-backend -- --ignored
```

### Manual Testing

```bash
# Check health
curl http://localhost:8081/health

# Get payment (will return exists: false)
curl http://localhost:8081/payment/0102030405060708091011121314151617181920212223242526272829303132

# Insert test payment
curl -X POST http://localhost:8081/admin/payment \
  -H "Content-Type: application/json" \
  -d '{
    "nullifier_hex": "0102030405060708091011121314151617181920212223242526272829303132",
    "amount": 10000000,
    "tx_id": "test_tx",
    "block_height": 100000
  }'

# Get stats
curl http://localhost:8081/stats
```

---

## Future Enhancements

1. **WebSocket subscription** for real-time payment notifications
2. **Mempool monitoring** for faster payment detection
3. **Multi-address support** for watching multiple payment addresses
4. **Prometheus metrics** for monitoring and alerting
5. **Automatic key rotation** and key management integration
6. **Payment expiry** - automatically expire old unused payments

---

## References

- [Zcash Protocol Specification](https://zips.z.cash/protocol/protocol.pdf)
- [Lightwalletd Documentation](https://github.com/zcash/lightwalletd)
- [Orchard Crate](https://docs.rs/orchard)
- [Zcash Integration Guide](./zcash-integration.md)
