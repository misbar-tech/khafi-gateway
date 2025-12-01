# Build stage
FROM rust:1.85-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig

WORKDIR /app

# Copy workspace Cargo files
COPY Cargo.toml Cargo.lock ./

# Copy all workspace crates (needed for workspace resolution)
COPY crates ./crates

# Build the application
# Skip RISC0 guest build since we use pre-built guest programs
ENV RISC0_SKIP_BUILD=1
RUN cargo build --release -p proof-generation-service

# Runtime stage
FROM alpine:latest

# Install runtime dependencies
RUN apk add --no-cache ca-certificates libgcc

# Create app user
RUN addgroup -S app && adduser -S app -G app

# Create directories for deployments
RUN mkdir -p /app/deployments && chown -R app:app /app

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/proof-generation-service /usr/local/bin/proof-generation-service

# Switch to app user
USER app

# Expose port
EXPOSE 8084

# Set default environment
ENV PROVER_HOST=0.0.0.0
ENV PROVER_PORT=8084
ENV RUST_LOG=info

CMD ["proof-generation-service"]
