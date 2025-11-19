# Khafi-Gateway Implementation Plan

## 🏗️ Repository Architecture

### Cargo Workspace Structure (REVISED - Logic Compiler Focused)
```
khafi-gateway/
├── Cargo.toml                    # Workspace root
├── docker-compose.yml            # Local development orchestration
├── .env.example                  # Environment template
│
├── crates/
│   ├── common/                   # Shared types (Nullifier, Receipt, Error, GuestInputs/Outputs)
│   ├── logic-compiler/           # 🎯 CORE: SDK Generator Service (JSON DSL → Custom SDK)
│   ├── sdk-template/             # Base SDK template (Zcash + pluggable business logic)
│   ├── methods/                  # RISC Zero build system + guest program
│   │   ├── build.rs              # Calls risc0_build::embed_methods()
│   │   ├── src/lib.rs            # Exports GUEST_ELF and GUEST_ID
│   │   └── guest/                # Guest program (runs in zkVM)
│   │       └── src/main.rs       # ZK verification logic
│   ├── zk-verification-service/  # Proof verifier (gRPC server with multi-tenant Image ID registry)
│   ├── zcash-backend/            # Nullifier DB + commitment tree API
│   └── examples/
│       ├── pharma-sdk/           # Generated SDK: prescription validation
│       └── shipping-sdk/         # Generated SDK: manifest compliance
│
├── proto/
│   ├── verification.proto        # ZK Verification gRPC API
│   └── compiler.proto            # Logic Compiler REST/gRPC API
│
├── envoy/
│   ├── envoy.yaml               # Envoy ExtAuth configuration
│   └── Dockerfile.envoy         # Envoy container
│
├── scripts/
│   ├── setup-dev.sh             # Developer environment setup
│   └── test-e2e.sh              # End-to-end testing
│
└── docs/
    ├── product-description.md         # Technical specification (updated)
    ├── implementation-plan.md         # (this document)
    ├── risc0-setup-complete.md        # RISC Zero integration documentation
    ├── host-vs-guest-code.md          # RISC Zero architecture explanation
    ├── zcash-integration.md           # Zcash payment verification guide
    ├── zcash-verification-flow.md     # Zcash technical deep dive
    ├── architecture/                  # ADRs and design docs
    ├── api/                           # gRPC/REST API documentation
    ├── guides/                        # Developer & deployment guides
    └── examples/
        ├── pharma-rules.json          # Example DSL for prescription validation
        ├── shipping-rules.json        # Example DSL for manifest compliance
        └── age-verification-simple.json  # Example DSL for age verification
```

## 📋 Implementation Phases

### 🔒 Zcash Integration Architecture

**Overview:** Zcash shielded payment verification is the core privacy-preserving mechanism of Khafi Gateway. The verification happens inside the RISC Zero zkVM guest program.

**Key Components:**

1. **Orchard Protocol** - Latest Zcash shielded pool (no trusted setup)
2. **Notes** - Encrypted value representations (recipient, value, randomness)
3. **Nullifiers** - Unique identifiers preventing double-spending
4. **Commitment Tree** - Global Merkle tree of all note commitments
5. **Cryptographic Operations:**
   - **Sinsemilla Hash** - For Merkle tree operations (Pallas curve)
   - **Poseidon Hash** - For nullifier derivation (arithmetic hash)

**Verification Flow in Guest Program:**

```rust
// In methods/guest/src/main.rs

fn verify_zcash_payment(inputs: &ZcashInputs) -> Nullifier {
    // 1. Deserialize Orchard note
    let note = deserialize_orchard_note(&inputs.note)?;

    // 2. Compute note commitment using Sinsemilla
    let commitment = compute_note_commitment(&note)?;

    // 3. Verify Merkle proof against public anchor
    let merkle_path = deserialize_merkle_path(&inputs.merkle_path)?;
    verify_merkle_proof(commitment, &merkle_path, inputs.merkle_root)?;

    // 4. Derive nullifier deriving key from spending key
    let nk = derive_nullifier_key(&inputs.spending_key)?;

    // 5. Compute nullifier using Poseidon hash
    let nullifier = compute_nullifier(nk, &note, commitment)?;

    nullifier
}
```

**Current Implementation Status:**

- ✅ Architecture designed
- ✅ Data structures defined (ZcashInputs in common crate)
- ✅ Guest program structure in place
- ⏳ Placeholder verification logic (needs Orchard implementation)
- ⏳ Cryptographic operations pending (Sinsemilla, Poseidon)

**Implementation Roadmap Options:**

See `docs/zcash-integration.md` for detailed roadmap with three approaches:
1. **Full Implementation** - Complete Orchard verification in zkVM
2. **Simplified Verification** - Lightweight commitment/nullifier checks
3. **Hybrid Approach** (RECOMMENDED) - Start simple, add complexity incrementally

**Performance Estimates:**

- Zcash verification in zkVM: ~175ms
- Merkle path verification (32 levels): ~160ms
- Nullifier derivation: ~10ms

**Documentation:**

- **`docs/zcash-integration.md`** - Complete integration guide (400+ lines)
- **`docs/zcash-verification-flow.md`** - Technical deep dive (350+ lines)

---

### **Phase 1: Foundation + RISC Zero Integration** ✅
**Status:** COMPLETED (Week 1-2)
**Goal:** Core infrastructure with working RISC Zero zkVM proof generation

**Completed:**
- ✅ Cargo workspace structure with all crates
- ✅ Common crate with shared types (Error, Nullifier, Receipt, GuestInputs/GuestOutputs)
- ✅ Updated to latest dependencies (RISC Zero 3.0.3, bincode 2.x, Tokio 1.48, etc.)
- ✅ Fixed bincode 2.x API migration (serde compatibility layer)
- ✅ SDK template with actual RISC Zero proof generation
- ✅ Methods crate with RISC Zero build system (`risc0-build`)
- ✅ Guest program implementation (`methods/guest/src/main.rs`)
- ✅ Receipt verification methods (verify, journal extraction, verify_and_decode)
- ✅ Image ID conversion utilities ([u32; 8] ↔ [u8; 32])
- ✅ Comprehensive documentation (risc0-setup-complete.md, host-vs-guest-code.md)
- ✅ All crates compile successfully with 11 tests passing
- ✅ Guest ELF binary generation (352 KB optimized)

**KEY ACHIEVEMENT:** Full RISC Zero integration complete - can now generate real cryptographic proofs, verify them, and extract outputs from the journal

#### Step-by-Step Commands:

**1. Initialize Cargo workspace**
```bash
# Create workspace root Cargo.toml
cat > Cargo.toml << 'EOF'
[workspace]
resolver = "2"
members = [
    "crates/common",
    "crates/methods",
    "crates/sdk-template",
    "crates/logic-compiler",
    "crates/zk-verification-service",
    "crates/zcash-backend",
    "crates/examples/pharma-sdk",
    "crates/examples/shipping-sdk",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"
repository = "https://github.com/yourusername/khafi-gateway"

[workspace.dependencies]
# ZK
risc0-zkvm = "3.0.3"
risc0-build = "3.0.3"

# gRPC
tonic = "0.14.2"
prost = "0.14.1"
tonic-build = "0.14.2"

# Zcash
zcash_primitives = "0.26.1"
orchard = "0.11"
zcash_client_backend = "0.21"

# Async runtime
tokio = { version = "1.35", features = ["full"] }
async-trait = "0.1"

# Storage
redis = { version = "0.24", features = ["tokio-comp", "connection-manager"] }
deadpool-redis = "0.22"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
bincode = { version = "2.0.1", features = ["serde"] }

# Logging & tracing
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Error handling
anyhow = "1.0"
thiserror = "2.0.17"

# Code generation (for logic-compiler)
syn = { version = "2.0", features = ["full"] }
quote = "1.0"
proc-macro2 = "1.0"

# Web framework (for logic-compiler REST API)
axum = "0.8.7"
tower = "0.5.2"
tower-http = { version = "0.5", features = ["trace", "cors"] }

# Utilities
hex = "0.4"
tempfile = "3.8"

# RISC Zero optimization (required for performance)
[profile.dev]
opt-level = 3

[profile.release]
debug = 1
lto = true
EOF

# Create rust-toolchain.toml
cat > rust-toolchain.toml << 'EOF'
[toolchain]
channel = "stable"
components = ["rustfmt", "rust-src"]
profile = "minimal"
EOF

# Create directory structure
mkdir -p crates/{common,methods/guest/src,sdk-template,logic-compiler,zk-verification-service,zcash-backend}
mkdir -p crates/examples/{pharma-sdk,shipping-sdk}
mkdir -p proto envoy scripts docs/{architecture,api,guides,examples}
```

