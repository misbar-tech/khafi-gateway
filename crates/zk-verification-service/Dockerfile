# Build stage
FROM rust:1.85-alpine AS builder

# Install build dependencies including protobuf compiler
RUN apk add --no-cache musl-dev protobuf-dev

WORKDIR /app

# Copy workspace Cargo files
COPY Cargo.toml Cargo.lock ./

# Copy all workspace crates (needed for workspace resolution)
COPY crates ./crates

# Build the zk-verification-service binary
# Skip RISC0 guest build since we use pre-built guest programs
ENV RISC0_SKIP_BUILD=1
RUN cargo build --release -p zk-verification-service

# Runtime stage
FROM alpine:latest

# Install runtime dependencies
RUN apk add --no-cache ca-certificates libgcc

# Create app user
RUN addgroup -S app && adduser -S app -G app

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/zk-verification-service /usr/local/bin/

# Switch to app user
USER app

# Expose gRPC port
EXPOSE 50051

# Set default environment
ENV RUST_LOG=info

# Run the service
CMD ["zk-verification-service"]
