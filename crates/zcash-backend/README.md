# Zcash Backend Service

Blockchain monitoring and payment verification service for the Khafi Gateway.

## Overview

The Zcash Backend monitors the Zcash blockchain for incoming payments and stores payment nullifiers in Redis. The Gateway queries this service to verify payments before granting API access.

### Architecture

```
User Wallet (client)
  ↓ Creates Zcash tx with spending key
  ↓ Broadcasts to blockchain
  ↓ Nullifier published (public)

Zcash Backend (server)
  ↓ Monitors blockchain
  ↓ Extracts nullifiers
  ↓ Stores in Redis database

Gateway (server)
  ↓ Checks nullifier in database (BEFORE zkVM)
  ↓ Runs zkVM for business logic (NO Zcash crypto)
  ↓ Marks nullifier as used
  ↓ Grants/denies API access
```

## Quick Start

### 1. Start Redis

```bash
cd /Users/ahmad/Code/khafi-gateway
docker compose up -d redis
```

### 2. Configure Environment

```bash
cp .env.example .env
# Edit .env if needed (defaults work for local development)
```

### 3. Run the Service

```bash
cargo run --bin zcash-backend
```

The service will:
- Start the API server on `http://localhost:8081`
- Begin monitoring the blockchain (mock mode by default)
- Store detected payments in Redis

## API Endpoints

### Health Check

```bash
curl http://localhost:8081/health
```

### Get Payment Status

```bash
curl http://localhost:8081/payment/{nullifier_hex}
```

Response:
```json
{
  "exists": true,
  "used": false,
  "amount": 10000000,
  "block_height": 100010,
  "tx_id": "mock_payment_tx_00018710"
}
```

### Get Statistics

```bash
curl http://localhost:8081/stats
```

Response:
```json
{
  "total_payments": 5,
  "unused_payments": 3,
  "total_amount_zec": 0.5
}
```

### Manually Insert Payment (Admin/Testing)

```bash
curl -X POST http://localhost:8081/admin/payment \
  -H "Content-Type: application/json" \
  -d '{
    "nullifier_hex": "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20",
    "amount": 10000000,
    "tx_id": "manual_test_tx",
    "block_height": 999999
  }'
```

## Configuration

Environment variables (set in `.env` or docker-compose.yml):

| Variable | Default | Description |
|----------|---------|-------------|
| `REDIS_URL` | `redis://localhost:6379` | Redis connection URL |
| `API_HOST` | `0.0.0.0` | API server host |
| `API_PORT` | `8081` | API server port |
| `POLLING_INTERVAL_SECS` | `60` | Blockchain polling interval |
| `MOCK_MODE` | `true` | Use mock Zcash node (for development) |
| `PAYMENT_ADDRESS` | `u1test_mock_address` | Khafi's Zcash payment address |
| `RUST_LOG` | `info,zcash_backend=debug` | Logging configuration |

## Mock Mode

In mock mode (default), the service uses a simulated Zcash blockchain:
- Generates mock blocks every 60 seconds
- Creates payments to our address every 10th block
- Payments have realistic structure with 32-byte nullifiers
- Perfect for development and testing without a real Zcash node

### Mock Payment Pattern

- **Block 100000**: Payment (10000000 zatoshis)
- **Block 100010**: Payment (10010000 zatoshis)
- **Block 100020**: Payment (10020000 zatoshis)
- etc.

## Development

### Run Tests

```bash
# Unit tests (no external dependencies)
cargo test -p zcash-backend

# Integration tests (requires Redis)
docker compose up -d redis
cargo test -p zcash-backend -- --ignored
```

### Build

```bash
cargo build -p zcash-backend --release
```

### Docker

```bash
# Build and run with docker compose
docker compose up zcash-backend

# Or build just the zcash-backend container
docker compose build zcash-backend
```

## Production Mode

To connect to a real Zcash node:

1. Set up a Zcash node (Zebra recommended):
   ```bash
   # Install Zebra
   # See: https://zebra.zfnd.org/
   ```

2. Configure environment:
   ```bash
   MOCK_MODE=false
   ZCASH_NODE_URL=http://localhost:8232
   ZCASH_NODE_USER=your_rpc_user
   ZCASH_NODE_PASSWORD=your_rpc_password
   PAYMENT_ADDRESS=u1... # Your actual Zcash unified address
   ```

3. Restart the service

**Note:** Production mode with real Zcash node integration is partially implemented. The mock mode is fully functional for development.

## Data Model

### Redis Storage

**Payment Records** (Hash per nullifier):
```
Key: payment:{nullifier_hex}
Fields:
  - amount: "10000000"
  - tx_id: "abc123..."
  - block_height: "12345"
  - timestamp: "2024-01-01T00:00:00Z"
  - used: "false"
  - used_at: ""
```

**Indexes** (Sets for queries):
```
Set: payments:all               → All nullifiers
Set: payments:unused            → Unused nullifiers only
Sorted Set: payments:by_height  → Nullifiers sorted by block height
```

### Persistence

Redis is configured with AOF (Append Only File) persistence:
- `appendonly yes` - Enable AOF
- `appendfsync everysec` - Fsync every second (good balance)

Data survives container restarts and system reboots.

## Integration with Gateway

The Gateway calls this service to verify payments:

```rust
// Gateway code (pseudocode)
async fn verify_payment(nullifier: Nullifier) -> Result<bool> {
    // 1. Check payment exists and is unused
    let response = reqwest::get(format!(
        "http://zcash-backend:8081/payment/{}",
        nullifier.to_hex()
    )).await?;

    let payment: PaymentStatusResponse = response.json().await?;

    if !payment.exists || payment.used {
        return Ok(false);
    }

    // 2. Run zkVM for business logic
    let proof_valid = run_zkvm(nullifier, business_data).await?;

    if proof_valid {
        // 3. Mark nullifier as used
        mark_nullifier_used(nullifier).await?;
        Ok(true)
    } else {
        Ok(false)
    }
}
```

## Monitoring

View logs:
```bash
docker compose logs -f zcash-backend
```

Check Redis data:
```bash
docker compose exec redis redis-cli

# List all payments
127.0.0.1:6379> SMEMBERS payments:all

# Get payment details
127.0.0.1:6379> HGETALL payment:{nullifier_hex}

# Count unused payments
127.0.0.1:6379> SCARD payments:unused
```

## Troubleshooting

### Service won't start

1. Check Redis is running:
   ```bash
   docker compose ps redis
   ```

2. Check logs:
   ```bash
   docker compose logs zcash-backend
   ```

3. Verify Redis connection:
   ```bash
   docker compose exec redis redis-cli ping
   ```

### No payments detected

1. Verify mock mode is enabled:
   ```bash
   echo $MOCK_MODE  # Should be "true"
   ```

2. Check polling interval isn't too long:
   ```bash
   echo $POLLING_INTERVAL_SECS  # Recommended: 60
   ```

3. View monitor logs:
   ```bash
   docker compose logs -f zcash-backend | grep "Processing block"
   ```

### Payment not found via API

1. Check nullifier format (must be 64-character hex):
   ```bash
   echo -n "your_nullifier" | wc -c  # Should be 64
   ```

2. List all payments in Redis:
   ```bash
   docker compose exec redis redis-cli SMEMBERS payments:all
   ```

## Future Enhancements

- [ ] Real Zcash node integration (Zebra/zcashd)
- [ ] Blockchain reorganization detection and rollback
- [ ] Confirmation depth requirements
- [ ] Payment expiration (time-based)
- [ ] Metrics and monitoring (Prometheus)
- [ ] Rate limiting on API endpoints
- [ ] Webhook notifications for new payments

## License

See workspace LICENSE file.