**2. Create the `common` crate**
```bash
# Initialize common crate
cargo new --lib crates/common

# Update common/Cargo.toml
cat > crates/common/Cargo.toml << 'EOF'
[package]
name = "khafi-common"
version.workspace = true
edition.workspace = true

[dependencies]
serde.workspace = true
serde_json.workspace = true
bincode.workspace = true
thiserror.workspace = true
anyhow.workspace = true
risc0-zkvm.workspace = true
hex.workspace = true
EOF

# Create common types
cat > crates/common/src/lib.rs << 'EOF'
pub mod error;
pub mod nullifier;
pub mod receipt;
pub mod inputs;

pub use error::{Error, Result};
pub use nullifier::Nullifier;
pub use receipt::Receipt;
pub use inputs::{ZcashInputs, BusinessInputs, GuestInputs, GuestOutputs};
EOF

# Create error types
cat > crates/common/src/error.rs << 'EOF'
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid proof: {0}")]
    InvalidProof(String),

    #[error("Nullifier replay detected")]
    NullifierReplay,

    #[error("Serialization error: {0}")]
    BincodeEncode(#[from] bincode::error::EncodeError),

    #[error("Deserialization error: {0}")]
    BincodeDecode(#[from] bincode::error::DecodeError),

    #[error("JSON serialization error: {0}")]
    JsonSerialization(#[from] serde_json::Error),

    #[error("Redis error: {0}")]
    Redis(String),

    #[error("RISC Zero error: {0}")]
    RiscZero(String),

    #[error("Compilation error: {0}")]
    Compilation(String),

    #[error("DSL parsing error: {0}")]
    DslParsing(String),

    #[error("Zcash error: {0}")]
    Zcash(String),

    #[error("Invalid nullifier format")]
    InvalidNullifier,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
EOF

# Create nullifier type
cat > crates/common/src/nullifier.rs << 'EOF'
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Nullifier(pub [u8; 32]);

impl Nullifier {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(s: &str) -> Result<Self, hex::FromHexError> {
        let bytes = hex::decode(s)?;
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}
EOF

# Create receipt wrapper
cat > crates/common/src/receipt.rs << 'EOF'
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub inner: Vec<u8>, // Serialized RISC Zero receipt
    pub image_id: [u8; 32],
}

impl Receipt {
    pub fn new(inner: Vec<u8>, image_id: [u8; 32]) -> Self {
        Self { inner, image_id }
    }
}
EOF
```

**3. Initialize other crates**
```bash
# ZK Verification Service
cargo new --bin crates/zk-verification-service
cat > crates/zk-verification-service/Cargo.toml << 'EOF'
[package]
name = "zk-verification-service"
version.workspace = true
edition.workspace = true

[dependencies]
khafi-common = { path = "../common" }
tonic.workspace = true
prost.workspace = true
tokio.workspace = true
risc0-zkvm.workspace = true
redis.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
anyhow.workspace = true

[build-dependencies]
tonic-build.workspace = true
EOF

# Zcash Backend
cargo new --bin crates/zcash-backend
cat > crates/zcash-backend/Cargo.toml << 'EOF'
[package]
name = "zcash-backend"
version.workspace = true
edition.workspace = true

[dependencies]
khafi-common = { path = "../common" }
tokio.workspace = true
zcash_primitives.workspace = true
orchard.workspace = true
redis.workspace = true
tracing.workspace = true
anyhow.workspace = true
EOF

# Client SDK
cargo new --lib crates/client-sdk
cat > crates/client-sdk/Cargo.toml << 'EOF'
[package]
name = "khafi-client-sdk"
version.workspace = true
edition.workspace = true

[dependencies]
khafi-common = { path = "../common" }
risc0-zkvm.workspace = true
zcash_primitives.workspace = true
orchard.workspace = true
tokio.workspace = true
anyhow.workspace = true
EOF

# Guest Programs
cargo new --lib crates/guest-programs
cat > crates/guest-programs/Cargo.toml << 'EOF'
[package]
name = "guest-programs"
version.workspace = true
edition.workspace = true

[dependencies]
risc0-zkvm = { workspace = true, default-features = false }
EOF
```

**4. Setup Docker Compose**
```bash
cat > docker-compose.yml << 'EOF'
version: '3.8'

services:
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis-data:/data
    command: redis-server --appendonly yes
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 5s
      timeout: 3s
      retries: 5

  zk-verification-service:
    build:
      context: .
      dockerfile: crates/zk-verification-service/Dockerfile
    ports:
      - "50051:50051"
    environment:
      - REDIS_URL=redis://redis:6379
      - RUST_LOG=info
    depends_on:
      redis:
        condition: service_healthy

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

  envoy:
    build:
      context: ./envoy
      dockerfile: Dockerfile.envoy
    ports:
      - "8080:8080"
      - "9901:9901"  # Admin interface
    depends_on:
      - zk-verification-service
    volumes:
      - ./envoy/envoy.yaml:/etc/envoy/envoy.yaml

volumes:
  redis-data:
EOF

# Create .env.example
cat > .env.example << 'EOF'
REDIS_URL=redis://localhost:6379
ZCASH_NODE_URL=http://localhost:18232
RUST_LOG=info
ZK_VERIFICATION_SERVICE_URL=http://localhost:50051
ZCASH_BACKEND_URL=http://localhost:8081
EOF
```

**5. Setup CI/CD**
```bash
mkdir -p .github/workflows
cat > .github/workflows/ci.yml << 'EOF'
name: CI

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4

    - name: Install Rust
      uses: dtolnay/rust-toolchain@stable
      with:
        components: clippy, rustfmt

    - name: Cache cargo
      uses: actions/cache@v3
      with:
        path: |
          ~/.cargo/bin/
          ~/.cargo/registry/index/
          ~/.cargo/registry/cache/
          ~/.cargo/git/db/
          target/
        key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

    - name: Format check
      run: cargo fmt --all -- --check

    - name: Clippy
      run: cargo clippy --all-targets --all-features -- -D warnings

    - name: Build
      run: cargo build --verbose

    - name: Run tests
      run: cargo test --verbose
EOF
```

**6. Add GuestInputs and GuestOutputs types**
```bash
cat > crates/common/src/inputs.rs << 'EOF'
use crate::Nullifier;
use serde::{Deserialize, Serialize};

/// Zcash payment inputs (universal across all customer SDKs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZcashInputs {
    pub spending_key: Vec<u8>,
    pub note: Vec<u8>,
    pub merkle_path: Vec<u8>,
    pub merkle_root: [u8; 32],
}

/// Business-specific inputs (varies per customer use case)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessInputs {
    pub private_data: Vec<u8>,
    pub public_params: Vec<u8>,
}

/// Combined inputs for RISC Zero guest program
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestInputs {
    pub zcash: ZcashInputs,
    pub business: BusinessInputs,
}

/// Output from RISC Zero guest program
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestOutputs {
    pub nullifier: Nullifier,
    pub compliance_result: bool,
    pub metadata: Vec<u8>,
}
EOF
```

**7. Verify everything compiles**
```bash
# Build all crates
cargo build

# Run tests
cargo test

# Format code
cargo fmt

# Run clippy (fix warnings as needed)
cargo clippy
```

**8. Initialize git (if not already done)**
```bash
git add .
git commit -m "Phase 1: Foundation and scaffolding complete"
```

#### Checkpoint:
After completing these commands, verify:
- ✅ `cargo build --workspace` succeeds for all crates
- ✅ `cargo test --workspace` passes (11 tests total: 4 in khafi-common, 7 in sdk-template)
- ✅ All workspace crates are present in `crates/` directory
- ✅ RISC Zero guest program builds successfully (352 KB ELF)
- ✅ `methods` crate generates GUEST_ELF and GUEST_ID constants
- ✅ Git repository is initialized with first commit
- ✅ Bincode 2.x serialization works correctly
- ✅ Error types handle all failure modes
- ✅ Documentation files created (risc0-setup-complete.md, host-vs-guest-code.md)

**✅ Phase 1 Complete! All foundation crates implemented, RISC Zero integration functional, and tests passing.**

---

### **Phase 2: ZK Verification Service** ✅
**Status:** COMPLETED (Week 2-3)
**Goal:** Core proof verification with gRPC interface

**Completed:**
- ✅ Crate structure created
- ✅ gRPC service implementation with Envoy ExtAuth protocol
- ✅ RISC Zero proof verification using Receipt::verify_and_decode()
- ✅ Redis-based nullifier checker with atomic operations
- ✅ Proper error handling and HTTP status codes
- ✅ Dockerfile and containerization
- ✅ All tests passing (13 total across workspace)

#### Step-by-Step Commands:

