# Khafi Gateway - Deployment Architecture

This document describes the updated multi-tenant SaaS architecture with server-side proof generation.

## Architecture Overview

The Khafi Gateway has been transformed from an "SDK download" model to a "Deploy to Gateway" model where:

1. **Users deploy DSL configurations** through the web UI
2. **Guest programs are built and hosted** server-side
3. **Proofs are generated on-demand** via API calls
4. **Multiple customers** share the same infrastructure (multi-tenant)

## System Components

### 1. Image ID Registry Service (Port 8083)
**Purpose:** Central registry mapping customer IDs to their deployed guest programs

**Key Features:**
- Maps `customer_id → (image_id, guest_program_path)`
- Redis-backed storage for fast lookups
- Bidirectional lookups: by customer ID or by Image ID

**API Endpoints:**
- `POST /api/deployments` - Register new deployment
- `GET /api/deployments/{customer_id}` - Get deployment by customer
- `GET /api/deployments/by-image-id/{image_id}` - Get deployment by Image ID
- `PUT /api/deployments/{customer_id}` - Update deployment
- `DELETE /api/deployments/{customer_id}` - Remove deployment

**Storage Schema:**
```
deployment:{customer_id} → {
  customer_id, image_id, guest_program_path, created_at, metadata
}
image_id:{image_id} → customer_id
deployments:all → Set of all customer IDs
```

### 2. Proof Generation Service (Port 8084)
**Purpose:** Hosts customer guest programs and generates RISC Zero proofs on their behalf

**Key Features:**
- Dynamically loads guest programs from registry
- In-memory caching of loaded programs
- Generates proofs using RISC Zero zkVM
- Returns hex-encoded proofs with public outputs

**API Endpoints:**
- `POST /api/generate-proof` - Generate proof for customer inputs
- `POST /api/load-program` - Preload guest program
- `GET /api/status` - Service health and loaded program count

**Request Format:**
```json
{
  "customer_id": "customer-123",
  "private_inputs": { ... },
  "public_params": { ... }
}
```

**Response Format:**
```json
{
  "success": true,
  "proof": "hex-encoded-proof",
  "image_id": "abc123...",
  "outputs": { "compliance_result": true }
}
```

### 3. Logic Compiler API (Port 8082)
**Purpose:** Validates, compiles, and deploys DSL configurations

**New Endpoint: POST /api/deploy**

**Deployment Workflow:**
1. Validate DSL
2. Generate guest program source code
3. Build with `cargo risczero build`
4. Compute Image ID from ELF binary
5. Register in Image ID Registry
6. Return deployment info

**Request:**
```json
{
  "dsl": { ... },
  "customer_id": "customer-123"
}
```

**Response:**
```json
{
  "success": true,
  "customer_id": "customer-123",
  "image_id": "abc123...",
  "api_endpoint": "http://localhost:8080/api/prove"
}
```

### 4. Envoy Proxy (Port 8080)
**Purpose:** API Gateway with ExtAuth, payment verification, and rate limiting

**Configuration:** `config/envoy.yaml`

**Key Routes:**
- `/api/validate` → Logic Compiler (no auth)
- `/api/compile` → Logic Compiler (no auth)
- `/api/deploy` → Logic Compiler (no auth)
- `/api/prove` → Proof Generation Service (protected by ExtAuth)
- `/registry/*` → Image ID Registry (internal)

**Security Layers:**
1. **ExtAuth Filter:** Validates ZK proofs via ZK Verification Service
2. **Payment Verification:** Checks Zcash payment via Zcash Backend
3. **Rate Limiting:** Per-customer quotas

### 5. Frontend UI (Port 3000)
**Purpose:** User interface for DSL design and deployment

**New Features:**
- "Deploy to Gateway" button (replaces SDK download)
- Shows deployment result with:
  - Customer ID
  - Image ID
  - API endpoint URL
- Legacy SDK download still available

## Deployment Flow

### Step 1: User Creates DSL
User designs validation rules using the web UI (Template Gallery → DSL Editor)

### Step 2: Deploy to Gateway
1. User clicks "Deploy to Gateway"
2. Frontend generates customer ID: `customer-{use_case}-{timestamp}`
3. POST request to `/api/deploy` with DSL + customer ID
4. Logic Compiler API:
   - Generates guest program code
   - Runs `cargo risczero build`
   - Computes Image ID
   - Registers in Image ID Registry
5. Returns API endpoint and credentials

### Step 3: Use API
Customer calls the API endpoint:
```bash
curl -X POST http://localhost:8080/api/prove \
  -H "Content-Type: application/json" \
  -d '{
    "customer_id": "customer-123",
    "private_inputs": { ... },
    "public_params": { ... }
  }'
```

### Step 4: Proof Generation
1. Request hits Envoy
2. ExtAuth validates ZK proof (if required)
3. Payment verified via Zcash Backend
4. Forwarded to Proof Generation Service
5. Service looks up guest program from registry
6. Generates proof using RISC Zero
7. Returns proof to customer

## Service Ports

