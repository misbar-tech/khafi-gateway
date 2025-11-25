# Logic Compiler API

REST API service for validating, compiling, and deploying custom business logic DSL rules. This service wraps the `logic-compiler` library and provides HTTP endpoints for SaaS customers to create custom zero-knowledge validation rules without requiring a local Rust development environment.

## Features

- **DSL Validation** - Validate business rules DSL without compiling
- **Code Compilation** - Compile DSL to RISC Zero guest programs
- **SDK Generation** - Generate complete SDK packages with build configuration
- **Template Management** - Browse and use pre-built validation rule templates
- **CORS Enabled** - Ready for frontend integration

## Quick Start

### Local Development

1. **Run the service:**
   ```bash
   cargo run -p logic-compiler-api
   ```

2. **Test the health endpoint:**
   ```bash
   curl http://localhost:8082/health
   ```

### Docker Deployment

1. **Build and start with Docker Compose:**
   ```bash
   docker compose up -d logic-compiler-api
   ```

2. **Check logs:**
   ```bash
   docker compose logs -f logic-compiler-api
   ```

## API Endpoints

### Health Check

```bash
GET /health
```

Returns service health status.

**Response:**
```json
{
  "status": "healthy",
  "service": "logic-compiler-api"
}
```

### Validate DSL

```bash
POST /api/validate
Content-Type: application/json
```

Validates a business rules DSL without compiling.

**Request Body:**
```json
{
  "dsl": {
    "use_case": "age_verification",
    "description": "Verify user is 18 or older",
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
}
```

**Response (Success):**
```json
{
  "valid": true,
  "error": null,
  "parsed_dsl": { ... }
}
```

**Response (Validation Failed):**
```json
{
  "valid": false,
  "error": "At least one validation rule is required",
  "parsed_dsl": null
}
```

### Compile DSL

```bash
POST /api/compile
Content-Type: application/json
```

Compiles DSL to Rust guest program code.

**Request Body:** Same as `/api/validate`

**Response (Success):**
```json
{
  "success": true,
  "code": "// RISC Zero Guest Program\n#![no_main]\nrisc0_zkvm::guest::entry!(main);\n...",
  "error": null
}
```

**Response (Compilation Failed):**
```json
{
  "success": false,
  "code": null,
  "error": "Code generation failed: ..."
}
```

### Generate SDK Package

```bash
POST /api/sdk/generate
Content-Type: application/json
```

Generates a complete SDK package with guest program, build configuration, and Cargo workspace.

**Request Body:** Same as `/api/validate`

**Response (Success):**
```json
{
  "success": true,
  "sdk_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "error": null
}
```

**Response (Generation Failed):**
```json
{
  "success": false,
  "sdk_id": null,
  "error": "SDK generation failed: ..."
}
```

### Download SDK Package

```bash
GET /api/sdk/download/{sdk_id}
```

Downloads the generated SDK package as a `.tar.gz` archive.

**Response:** Binary tarball file

**Example:**
```bash
curl -O http://localhost:8082/api/sdk/download/a1b2c3d4-e5f6-7890-abcd-ef1234567890
```

### List Templates

```bash
GET /api/templates
```

Returns a list of available validation rule templates.

**Response:**
```json
{
  "templates": [
    {
      "name": "age-verification-simple",
      "title": "age_verification",
      "description": "Verify user is 18 or older",
      "category": "Identity Verification"
    },
    {
      "name": "pharma-rules",
      "title": "prescription_validation",
      "description": "Validate pharmaceutical prescription compliance",
      "category": "Healthcare"
    }
  ]
}
```

### Get Template

```bash
GET /api/templates/{name}
```

Returns the full DSL specification for a specific template.

**Example:**
```bash
curl http://localhost:8082/api/templates/age-verification-simple
```

**Response:** Complete DSL JSON object

## Configuration

The service is configured via environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `API_HOST` | Server bind address | `0.0.0.0` |
| `API_PORT` | Server port | `8082` |
| `SDK_OUTPUT_DIR` | Directory for generated SDKs | `./output/sdks` |
| `TEMPLATES_DIR` | Directory containing templates | `./docs/examples` |
| `RUST_LOG` | Logging level | `info` |

### Environment File

Copy `.env.example` to `.env` and customize:

```bash
cp .env.example .env
```

## Development

### Running Tests

```bash
# Run all tests
cargo test -p logic-compiler-api

# Run only unit tests
cargo test -p logic-compiler-api --lib

# Run only integration tests
cargo test -p logic-compiler-api --test integration_test
```