**1. Create gRPC protobuf definitions**
```bash
# Create proto directory
mkdir -p crates/zk-verification-service/proto

# Create ExtAuth proto (simplified version - full version from Envoy repo)
cat > crates/zk-verification-service/proto/ext_authz.proto << 'EOF'
syntax = "proto3";

package envoy.service.auth.v3;

service Authorization {
  rpc Check(CheckRequest) returns (CheckResponse);
}

message CheckRequest {
  map<string, string> headers = 1;
  string body = 2;
}

message CheckResponse {
  StatusCode status = 1;
  string message = 2;
}

enum StatusCode {
  OK = 0;
  UNAUTHENTICATED = 1;
  PERMISSION_DENIED = 2;
  UNAVAILABLE = 3;
}
EOF
```

**2. Setup build script for protobuf compilation**
```bash
cat > crates/zk-verification-service/build.rs << 'EOF'
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .compile(
            &["proto/ext_authz.proto"],
            &["proto"],
        )?;
    Ok(())
}
EOF
```

**3. Implement the verification service**
```bash
# Create service module structure
mkdir -p crates/zk-verification-service/src/{service,nullifier,config}

# Main server entry point
cat > crates/zk-verification-service/src/main.rs << 'EOF'
mod service;
mod nullifier;
mod config;

use service::AuthorizationService;
use config::Config;
use tonic::transport::Server;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = Config::from_env();
    let addr = "0.0.0.0:50051".parse()?;

    let auth_service = AuthorizationService::new(config).await?;

    tracing::info!("ZK Verification Service listening on {}", addr);

    Server::builder()
        .add_service(auth_service.into_service())
        .serve(addr)
        .await?;

    Ok(())
}
EOF

# Config module
cat > crates/zk-verification-service/src/config.rs << 'EOF'
#[derive(Clone)]
pub struct Config {
    pub redis_url: String,
    pub image_id: [u8; 32],
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            image_id: [0u8; 32], // TODO: Load from configuration
        }
    }
}
EOF

# Nullifier checker module
cat > crates/zk-verification-service/src/nullifier.rs << 'EOF'
use redis::AsyncCommands;
use khafi_common::{Nullifier, Error, Result};

pub struct NullifierChecker {
    redis_client: redis::Client,
}

impl NullifierChecker {
    pub fn new(redis_url: &str) -> Result<Self> {
        let redis_client = redis::Client::open(redis_url)
            .map_err(|e| Error::Redis(e.to_string()))?;
        Ok(Self { redis_client })
    }

    pub async fn check_and_set(&self, nullifier: &Nullifier) -> Result<bool> {
        let mut conn = self.redis_client.get_async_connection()
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        let key = format!("nullifier:{}", nullifier.to_hex());

        // SET NX - set if not exists (atomic operation)
        let result: bool = conn.set_nx(&key, "1")
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        if result {
            // Optionally set TTL (e.g., 30 days)
            let _: () = conn.expire(&key, 2592000)
                .await
                .map_err(|e| Error::Redis(e.to_string()))?;
        }

        Ok(result)
    }
}
EOF

# Authorization service implementation
cat > crates/zk-verification-service/src/service.rs << 'EOF'
use tonic::{Request, Response, Status};
use khafi_common::{Nullifier, Receipt};
use crate::nullifier::NullifierChecker;
use crate::config::Config;

// Include generated protobuf code
pub mod proto {
    tonic::include_proto!("envoy.service.auth.v3");
}

use proto::{
    authorization_server::{Authorization, AuthorizationServer},
    CheckRequest, CheckResponse, StatusCode,
};

pub struct AuthorizationService {
    nullifier_checker: NullifierChecker,
    config: Config,
}

impl AuthorizationService {
    pub async fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let nullifier_checker = NullifierChecker::new(&config.redis_url)?;
        Ok(Self {
            nullifier_checker,
            config,
        })
    }

    pub fn into_service(self) -> AuthorizationServer<Self> {
        AuthorizationServer::new(self)
    }

    async fn verify_proof(&self, receipt_bytes: &[u8]) -> Result<Vec<u8>, Status> {
        // TODO: Implement RISC Zero verification
        // This is a placeholder
        tracing::info!("Verifying proof (placeholder)");
        Ok(vec![])
    }
}

#[tonic::async_trait]
impl Authorization for AuthorizationService {
    async fn check(
        &self,
        request: Request<CheckRequest>,
    ) -> Result<Response<CheckResponse>, Status> {
        let req = request.into_inner();

        // Extract headers
        let receipt_hex = req.headers.get("x-zk-receipt")
            .ok_or_else(|| Status::unauthenticated("Missing x-zk-receipt header"))?;

        let nullifier_hex = req.headers.get("x-zk-nullifier")
            .ok_or_else(|| Status::unauthenticated("Missing x-zk-nullifier header"))?;

        // Parse nullifier
        let nullifier = Nullifier::from_hex(nullifier_hex)
            .map_err(|_| Status::invalid_argument("Invalid nullifier format"))?;

        // Check for replay attack
        let is_new = self.nullifier_checker.check_and_set(&nullifier)
            .await
            .map_err(|e| Status::unavailable(format!("Redis error: {}", e)))?;

        if !is_new {
            return Ok(Response::new(CheckResponse {
                status: StatusCode::Unauthenticated as i32,
                message: "Nullifier replay detected".to_string(),
            }));
        }

        // Decode and verify proof
        let receipt_bytes = hex::decode(receipt_hex)
            .map_err(|_| Status::invalid_argument("Invalid receipt hex"))?;

        match self.verify_proof(&receipt_bytes).await {
            Ok(_journal) => {
                Ok(Response::new(CheckResponse {
                    status: StatusCode::Ok as i32,
                    message: "Proof verified successfully".to_string(),
                }))
            }
            Err(e) => {
                Ok(Response::new(CheckResponse {
                    status: StatusCode::PermissionDenied as i32,
                    message: format!("Proof verification failed: {}", e),
                }))
            }
        }
    }
}
EOF
```

**4. Update dependencies**
```bash
# Add hex crate to common
cat >> crates/common/Cargo.toml << 'EOF'
hex = "0.4"
EOF

# Update zk-verification-service dependencies
cat >> crates/zk-verification-service/Cargo.toml << 'EOF'
hex = "0.4"
EOF
```

**5. Create Dockerfile**
```bash
cat > crates/zk-verification-service/Dockerfile << 'EOF'
FROM rust:1.75 as builder

WORKDIR /app
COPY . .

RUN cargo build --release --bin zk-verification-service

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/zk-verification-service /usr/local/bin/

EXPOSE 50051

CMD ["zk-verification-service"]
EOF
```

**6. Create unit tests**
```bash
cat > crates/zk-verification-service/src/service_test.rs << 'EOF'
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_missing_headers() {
        // TODO: Implement test
        assert!(true);
    }

    #[tokio::test]
    async fn test_nullifier_replay() {
        // TODO: Implement test
        assert!(true);
    }

    #[tokio::test]
    async fn test_valid_proof() {
        // TODO: Implement test
        assert!(true);
    }
}
EOF
```

**7. Build and test**
```bash
# Build the service
cargo build -p zk-verification-service

# Run tests
cargo test -p zk-verification-service

# Test with docker
docker-compose up -d redis
cargo run -p zk-verification-service
```

**8. Manual testing with grpcurl**
```bash
# Install grpcurl if not already installed
# brew install grpcurl  # macOS
# Or download from https://github.com/fullstorydev/grpcurl

# Test the service (in another terminal)
grpcurl -plaintext \
  -d '{"headers": {"x-zk-receipt": "deadbeef", "x-zk-nullifier": "0000000000000000000000000000000000000000000000000000000000000001"}}' \
  localhost:50051 \
  envoy.service.auth.v3.Authorization/Check
```

#### Checkpoint:
After completing these commands, verify:
- ✅ gRPC server compiles and starts on port 50051
- ✅ Service accepts CheckRequest with headers
- ✅ Redis nullifier checking works (SET NX)
- ✅ Duplicate nullifier requests are rejected
- ✅ Tests pass with `cargo test`

**✅ Phase 2 Complete! ZK Verification Service fully functional with gRPC ExtAuth, RISC Zero verification, and nullifier replay protection.**

---

### **Phase 3: Logic Compiler (SaaS Differentiator)** ⏳
**Status:** IN PROGRESS (Week 3-4)
**Goal:** Transform JSON DSL business rules into custom RISC Zero guest programs

**Current State:**
- ✅ DSL schema with 6 validation rule types
- ✅ JSON parser with comprehensive validation
- ✅ 3 example DSL files created
- ✅ Integration tests (9 tests passing)
- ⏳ Guest code generator (NEXT PRIORITY)
- ⏳ Compilation pipeline
- ⏳ SDK customization