| Service                    | Port  | Purpose                          |
|----------------------------|-------|----------------------------------|
| Envoy Proxy                | 8080  | Main API Gateway                 |
| Envoy Admin                | 9901  | Admin interface                  |
| Zcash Backend              | 8081  | Payment verification             |
| Logic Compiler API         | 8082  | DSL compilation & deployment     |
| Image ID Registry          | 8083  | Deployment registry              |
| Proof Generation Service   | 8084  | Proof generation                 |
| ZK Verification Service    | 50051 | gRPC ExtAuth (ZK verification)   |
| Redis                      | 6379  | Storage backend                  |
| Frontend                   | 3000  | Web UI                           |

## Running the System

### Development (Individual Services)

**Start Redis:**
```bash
docker run -p 6379:6379 redis:8.4.0-alpine3.22 \
  redis-server --appendonly yes
```

**Start Image ID Registry:**
```bash
REDIS_URL=redis://127.0.0.1:6379 \
  cargo run -p image-id-registry
```

**Start Proof Generation Service:**
```bash
REGISTRY_URL=http://127.0.0.1:8083 \
  cargo run -p proof-generation-service
```

**Start Logic Compiler API:**
```bash
REGISTRY_URL=http://127.0.0.1:8083 \
GATEWAY_URL=http://localhost:8080 \
TEMPLATES_DIR=./docs/examples \
  cargo run -p logic-compiler-api
```

**Start Frontend:**
```bash
cd frontend
VITE_API_URL=http://localhost:8082 npm run dev
```

### Production (Docker Compose)

```bash
docker-compose up -d
```

This starts all services with proper networking and dependencies.

## Environment Variables

### Logic Compiler API
- `REGISTRY_URL` - Image ID Registry URL (default: http://127.0.0.1:8083)
- `GATEWAY_URL` - Gateway URL for API endpoint (default: http://localhost:8080)
- `TEMPLATES_DIR` - Path to DSL templates
- `SDK_OUTPUT_DIR` - Path for SDK/deployment artifacts

### Image ID Registry
- `REDIS_URL` - Redis connection string
- `REGISTRY_HOST` - Bind address
- `REGISTRY_PORT` - Port number

### Proof Generation Service
- `REGISTRY_URL` - Image ID Registry URL
- `PROVER_HOST` - Bind address
- `PROVER_PORT` - Port number

## Multi-Tenancy

The system supports multiple customers on the same infrastructure:

1. **Isolation:** Each customer has a unique `customer_id`
2. **Guest Programs:** Stored separately in the Image ID Registry
3. **Proof Generation:** Service loads the correct program per customer
4. **Rate Limiting:** Envoy enforces per-customer quotas
5. **Payment:** Each customer's payments tracked separately in Zcash Backend

## Security Considerations

1. **ExtAuth Validation:** All `/api/prove` requests must pass ZK verification
2. **Payment Verification:** Checked in Zcash Backend, not in zkVM
3. **Image ID Registry:** Should be internal-only (not exposed publicly)
4. **Rate Limiting:** Prevents abuse
5. **Proof Generation:** Runs in isolated RISC Zero zkVM environment

## Future Enhancements

1. **Customer Authentication:** Add API keys or OAuth
2. **Deployment Management UI:** View/update/delete deployments
3. **Monitoring & Metrics:** Prometheus/Grafana integration
4. **Auto-scaling:** Scale Proof Generation Service based on load
5. **Guest Program Versioning:** Support multiple versions per customer
6. **Pre-warming:** Pre-load frequently used guest programs
7. **Caching:** Cache proofs for identical inputs
8. **Multi-region:** Deploy across multiple regions for low latency

## Troubleshooting

**Deployment fails with "Build failed":**
- Check that `cargo` and `cargo-risczero` are installed
- Ensure sufficient disk space
- Check logs: `docker-compose logs logic-compiler-api`

**Proof generation fails with "Guest program not found":**
- Verify deployment was successful
- Check Image ID Registry: `curl http://localhost:8083/api/deployments/{customer_id}`
- Restart Proof Generation Service to reload from registry

**Envoy returns 503:**
- Check that all backend services are running
- Verify health checks: `curl http://localhost:8083/health`
- Check Envoy admin: `http://localhost:9901`

## API Examples

### Deploy DSL
```bash
curl -X POST http://localhost:8082/api/deploy \
  -H "Content-Type: application/json" \
  -d '{
    "dsl": {
      "use_case": "age_verification",
      "description": "Verify user age",
      "version": "1.0",
      "private_inputs": { ... },
      "public_params": { ... },
      "validation_rules": [ ... ]
    },
    "customer_id": "customer-age-verify-123"
  }'
```

### Generate Proof
```bash
curl -X POST http://localhost:8080/api/prove \
  -H "Content-Type: application/json" \
  -d '{
    "customer_id": "customer-age-verify-123",
    "private_inputs": {
      "user_data": {
        "date_of_birth": "1990-01-01"
      }
    },
    "public_params": {
      "min_age": 18
    }
  }'
```

### Check Deployment
```bash
curl http://localhost:8083/api/deployments/customer-age-verify-123
```

## Monitoring

### Health Checks
```bash
# Image ID Registry
curl http://localhost:8083/health

# Proof Generation Service
curl http://localhost:8084/health

# Logic Compiler API
curl http://localhost:8082/health

# Envoy Admin
curl http://localhost:9901/stats
```

### Logs
```bash
# All services
docker-compose logs -f

# Specific service
docker-compose logs -f proof-generation-service
```
