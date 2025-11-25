# khafi-gateway
Khafi Gateway, A privacy-preserving API Gateway enabled by RISC0 and ZCash

## Quick Start Guide

This guide will help you get the complete system running in minutes.

## System Overview

Khafi Gateway consists of:

1. **Logic Compiler API** - REST API for DSL validation and SDK generation (Port 8082)
2. **Frontend UI** - React-based web interface for creating rules (Port 3000)
3. **Zcash Backend** - Payment verification service (Port 8081)
4. **ZK Verification Service** - RISC Zero proof verification (Port 50051)
5. **Redis** - Data storage (Port 6379)

## Prerequisites

- **Rust** 1.75+ (for backend services)
- **Node.js** 18+ (for frontend)
- **Docker & Docker Compose** (optional, for containerized deployment)

## Option 1: Quick Start with Terminal

### 1. Start Backend Services

In one terminal, start the Logic Compiler API:

```bash
cargo run -p logic-compiler-api
```

In another terminal, start the Zcash Backend (optional, for payment features):

```bash
cargo run -p zcash-backend
```

### 2. Start Frontend

In a third terminal:

```bash
cd frontend
npm install    # First time only
npm run dev
```

### 3. Open Your Browser

Navigate to: **http://localhost:3000**

You should see the Khafi Logic Compiler UI with three panels:
- Left: Template Gallery
- Center: DSL Editor
- Right: Live Preview

## Option 2: Docker Compose (Full Stack)

Start all services at once:

```bash
docker compose up -d
```

Check service status:

```bash
docker compose ps
```

View logs:

```bash
docker compose logs -f logic-compiler-api
```

Access the services:
- **Frontend UI**: http://localhost:3000 (Coming soon in Docker)
- **Logic Compiler API**: http://localhost:8082
- **Zcash Backend**: http://localhost:8081
- **ZK Verification**: http://localhost:50051

## Using the UI

### Step 1: Select a Template

1. Browse the Template Gallery on the left
2. Click on any template (e.g., "age-verification-simple")
3. The DSL will load into the editor

### Step 2: Edit the DSL

1. Modify the JSON in the center editor
2. Use the **Format** button to prettify the JSON
3. Watch the validation status update in real-time

### Step 3: Validate

The DSL is validated automatically as you type. Check the right panel for:
- ✅ **Valid** - Green checkmark with rule count
- ❌ **Invalid** - Red X with error details

### Step 4: Compile (Optional)

Click **"Compile to Code"** to see the generated Rust guest program.

### Step 5: Generate SDK

Click **"Generate & Download SDK"** to:
1. Generate a complete SDK package
2. Automatically download it as a `.tar.gz` file

### Step 6: Use Your SDK

Extract and build the SDK:

```bash
tar -xzf <sdk-id>.tar.gz
cd <sdk-id>
cargo build --release
```

Your custom validation logic is now ready to use!

## API Testing (Without UI)

Test the API directly with curl:

### Health Check

```bash
curl http://localhost:8082/health
```

### List Templates

```bash
curl http://localhost:8082/api/templates
```

### Validate DSL

```bash
curl -X POST http://localhost:8082/api/validate \
  -H "Content-Type: application/json" \
  -d '{
    "dsl": {
      "use_case": "age_verification",
      "description": "Simple age check",
      "version": "1.0",
      "private_inputs": {
        "user_data": {
          "type": "object",
          "fields": {
            "date_of_birth": "string"
          }
        }
      },
      "public_params": {
        "min_age": "u32"
      },
      "validation_rules": [
        {
          "type": "age_verification",
          "description": "Check minimum age",
          "dob_field": "date_of_birth",
          "min_age": 18
        }
      ]
    }
  }'
```

## Example Workflow

Here's a complete example of creating a custom validation rule:

### 1. Create Age Verification Rule

Load the age-verification template and modify it:

```json
{
  "use_case": "age_verification",
  "description": "Verify user is 21 or older for alcohol purchase",
  "version": "1.0",
  "private_inputs": {
    "user": {
      "type": "object",
      "fields": {
        "date_of_birth": "string",
        "user_id": "string"
      }
    }
  },
  "public_params": {
    "min_age": "u32"
  },
  "validation_rules": [
    {
      "type": "age_verification",
      "description": "Must be 21+ for alcohol",
      "dob_field": "date_of_birth",
      "min_age": 21
    }
  ],
  "outputs": {
    "compliance_result": "bool"
  }
}
```

### 2. Validate in UI

- Status shows: ✅ Valid
- Use case: age_verification
- Rules: 1

### 3. Generate SDK

Click "Generate & Download SDK" → Downloads `<uuid>.tar.gz`

### 4. Build SDK

```bash
tar -xzf a1b2c3d4-e5f6-7890-abcd-ef1234567890.tar.gz
cd a1b2c3d4-e5f6-7890-abcd-ef1234567890
cargo build --release
```

### 5. Use in Your App

```rust
use age_verification_methods::AGE_VERIFICATION_ID;
use risc0_zkvm::{default_prover, ExecutorEnv};

// Your private data
let user_data = UserData {
    date_of_birth: "1995-06-15".to_string(),
    user_id: "user123".to_string(),
};

// Public parameters
let min_age = 21u32;

// Generate proof
let env = ExecutorEnv::builder()
    .write(&user_data).unwrap()
    .write(&min_age).unwrap()
    .build()
    .unwrap();

let receipt = default_prover().prove(env, AGE_VERIFICATION_ID).unwrap();

// Verify proof
receipt.verify(AGE_VERIFICATION_ID).unwrap();

// Extract result
let result: bool = receipt.journal.decode().unwrap();
println!("Age verification: {}", result);
```

## Troubleshooting

### "Connection refused" errors

**Problem:** Cannot connect to API

**Solution:**
1. Ensure API is running: `cargo run -p logic-compiler-api`
2. Check port 8082 is not in use: `lsof -i :8082`
3. Verify VITE_API_URL in `frontend/.env`

### "Template not found" errors

**Problem:** Templates not loading

**Solution:**
1. Check TEMPLATES_DIR environment variable
2. Verify `docs/examples/` directory exists
3. Ensure template JSON files are present

### Frontend won't start

**Problem:** `npm run dev` fails

**Solution:**
```bash
cd frontend
rm -rf node_modules package-lock.json
npm install
npm run dev
```

### Monaco Editor not loading

**Problem:** Editor shows blank

**Solution:**
1. Clear browser cache
2. Open DevTools (F12) and check for errors
3. Verify `@monaco-editor/react` is installed

## Next Steps

Now that you have the system running:

1. **Explore Templates** - Try pharma-rules, shipping-rules
2. **Create Custom Rules** - Modify templates for your use case
3. **Read the Docs** - Check `/docs` for detailed guides
4. **Review Examples** - See `/docs/examples` for DSL patterns

## Service URLs Summary

| Service | Local URL | Purpose |
|---------|-----------|---------|
| Frontend UI | http://localhost:3000 | Web interface |
| Logic Compiler API | http://localhost:8082 | DSL compilation |
| Zcash Backend | http://localhost:8081 | Payment verification |
| ZK Verification | localhost:50051 | Proof verification (gRPC) |
| Redis | localhost:6379 | Data storage |

## Documentation

- [Logic Compiler API](crates/logic-compiler-api/README.md)
- [Frontend UI](frontend/README.md)
- [Zcash Backend](crates/zcash-backend/README.md)
- [Architecture](docs/ARCHITECTURE.md)

## Getting Help

- **GitHub Issues**: https://github.com/misbar/khafi-gateway/issues
- **Documentation**: Check `/docs` directory
- **Examples**: See `/docs/examples`

---

**Happy Building! 🚀**