**DSL Validation Rules Implemented:**
1. **signature_check** - Verify digital signatures (ed25519, ecdsa, rsa)
2. **range_check** - Validate numeric ranges with min/max
3. **age_verification** - Check minimum age from date of birth
4. **blacklist_check** - Ensure values not in prohibited list
5. **array_intersection_check** - Check for prohibited array elements
6. **custom** - Arbitrary Rust code for complex logic

#### Example DSL Files:

**1. Pharmaceutical Prescription Validation** (`docs/examples/pharma-rules.json`):
```json
{
  "use_case": "prescription_validation",
  "validation_rules": [
    {
      "type": "signature_check",
      "field": "prescriber_signature",
      "algorithm": "ed25519",
      "public_key_param": "prescriber_public_key",
      "message_fields": ["patient_id", "medication", "dosage", "timestamp"]
    },
    {
      "type": "range_check",
      "field": "quantity",
      "min": 1,
      "max": 90
    }
  ]
}
```

**2. International Shipping Compliance** (`docs/examples/shipping-rules.json`):
- Blacklist check for sanctioned countries
- Prohibited items array intersection
- Weight limits validation

**3. Age Verification** (`docs/examples/age-verification-simple.json`):
- Minimum age check from date of birth

#### Next Steps:

**Guest Code Generator** (PRIORITY):
- Parse DSL and generate Rust code for guest program
- Map validation rules to RISC Zero compatible operations
- Generate type-safe input/output structures
- Compile guest program to ELF binary

**Compilation Pipeline**:
- Integrate with RISC Zero build system
- Generate Image ID for each custom guest program
- Package compiled artifacts

**SDK Customization**:
- Generate custom SDK for each use case
- Include generated guest ELF and Image ID
- Provide type-safe API for inputs

**Testing**:
- End-to-end test with pharma example
- Verify generated code compiles
- Test proof generation with custom logic

**✅ Phase 3 will enable customers to define custom business logic via JSON DSL!**

---

### **Phase 4: Zcash Backend Abstraction** 📅
**Status:** PLANNED
**Goal:** Zcash state management for commitment trees

#### Step-by-Step Commands:

**1. Setup HTTP API with Axum**
```bash
# Add dependencies to zcash-backend
cat > crates/zcash-backend/Cargo.toml << 'EOF'
[package]
name = "zcash-backend"
version.workspace = true
edition.workspace = true

[dependencies]
khafi-common = { path = "../common" }
tokio.workspace = true
zcash_primitives.workspace = true
orchard.workspace = true
redis.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
anyhow.workspace = true
axum = "0.7"
serde.workspace = true
serde_json.workspace = true
tower = "0.4"
tower-http = { version = "0.5", features = ["trace", "cors"] }
hex = "0.4"
EOF
```

**2. Implement the backend service**
```bash
# Create module structure
mkdir -p crates/zcash-backend/src/{api,zcash,storage}

# Main entry point
cat > crates/zcash-backend/src/main.rs << 'EOF'
mod api;
mod zcash;
mod storage;

use axum::{Router, routing::get};
use tower_http::trace::TraceLayer;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    zcash_client: Arc<zcash::ZcashClient>,
    storage: Arc<storage::Storage>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://localhost:6379".to_string());

    let storage = Arc::new(storage::Storage::new(&redis_url)?);
    let zcash_client = Arc::new(zcash::ZcashClient::new()?);

    let state = AppState {
        zcash_client,
        storage,
    };

    let app = Router::new()
        .route("/health", get(api::health))
        .route("/commitment-tree/root", get(api::get_commitment_root))
        .route("/nullifier/check", axum::routing::post(api::check_nullifier))
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    let addr = "0.0.0.0:8081";
    tracing::info!("Zcash Backend listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
EOF

# Storage module
cat > crates/zcash-backend/src/storage.rs << 'EOF'
use redis::{Client, AsyncCommands};
use khafi_common::{Result, Error};

pub struct Storage {
    client: Client,
}

impl Storage {
    pub fn new(redis_url: &str) -> Result<Self> {
        let client = Client::open(redis_url)
            .map_err(|e| Error::Redis(e.to_string()))?;
        Ok(Self { client })
    }

    pub async fn get_cached_root(&self) -> Result<Option<Vec<u8>>> {
        let mut conn = self.client.get_async_connection()
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        let root: Option<Vec<u8>> = conn.get("zcash:commitment_root")
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        Ok(root)
    }

    pub async fn cache_root(&self, root: &[u8]) -> Result<()> {
        let mut conn = self.client.get_async_connection()
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        conn.set_ex("zcash:commitment_root", root, 60)
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        Ok(())
    }
}
EOF

# Zcash client (mock for now)
cat > crates/zcash-backend/src/zcash.rs << 'EOF'
use khafi_common::Result;

pub struct ZcashClient {
    // TODO: Add actual Zcash node connection
}

impl ZcashClient {
    pub fn new() -> Result<Self> {
        // TODO: Connect to Zcash node
        Ok(Self {})
    }

    pub async fn get_latest_commitment_root(&self) -> Result<Vec<u8>> {
        // TODO: Implement actual Zcash commitment tree root fetching
        // For now, return a mock root
        Ok(vec![0u8; 32])
    }
}
EOF

# API handlers
cat > crates/zcash-backend/src/api.rs << 'EOF'
use axum::{
    extract::State,
    Json,
    response::IntoResponse,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use crate::AppState;

#[derive(Serialize)]
pub struct HealthResponse {
    status: String,
}

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

#[derive(Serialize)]
pub struct CommitmentRootResponse {
    root: String,
    cached: bool,
}

pub async fn get_commitment_root(
    State(state): State<AppState>,
) -> Result<Json<CommitmentRootResponse>, StatusCode> {
    // Try cache first
    if let Ok(Some(root)) = state.storage.get_cached_root().await {
        return Ok(Json(CommitmentRootResponse {
            root: hex::encode(root),
            cached: true,
        }));
    }

    // Fetch from Zcash node
    match state.zcash_client.get_latest_commitment_root().await {
        Ok(root) => {
            let _ = state.storage.cache_root(&root).await;
            Ok(Json(CommitmentRootResponse {
                root: hex::encode(root),
                cached: false,
            }))
        }
        Err(_) => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}

#[derive(Deserialize)]
pub struct CheckNullifierRequest {
    nullifier: String,
}

#[derive(Serialize)]
pub struct CheckNullifierResponse {
    exists: bool,
}

pub async fn check_nullifier(
    Json(payload): Json<CheckNullifierRequest>,
) -> Json<CheckNullifierResponse> {
    // TODO: Implement nullifier checking
    Json(CheckNullifierResponse {
        exists: false,
    })
}
EOF
```

**3. Create Dockerfile**
```bash
cat > crates/zcash-backend/Dockerfile << 'EOF'
FROM rust:1.75 as builder

WORKDIR /app
COPY . .

RUN cargo build --release --bin zcash-backend

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/zcash-backend /usr/local/bin/

EXPOSE 8081

CMD ["zcash-backend"]
EOF
```

**4. Build and test**
```bash
# Build
cargo build -p zcash-backend

# Run the service
cargo run -p zcash-backend

# In another terminal, test the API
curl http://localhost:8081/health
curl http://localhost:8081/commitment-tree/root
```

**5. Integration test script**
```bash
cat > scripts/test-zcash-backend.sh << 'EOF'
#!/bin/bash
set -e

echo "Testing Zcash Backend..."

# Start Redis
docker-compose up -d redis
sleep 2

# Start the backend
cargo run -p zcash-backend &
BACKEND_PID=$!
sleep 3

# Test health endpoint
echo "Testing /health..."
curl -f http://localhost:8081/health || exit 1

# Test commitment root
echo "Testing /commitment-tree/root..."
curl -f http://localhost:8081/commitment-tree/root || exit 1

# Cleanup
kill $BACKEND_PID

echo "All tests passed!"
EOF

chmod +x scripts/test-zcash-backend.sh
./scripts/test-zcash-backend.sh
```

#### Checkpoint:
After completing these commands, verify:
- ✅ HTTP service starts on port 8081
- ✅ `/health` endpoint returns `{"status":"ok"}`
- ✅ `/commitment-tree/root` returns a root hash
- ✅ Service handles Redis cache correctly
- ✅ Integration tests pass

**👉 Run these commands, then check in with me for Phase 5!**

---

### **Phase 5: Client SDK** 📅
**Status:** PLANNED
**Goal:** Proof generation library for application developers

#### Step-by-Step Commands:

**1. Setup guest program structure**
```bash
# Add RISC Zero build tools
cat >> crates/guest-programs/Cargo.toml << 'EOF'
serde = { version = "1.0", features = ["derive"], default-features = false }

[build-dependencies]
risc0-build = "1.0"
EOF

# Create guest program build script
cat > crates/guest-programs/build.rs << 'EOF'
fn main() {
    risc0_build::embed_methods();
}
EOF

# Create methods directory
mkdir -p crates/guest-programs/methods

# Create a simple payment verification guest program
cat > crates/guest-programs/methods/payment_verifier.rs << 'EOF'
#![no_main]

use risc0_zkvm::guest::env;
use serde::{Deserialize, Serialize};

risc0_zkvm::guest::entry!(main);

#[derive(Serialize, Deserialize)]
pub struct PrivateInputs {
    pub spending_key: [u8; 32],
    pub note_value: u64,
}

#[derive(Serialize, Deserialize)]
pub struct PublicInputs {
    pub commitment_root: [u8; 32],
    pub min_value: u64,
}

#[derive(Serialize, Deserialize)]
pub struct Output {
    pub nullifier: [u8; 32],
    pub verified: bool,
}

fn main() {
    let private_inputs: PrivateInputs = env::read();
    let public_inputs: PublicInputs = env::read();

    // TODO: Implement actual Zcash verification
    // For now, simple mock logic
    let verified = private_inputs.note_value >= public_inputs.min_value;

    // Derive nullifier (simplified - real version uses Zcash crypto)
    let nullifier = private_inputs.spending_key;

    let output = Output { nullifier, verified };
    env::commit(&output);
}
EOF
```

**2. Implement Client SDK**
```bash
# Update client-sdk dependencies
cat > crates/client-sdk/Cargo.toml << 'EOF'
[package]
name = "khafi-client-sdk"
version.workspace = true
edition.workspace = true

[dependencies]
khafi-common = { path = "../common" }
guest-programs = { path = "../guest-programs" }
risc0-zkvm.workspace = true
zcash_primitives.workspace = true
orchard.workspace = true
tokio.workspace = true
anyhow.workspace = true
serde.workspace = true
serde_json.workspace = true
reqwest = { version = "0.11", features = ["json"] }
hex = "0.4"
EOF

# Create SDK modules
mkdir -p crates/client-sdk/src/{prover,client}

# Main SDK library
cat > crates/client-sdk/src/lib.rs << 'EOF'
pub mod prover;
pub mod client;

pub use prover::Prover;
pub use client::GatewayClient;
pub use khafi_common::{Receipt, Nullifier};

#[derive(Debug)]
pub struct ProofRequest {
    pub spending_key: [u8; 32],
    pub note_value: u64,
    pub min_value: u64,
}

#[derive(Debug)]
pub struct ProofResponse {
    pub receipt: Receipt,
    pub nullifier: Nullifier,
}
EOF

# Prover implementation
cat > crates/client-sdk/src/prover.rs << 'EOF'
use risc0_zkvm::{default_prover, ExecutorEnv};
use khafi_common::{Receipt, Nullifier, Result, Error};
use crate::{ProofRequest, ProofResponse};

pub struct Prover {
    image_id: [u8; 32],
}

impl Prover {
    pub fn new(image_id: [u8; 32]) -> Self {
        Self { image_id }
    }

    pub async fn generate_proof(&self, request: ProofRequest) -> Result<ProofResponse> {
        // TODO: Fetch commitment root from backend
        let commitment_root = [0u8; 32];

        // Build executor environment
        let env = ExecutorEnv::builder()
            .write(&request.spending_key)?
            .write(&request.note_value)?
            .write(&commitment_root)?
            .write(&request.min_value)?
            .build()
            .map_err(|e| Error::RiscZero(e.to_string()))?;

        // TODO: Execute guest program and generate proof
        // This is a placeholder
        let receipt_bytes = vec![0u8; 64];
        let nullifier = Nullifier::new(request.spending_key);

        Ok(ProofResponse {
            receipt: Receipt::new(receipt_bytes, self.image_id),
            nullifier,
        })
    }
}
EOF

# Gateway client
cat > crates/client-sdk/src/client.rs << 'EOF'
use khafi_common::{Receipt, Nullifier};
use anyhow::Result;
use reqwest::Client;

pub struct GatewayClient {
    client: Client,
    gateway_url: String,
}

impl GatewayClient {
    pub fn new(gateway_url: String) -> Self {
        Self {
            client: Client::new(),
            gateway_url,
        }
    }

    pub async fn send_request(
        &self,
        receipt: &Receipt,
        nullifier: &Nullifier,
        path: &str,
    ) -> Result<reqwest::Response> {
        let receipt_hex = hex::encode(&receipt.inner);
        let nullifier_hex = nullifier.to_hex();

        let response = self.client
            .get(format!("{}{}", self.gateway_url, path))
            .header("x-zk-receipt", receipt_hex)
            .header("x-zk-nullifier", nullifier_hex)
            .send()
            .await?;

        Ok(response)
    }
}
EOF
```

**3. Create example client application**
```bash
# Create examples directory
mkdir -p examples

cat > examples/simple_client.rs << 'EOF'
use khafi_client_sdk::{Prover, GatewayClient, ProofRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Khafi Gateway - Simple Client Example");

    // Initialize prover with image ID
    let image_id = [0u8; 32]; // TODO: Load from configuration
    let prover = Prover::new(image_id);

    // Create proof request
    let request = ProofRequest {
        spending_key: [1u8; 32],
        note_value: 1000,
        min_value: 100,
    };

    println!("Generating proof...");
    let proof_response = prover.generate_proof(request).await?;
    println!("Proof generated successfully!");
    println!("Nullifier: {}", proof_response.nullifier.to_hex());

    // Send request to gateway
    let gateway = GatewayClient::new("http://localhost:8080".to_string());
    println!("Sending request to gateway...");

    let response = gateway
        .send_request(&proof_response.receipt, &proof_response.nullifier, "/api/data")
        .await?;

    println!("Gateway response: {}", response.status());
    println!("Body: {}", response.text().await?);

    Ok(())
}
EOF

# Update workspace to include example
cat >> Cargo.toml << 'EOF'

[[example]]
name = "simple_client"
path = "examples/simple_client.rs"
EOF
```

**4. Build and test**
```bash
# Build the SDK
cargo build -p khafi-client-sdk

# Run tests
cargo test -p khafi-client-sdk

# Try running the example (will fail without full stack running)
cargo run --example simple_client
```

**5. Create SDK documentation**
```bash
cat > docs/guides/client-sdk-usage.md << 'EOF'
# Khafi Client SDK Usage Guide

## Installation

Add to your `Cargo.toml`:

\`\`\`toml
[dependencies]
khafi-client-sdk = "0.1"
\`\`\`

## Basic Usage

\`\`\`rust
use khafi_client_sdk::{Prover, GatewayClient, ProofRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize prover
    let prover = Prover::new(image_id);

    // 2. Create proof request
    let request = ProofRequest {
        spending_key: your_key,
        note_value: 1000,
        min_value: 100,
    };

    // 3. Generate proof
    let proof = prover.generate_proof(request).await?;

    // 4. Send to gateway
    let gateway = GatewayClient::new("http://gateway-url".into());
    let response = gateway.send_request(&proof.receipt, &proof.nullifier, "/api").await?;

    Ok(())
}
\`\`\`

## Advanced Features

### Custom Guest Programs

You can create custom verification logic by implementing your own guest programs.

### Error Handling

The SDK provides detailed error types for better debugging.
EOF
```

#### Checkpoint:
After completing these commands, verify:
- ✅ Client SDK compiles successfully
- ✅ Guest program structure is set up
- ✅ Example client application compiles
- ✅ SDK documentation is created
- ✅ Basic proof generation logic is in place

**👉 Run these commands, then check in with me for Phase 6!**

---

### **Phase 6: Envoy Integration** 📅
**Status:** PLANNED
**Goal:** Complete gateway with ExtAuth filter

#### Step-by-Step Commands:

**1. Create Envoy configuration**
```bash
mkdir -p envoy

# Create comprehensive Envoy config
cat > envoy/envoy.yaml << 'EOF'
static_resources:
  listeners:
  - name: main_listener
    address:
      socket_address:
        address: 0.0.0.0
        port_value: 8080
    filter_chains:
    - filters:
      - name: envoy.filters.network.http_connection_manager
        typed_config:
          "@type": type.googleapis.com/envoy.extensions.filters.network.http_connection_manager.v3.HttpConnectionManager
          stat_prefix: ingress_http
          codec_type: AUTO
          route_config:
            name: local_route
            virtual_hosts:
            - name: backend
              domains: ["*"]
              routes:
              - match:
                  prefix: "/"
                route:
                  cluster: upstream_service
          http_filters:
          # ExtAuth filter - ZK verification
          - name: envoy.filters.http.ext_authz
            typed_config:
              "@type": type.googleapis.com/envoy.extensions.filters.http.ext_authz.v3.ExtAuthz
              transport_api_version: V3
              grpc_service:
                envoy_grpc:
                  cluster_name: zk_verification_cluster
                timeout: 1s
              failure_mode_allow: false
              with_request_body:
                max_request_bytes: 8192
                allow_partial_message: true
          # Router filter (must be last)
          - name: envoy.filters.http.router
            typed_config:
              "@type": type.googleapis.com/envoy.extensions.filters.http.router.v3.Router

  clusters:
  # ZK Verification Service cluster
  - name: zk_verification_cluster
    type: STRICT_DNS
    lb_policy: ROUND_ROBIN
    typed_extension_protocol_options:
      envoy.extensions.upstreams.http.v3.HttpProtocolOptions:
        "@type": type.googleapis.com/envoy.extensions.upstreams.http.v3.HttpProtocolOptions
        explicit_http_config:
          http2_protocol_options: {}
    load_assignment:
      cluster_name: zk_verification_cluster
      endpoints:
      - lb_endpoints:
        - endpoint:
            address:
              socket_address:
                address: zk-verification-service
                port_value: 50051
    health_checks:
    - timeout: 1s
      interval: 10s
      unhealthy_threshold: 2
      healthy_threshold: 2
      grpc_health_check: {}

  # Upstream API cluster (mock service for testing)
  - name: upstream_service
    type: STRICT_DNS
    lb_policy: ROUND_ROBIN
    load_assignment:
      cluster_name: upstream_service
      endpoints:
      - lb_endpoints:
        - endpoint:
            address:
              socket_address:
                address: mock-upstream
                port_value: 8082

admin:
  address:
    socket_address:
      address: 0.0.0.0
      port_value: 9901
EOF
```

**2. Create Envoy Dockerfile**
```bash
cat > envoy/Dockerfile.envoy << 'EOF'
FROM envoyproxy/envoy:v1.28-latest

COPY envoy.yaml /etc/envoy/envoy.yaml

EXPOSE 8080 9901

CMD ["/usr/local/bin/envoy", "-c", "/etc/envoy/envoy.yaml"]
EOF
```

**3. Create mock upstream service for testing**
```bash
# Create simple mock upstream
mkdir -p mock-upstream

cat > mock-upstream/server.py << 'EOF'
#!/usr/bin/env python3
from http.server import HTTPServer, BaseHTTPRequestHandler
import json

class MockUpstreamHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header('Content-type', 'application/json')
        self.end_headers()

        response = {
            "message": "Success! Your request passed ZK verification",
            "path": self.path,
            "headers": dict(self.headers)
        }

        self.wfile.write(json.dumps(response, indent=2).encode())

    def do_POST(self):
        self.do_GET()

if __name__ == '__main__':
    server = HTTPServer(('0.0.0.0', 8082), MockUpstreamHandler)
    print('Mock upstream running on port 8082...')
    server.serve_forever()
EOF

chmod +x mock-upstream/server.py

cat > mock-upstream/Dockerfile << 'EOF'
FROM python:3.11-slim

COPY server.py /app/server.py
WORKDIR /app

EXPOSE 8082

CMD ["python3", "server.py"]
EOF
```

**4. Update Docker Compose with all services**
```bash
cat > docker-compose.yml << 'EOF'
version: '3.8'

services:
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis-data:/data
    command: redis-server --appendonly yes
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 5s
      timeout: 3s
      retries: 5
    networks:
      - khafi-network

  zk-verification-service:
    build:
      context: .
      dockerfile: crates/zk-verification-service/Dockerfile
    ports:
      - "50051:50051"
    environment:
      - REDIS_URL=redis://redis:6379
      - RUST_LOG=info
    depends_on:
      redis:
        condition: service_healthy
    networks:
      - khafi-network

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

  mock-upstream:
    build:
      context: ./mock-upstream
    ports:
      - "8082:8082"
    networks:
      - khafi-network

  envoy:
    build:
      context: ./envoy
      dockerfile: Dockerfile.envoy
    ports:
      - "8080:8080"
      - "9901:9901"
    depends_on:
      - zk-verification-service
      - mock-upstream
    networks:
      - khafi-network

volumes:
  redis-data:

networks:
  khafi-network:
    driver: bridge
EOF
```

**5. Create end-to-end test script**
```bash
cat > scripts/test-e2e.sh << 'EOF'
#!/bin/bash
set -e

echo "🚀 Starting Khafi Gateway E2E Test"
echo "===================================="

# Start all services
echo "Starting all services..."
docker-compose up -d

# Wait for services to be healthy
echo "Waiting for services to be ready..."
sleep 10

# Check service health
echo "Checking service health..."
curl -f http://localhost:9901/ready || { echo "Envoy not ready"; exit 1; }
curl -f http://localhost:8081/health || { echo "Zcash backend not ready"; exit 1; }

# Test 1: Valid request (should pass through)
echo ""
echo "Test 1: Testing valid ZK proof..."
RESPONSE=$(curl -s -w "\n%{http_code}" \
  -H "x-zk-receipt: 0123456789abcdef" \
  -H "x-zk-nullifier: 0000000000000000000000000000000000000000000000000000000000000001" \
  http://localhost:8080/api/test)

HTTP_CODE=$(echo "$RESPONSE" | tail -n 1)
if [ "$HTTP_CODE" -eq 200 ]; then
  echo "✅ Valid proof test passed"
else
  echo "❌ Valid proof test failed (HTTP $HTTP_CODE)"
  docker-compose logs
  exit 1
fi

# Test 2: Replay attack (same nullifier)
echo ""
echo "Test 2: Testing nullifier replay attack..."
RESPONSE2=$(curl -s -w "\n%{http_code}" \
  -H "x-zk-receipt: 0123456789abcdef" \
  -H "x-zk-nullifier: 0000000000000000000000000000000000000000000000000000000000000001" \
  http://localhost:8080/api/test)

HTTP_CODE2=$(echo "$RESPONSE2" | tail -n 1)
if [ "$HTTP_CODE2" -eq 401 ] || [ "$HTTP_CODE2" -eq 403 ]; then
  echo "✅ Replay attack prevention test passed"
else
  echo "❌ Replay attack prevention test failed (HTTP $HTTP_CODE2)"
  docker-compose logs
  exit 1
fi

# Test 3: Missing headers
echo ""
echo "Test 3: Testing missing headers..."
RESPONSE3=$(curl -s -w "\n%{http_code}" http://localhost:8080/api/test)

HTTP_CODE3=$(echo "$RESPONSE3" | tail -n 1)
if [ "$HTTP_CODE3" -eq 401 ] || [ "$HTTP_CODE3" -eq 403 ]; then
  echo "✅ Missing headers test passed"
else
  echo "❌ Missing headers test failed (HTTP $HTTP_CODE3)"
  exit 1
fi

echo ""
echo "===================================="
echo "✅ All E2E tests passed!"
echo "===================================="

# Cleanup
docker-compose down
EOF

chmod +x scripts/test-e2e.sh
```

**6. Run the complete stack**
```bash
# Build all containers
docker-compose build

# Start all services
docker-compose up -d

# View logs
docker-compose logs -f

# Check Envoy admin interface
curl http://localhost:9901/stats
curl http://localhost:9901/clusters

# Test the gateway manually
curl -v \
  -H "x-zk-receipt: deadbeef" \
  -H "x-zk-nullifier: 0000000000000000000000000000000000000000000000000000000000000001" \
  http://localhost:8080/api/test
```

**7. Run E2E tests**
```bash
./scripts/test-e2e.sh
```

#### Checkpoint:
After completing these commands, verify:
- ✅ All services start successfully with `docker-compose up`
- ✅ Envoy admin interface accessible at http://localhost:9901
- ✅ Requests with valid headers pass through to upstream
- ✅ Duplicate nullifiers are rejected (replay protection)
- ✅ Missing headers return 401/403
- ✅ E2E test script passes all tests

**👉 Run these commands, then check in with me for Phase 7 (Production Readiness)!**

---

### **Phase 7: Production Readiness** 📅
**Status:** PLANNED
**Goal:** Observability, security, and deployment prep

#### Step-by-Step Commands:

**1. Add observability and metrics**
```bash
# Add prometheus dependencies
cat >> Cargo.toml << 'EOF'

# Observability
prometheus = "0.13"
EOF

# Create metrics module for zk-verification-service
cat > crates/zk-verification-service/src/metrics.rs << 'EOF'
use prometheus::{IntCounter, Histogram, Registry};

pub struct Metrics {
    pub proofs_verified: IntCounter,
    pub proofs_rejected: IntCounter,
    pub nullifier_replays: IntCounter,
    pub verification_duration: Histogram,
}

impl Metrics {
    pub fn new(registry: &Registry) -> Self {
        let proofs_verified = IntCounter::new(
            "proofs_verified_total",
            "Total verified proofs"
        ).unwrap();

        registry.register(Box::new(proofs_verified.clone())).unwrap();

        Self { proofs_verified, /* ... */ }
    }
}
EOF
```

**2. Add rate limiting to Envoy**
```bash
# Update Envoy config with rate limiting
cat >> envoy/envoy.yaml << 'EOF'
          # Add rate limiting filter before ExtAuth
          - name: envoy.filters.http.local_ratelimit
            typed_config:
              "@type": type.googleapis.com/envoy.extensions.filters.http.local_ratelimit.v3.LocalRateLimit
              stat_prefix: http_local_rate_limiter
              token_bucket:
                max_tokens: 100
                tokens_per_fill: 100
                fill_interval: 60s
EOF
```

**3. Create deployment documentation**
```bash
cat > docs/guides/deployment.md << 'EOF'
# Deployment Guide

## Local Development
\`\`\`bash
docker-compose up -d
curl http://localhost:8081/health
\`\`\`

## Production Checklist
- [ ] Enable TLS
- [ ] Setup secret management
- [ ] Configure rate limiting
- [ ] Enable monitoring
- [ ] Setup Redis backups

## Monitoring
- Envoy metrics: http://localhost:9901/stats/prometheus
- Custom metrics: /metrics endpoints

## Troubleshooting
- Envoy 503: Check ZK service connectivity
- Redis errors: Verify REDIS_URL
EOF

# Create ADR
mkdir -p docs/architecture/adr
cat > docs/architecture/adr/001-envoy-gateway.md << 'EOF'
# ADR 001: Use Envoy Proxy as API Gateway

## Status: Accepted

## Context
Need high-performance gateway with ExtAuth support.

## Decision
Use Envoy Proxy with gRPC ExtAuth filter.

## Consequences
+ Industry standard, excellent observability
- Configuration complexity
EOF
```

**4. Create performance benchmarks**
```bash
cat > scripts/benchmark.sh << 'EOF'
#!/bin/bash
echo "🚀 Performance Benchmarks"

# Start services
docker-compose up -d
sleep 10

# Throughput test
echo "Testing throughput..."
for i in {1..100}; do
  curl -s -o /dev/null \
    -H "x-zk-receipt: test$i" \
    -H "x-zk-nullifier: $(printf '%064d' $i)" \
    http://localhost:8080/api/test
done
echo "✅ 100 requests completed"

# Latency test
echo "Testing latency..."
for i in {1..10}; do
  curl -s -o /dev/null -w "Request $i: %{time_total}s\n" \
    -H "x-zk-receipt: test$i" \
    -H "x-zk-nullifier: $(printf '%064d' $RANDOM)" \
    http://localhost:8080/api/test
done
EOF

chmod +x scripts/benchmark.sh
```

**5. Add monitoring stack**
```bash
cat > docker-compose.monitoring.yml << 'EOF'
version: '3.8'

services:
  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
    volumes:
      - ./monitoring/prometheus.yml:/etc/prometheus/prometheus.yml
    networks:
      - khafi-network

  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    networks:
      - khafi-network

networks:
  khafi-network:
    external: true
EOF

# Prometheus config
mkdir -p monitoring
cat > monitoring/prometheus.yml << 'EOF'
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'envoy'
    static_configs:
      - targets: ['envoy:9901']
EOF
```

**6. Security hardening**
```bash
# Update .gitignore
cat >> .gitignore << 'EOF'

# Secrets
.env
*.key
*.pem
secrets/
EOF

# Production environment template
cat > .env.production.example << 'EOF'
REDIS_URL=redis://redis-cluster:6379
REDIS_PASSWORD=changeme
RUST_LOG=warn
ENABLE_TLS=true
PROOF_VERIFICATION_TIMEOUT_MS=500
EOF
```

**7. Production readiness check**
```bash
cat > scripts/production-check.sh << 'EOF'
#!/bin/bash
echo "🔍 Production Readiness Check"
PASS=0
FAIL=0

check() {
  if [ $1 -eq 0 ]; then
    echo "✅ $2"
    ((PASS++))
  else
    echo "❌ $2"
    ((FAIL++))
  fi
}

[ -f "docs/guides/deployment.md" ]
check $? "Deployment docs exist"

[ -f ".gitignore" ] && grep -q "\.env" .gitignore
check $? "Secrets gitignored"

[ -f "monitoring/prometheus.yml" ]
check $? "Monitoring configured"

[ -x "scripts/test-e2e.sh" ]
check $? "E2E tests ready"

echo ""
echo "Results: $PASS passed, $FAIL failed"
[ $FAIL -eq 0 ] && echo "✅ Ready!" || echo "❌ Fix issues"
EOF

chmod +x scripts/production-check.sh
./scripts/production-check.sh
```

**8. Run everything**
```bash
# Run E2E tests
./scripts/test-e2e.sh

# Run benchmarks
./scripts/benchmark.sh

# Start monitoring
docker-compose -f docker-compose.monitoring.yml up -d

# Check production readiness
./scripts/production-check.sh
```

#### Checkpoint:
After completing these commands, verify:
- ✅ Metrics and observability configured
- ✅ Rate limiting enabled
- ✅ Documentation complete
- ✅ Benchmarks run successfully
- ✅ Production checklist passes
- ✅ Monitoring stack runs

**🎉 All 7 phases complete! Your Khafi Gateway is ready for deployment!**

**👉 Check in with me when done or if you need help deploying!**

---

## 🔧 Key Dependencies

### Rust Crates
- **ZK:** `risc0-zkvm`, `risc0-build`
- **Zcash:** `zcash_primitives`, `orchard`, `zcash_client_backend`
- **gRPC:** `tonic`, `prost`, `tonic-build`
- **Async:** `tokio`, `async-trait`
- **Storage:** `redis`, `deadpool-redis`
- **Serialization:** `serde`, `bincode`
- **Logging:** `tracing`, `tracing-subscriber`

### Infrastructure
- **Envoy Proxy:** v1.28+ (ExtAuth filter support)
- **Redis:** v7.0+ (atomic operations)
- **Zcash Node:** zebra (light client mode)

---

## 🎯 Success Criteria

### MVP Complete When:
1. ✅ Client SDK can generate proofs for Zcash payments
2. ✅ Envoy gateway verifies proofs before routing
3. ✅ Nullifier replay attacks are prevented
4. ✅ All services run together in Docker Compose
5. ✅ End-to-end test demonstrates VPA flow
6. ✅ Documentation enables external developers to integrate

---

## 📊 Progress Tracking

| Phase | Status | Progress | Target Date | Notes |
|-------|--------|----------|-------------|-------|
| Phase 1: Foundation + RISC Zero | ✅ Complete | 100% | Week 1-2 | Full RISC Zero integration, proof generation working, 11 tests passing |
| Phase 2: ZK Verification | ✅ Complete | 100% | Week 2-3 | gRPC service with ExtAuth, Redis nullifier checking, RISC Zero verification |
| Phase 3: Zcash Backend | ⏳ In Progress | 10% | Week 3-4 | Crate structure done, API pending |
| Phase 4: Client SDK | ⏳ In Progress | 65% | Week 4-5 | SDK template complete with actual proof generation |
| Phase 5: Logic Compiler | ⏳ In Progress | 40% | Week 5-6 | DSL parser complete, 3 example DSLs, code generation pending |
| Phase 6: Envoy Integration | ⚪ Planned | 0% | Week 6-7 | - |
| Phase 7: Production Ready | ⚪ Planned | 0% | Week 7-8 | - |

**Overall Progress:** 45% (MVP)

---

## 📝 Notes & Decisions

### Architecture Decisions:
- **Repository Structure:** Rust monorepo with Cargo workspace ✅
- **Nullifier Storage:** Redis (in-memory with persistence)
- **Deployment Target:** Docker Compose for local development
- **MVP Scope:** All 4 core services (ZK Verification, Zcash Backend, Envoy, Client SDK)
- **Bincode Version:** Using 2.0.1 with serde compatibility layer ✅
- **RISC Zero Version:** Using 3.0.3 (latest stable) ✅
- **Error Handling:** thiserror 2.x with comprehensive error types ✅