### Project Structure

```
crates/logic-compiler-api/
├── src/
│   ├── main.rs           # Application entry point
│   ├── lib.rs            # Router setup
│   ├── config.rs         # Configuration management
│   └── handlers.rs       # API request handlers
├── tests/
│   └── integration_test.rs  # Integration tests
├── Dockerfile            # Docker build configuration
├── .env.example          # Environment template
└── README.md             # This file
```

## Usage Examples

### Example: Validate DSL with cURL

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

### Example: Generate and Download SDK

```bash
# Step 1: Generate SDK
RESPONSE=$(curl -X POST http://localhost:8082/api/sdk/generate \
  -H "Content-Type: application/json" \
  -d @age-verification-simple.json)

# Step 2: Extract SDK ID
SDK_ID=$(echo $RESPONSE | jq -r '.sdk_id')

# Step 3: Download SDK
curl -O http://localhost:8082/api/sdk/download/$SDK_ID

# Step 4: Extract and use
tar -xzf $SDK_ID.tar.gz
cd $SDK_ID
cargo build --release
```

### Example: List and Use Templates

```bash
# List available templates
curl http://localhost:8082/api/templates

# Get a specific template
curl http://localhost:8082/api/templates/age-verification-simple > my-dsl.json

# Compile the template
curl -X POST http://localhost:8082/api/compile \
  -H "Content-Type: application/json" \
  -d "{\"dsl\": $(cat my-dsl.json)}"
```

## Error Handling

All endpoints return appropriate HTTP status codes:

- `200 OK` - Request successful
- `400 Bad Request` - Invalid JSON or DSL
- `404 Not Found` - Template or SDK not found
- `500 Internal Server Error` - Server-side error

Error responses include a descriptive message:

```json
{
  "error": "Detailed error message here"
}
```

## Integration with Frontend

The API is CORS-enabled and ready for frontend integration. Example JavaScript usage:

```javascript
// Validate DSL
async function validateDSL(dsl) {
  const response = await fetch('http://localhost:8082/api/validate', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ dsl }),
  });

  return await response.json();
}

// Compile DSL
async function compileDSL(dsl) {
  const response = await fetch('http://localhost:8082/api/compile', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ dsl }),
  });

  return await response.json();
}

// Generate SDK
async function generateSDK(dsl) {
  const response = await fetch('http://localhost:8082/api/sdk/generate', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ dsl }),
  });

  const result = await response.json();

  if (result.success) {
    // Download SDK
    window.location.href = `http://localhost:8082/api/sdk/download/${result.sdk_id}`;
  }

  return result;
}
```

## Troubleshooting

### Service won't start

**Problem:** Port 8082 already in use

**Solution:** Either stop the other service or change `API_PORT`:
```bash
API_PORT=8083 cargo run -p logic-compiler-api
```

### Templates not found

**Problem:** Template directory doesn't exist

**Solution:** Ensure `TEMPLATES_DIR` points to the correct location:
```bash
export TEMPLATES_DIR=../../docs/examples
cargo run -p logic-compiler-api
```

### SDK generation fails

**Problem:** Output directory not writable

**Solution:** Create the directory with proper permissions:
```bash
mkdir -p ./output/sdks
chmod 755 ./output/sdks
```

## Architecture

```
┌──────────────┐
│   Frontend   │
│  (React UI)  │
└──────┬───────┘
       │ HTTP/REST
       ▼
┌──────────────────┐
│ Logic Compiler   │
│      API         │
├──────────────────┤
│  • Validate      │
│  • Compile       │
│  • Generate SDK  │
│  • Templates     │
└──────┬───────────┘
       │
       ▼
┌──────────────────┐
│ Logic Compiler   │
│    (Library)     │
├──────────────────┤
│  • DSL Parser    │
│  • Code Gen      │
│  • Type Gen      │
│  • Validation    │
└──────────────────┘
```

## Security Considerations

1. **Input Validation** - All DSL inputs are validated before compilation
2. **Resource Limits** - Consider adding rate limiting for production
3. **SDK Storage** - Generated SDKs should be cleaned up periodically
4. **CORS** - Currently permissive; restrict origins in production
5. **File Access** - Service runs as non-root user in Docker

## Performance

- DSL validation: ~5-10ms
- Code compilation: ~50-100ms
- SDK generation: ~100-200ms
- Tarball creation: ~50-100ms

## License

MIT

## Contributing

See the main [Khafi Gateway README](../../README.md) for contribution guidelines.
