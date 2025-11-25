# Build stage
FROM rust:1.85-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates/common ./crates/common
COPY crates/logic-compiler ./crates/logic-compiler
COPY crates/logic-compiler-api ./crates/logic-compiler-api

# Build the application
RUN cargo build --release -p logic-compiler-api

# Runtime stage
FROM alpine:latest

# Install runtime dependencies
RUN apk add --no-cache ca-certificates

# Create app user
RUN addgroup -S app && adduser -S app -G app

# Create directories
RUN mkdir -p /app/output/sdks /app/templates && chown -R app:app /app

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/logic-compiler-api /usr/local/bin/logic-compiler-api

# Copy example templates
COPY docs/examples/*.json /app/templates/

# Switch to app user
USER app

# Expose port
EXPOSE 8082

# Set default environment
ENV API_HOST=0.0.0.0
ENV API_PORT=8082
ENV SDK_OUTPUT_DIR=/app/output/sdks
ENV TEMPLATES_DIR=/app/templates
ENV RUST_LOG=info

CMD ["logic-compiler-api"]