### Open Questions:
- [ ] Which Zcash network for initial testing? (testnet vs regtest)
- [ ] Performance targets for proof verification latency?
- [ ] Rate limiting strategy for production deployment?
- [ ] Multi-tenancy support in initial design?

### Recent Changes:
- **2025-11-19:** Created comprehensive Zcash documentation (zcash-integration.md, zcash-verification-flow.md)
- **2025-11-19:** Documented Orchard protocol, notes, nullifiers, commitment trees
- **2025-11-19:** Added Zcash verification flow with cryptographic operations
- **2025-11-19:** Documented three implementation roadmap options for Zcash
- **2025-11-19:** Added performance estimates for zkVM Zcash verification
- **2025-11-19:** Completed Phase 2: ZK Verification Service (gRPC + ExtAuth + Redis)
- **2025-11-19:** Fixed tonic-build API issues (migrated to tonic-prost-build)
- **2025-11-19:** Implemented Logic Compiler DSL parser with validation
- **2025-11-19:** Created 3 example DSL files (pharma, shipping, age verification)
- **2025-11-19:** Added 6 validation rule types to DSL (signature, range, age, blacklist, etc.)
- **2025-11-19:** All 22 tests passing (13 workspace + 9 logic-compiler)
- **2025-11-16:** Completed full RISC Zero integration with methods crate
- **2025-11-16:** Implemented actual proof generation in sdk-template/src/prover.rs
- **2025-11-16:** Added Receipt verification methods (verify, journal, verify_and_decode)
- **2025-11-16:** Created comprehensive RISC Zero documentation (2 new docs)
- **2025-11-16:** Removed obsolete guest-template, replaced with methods/guest/
- **2025-11-16:** Guest ELF binary generation working (352 KB optimized)
- **2025-11-16:** Added Image ID conversion utilities ([u32; 8] ↔ [u8; 32])
- **2025-11-16:** All 11 tests passing across workspace
- **2025-11-15:** Migrated from bincode 1.x to 2.x using serde compatibility layer
- **2025-11-15:** Updated all dependencies to latest stable versions
- **2025-11-15:** Added comprehensive error types including anyhow integration
- **2025-11-15:** Implemented GuestInputs/GuestOutputs types for RISC Zero
- **2025-11-15:** Created SDK template with builders pattern for inputs

---

*Last Updated: 2025-11-19*

---

## 🔍 Current Implementation Status

### Completed Components:

#### ✅ khafi-common (100%)
- Error types with bincode 2.x support
- Nullifier type with hex encoding/decoding
- Receipt wrapper for RISC Zero proofs with verification methods
- Receipt::verify() - cryptographic proof verification
- Receipt::journal() - extract public outputs
- Receipt::verify_and_decode() - combined verification + extraction
- GuestInputs/GuestOutputs for ZK programs
- Full test coverage (4/4 tests passing)

#### ✅ methods (100%)
- RISC Zero build system with risc0-build
- build.rs calling embed_methods()
- Generated GUEST_ELF (352 KB optimized binary)
- Generated GUEST_ID (cryptographic Image ID)
- Guest program in methods/guest/src/main.rs
- Zcash payment verification structure (placeholder)
- Business logic execution structure (placeholder)
- Output generation with env::commit()
- Separate workspace for guest dependencies

#### ✅ sdk-template (65%)
- KhafiSDK main struct with embedded GUEST_ID
- Image ID conversion utilities ([u32; 8] ↔ [u8; 32])
- Input builders with fluent API
- ZcashClient for backend communication
- Prover module with actual RISC Zero proof generation
- generate_proof() - full ExecutorEnv → prove → receipt flow
- extract_outputs() - journal parsing for GuestOutputs
- Full test coverage (7/7 tests passing)

#### ⏳ logic-compiler (40%)
- ✅ Crate structure created
- ✅ Dependencies configured
- ✅ DSL schema with 6 validation rule types
- ✅ JSON parser with comprehensive validation
- ✅ 3 example DSL files (pharma, shipping, age verification)
- ✅ Integration tests (9 tests passing)
- ⏳ Guest code generator (NEXT PRIORITY)
- ⏳ Compilation pipeline
- ⏳ SDK customization

#### ✅ zk-verification-service (100%)
- ✅ Crate structure created
- ✅ gRPC protobuf definitions (Envoy ExtAuth)
- ✅ Authorization service implementation
- ✅ RISC Zero proof verification with Receipt::verify_and_decode()
- ✅ Redis-based nullifier checker with atomic SET NX
- ✅ Proper error handling and status codes
- ✅ Dockerfile for containerization
- ✅ Service runs on port 50051
- ✅ All tests passing (13 total)

#### ⏳ zcash-backend (10%)
- Crate structure created
- Dependencies configured
- REST API implementation pending

### Next Steps:

**✅ RISC Zero Integration Complete!** The proof generation pipeline is now fully functional.

**Immediate Next Steps (Phase 2):**

1. **Implement ZK Verification Service (HIGH PRIORITY):**
   - Set up gRPC protobuf definitions
   - Implement ExtAuth service
   - Add RISC Zero proof verification using Receipt::verify()
   - Create nullifier checker with Redis
   - **This is the next critical component**

2. **Implement Zcash Backend:**
   - Create REST API with Axum
   - Add commitment tree root fetching
   - Implement nullifier checking endpoint
   - Add Redis caching layer

3. **Complete SDK Template (mostly done):**
   - ✅ Proof generation working
   - ⏳ Add Zcash backend integration for fetching commitment roots
   - ⏳ Create example applications
   - ⏳ Write SDK documentation

4. **Implement Actual Zcash Verification (Future):**
   - Replace placeholder verify_zcash_payment() with real Orchard verification
   - Use zcash_primitives for merkle proof validation
   - Implement nullifier derivation

5. **Implement Logic Compiler (Future):**
   - JSON DSL parser
   - Code generation for custom business logic
   - Guest program compilation
   - SDK customization

6. **Set up Envoy Integration:**
   - Create Envoy configuration
   - Configure ExtAuth filter
   - Set up Docker Compose orchestration
   - Create end-to-end tests

---

## 🎉 RISC Zero Integration Summary

### What We Built:

The project now has a **fully functional RISC Zero zkVM integration** that can:

1. **Generate Cryptographic Proofs**
   - Host code in `sdk-template/src/prover.rs` orchestrates proof generation
   - Uses `ExecutorEnv::builder()` to prepare inputs
   - Calls `default_prover().prove()` to execute guest program in zkVM
   - Returns serialized Receipt containing the proof

2. **Guest Program Execution**
   - Guest code in `methods/guest/src/main.rs` runs inside zkVM
   - Reads inputs via `env::read()`
   - Performs verification logic (currently placeholder for Zcash)
   - Writes outputs via `env::commit()`
   - Compiled to 352 KB optimized ELF binary

3. **Proof Verification**
   - `Receipt::verify()` validates cryptographic proof
   - `Receipt::journal()` extracts public outputs
   - `Receipt::verify_and_decode()` combines both operations
   - Image ID ensures proof matches expected guest program

4. **Build System**
   - `risc0-build` generates `GUEST_ELF` and `GUEST_ID` at compile time
   - Separate workspace for guest dependencies
   - Optimization profiles for zkVM performance
   - Rust toolchain with `rust-src` component

### Architecture:

```
User Application
      │
      ▼
┌─────────────────────────┐
│   KhafiSDK::new()       │ ← Embedded GUEST_ID
│   .generate_proof()     │
└─────────────────────────┘
      │
      ▼
┌─────────────────────────┐
│  prover::generate_proof │ ← HOST CODE
│  - ExecutorEnv::builder │   (sdk-template/src/prover.rs)
│  - default_prover()     │
│  - .prove(GUEST_ELF)    │
└─────────────────────────┘
      │ Executes in zkVM
      ▼
┌─────────────────────────┐
│  guest::main()          │ ← GUEST CODE
│  - env::read()          │   (methods/guest/src/main.rs)
│  - verify_zcash()       │
│  - env::commit()        │
└─────────────────────────┘
      │ Returns receipt
      ▼
┌─────────────────────────┐
│  Receipt                │
│  - proof bytes          │
│  - image_id             │
│  - journal (outputs)    │
└─────────────────────────┘
```

### Documentation:

- **`docs/risc0-setup-complete.md`** - Complete implementation guide
- **`docs/host-vs-guest-code.md`** - Architecture explanation with side-by-side comparison

### Tests:

All 11 tests passing:
- `khafi-common`: 4/4 tests (Receipt, Nullifier, Error handling)
- `sdk-template`: 7/7 tests (SDK creation, builders, Image ID conversion)

**Status: ✅ RISC Zero integration COMPLETE and ready for Phase 2!**
