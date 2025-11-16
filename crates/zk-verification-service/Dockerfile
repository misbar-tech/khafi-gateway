FROM rust:1.75 as builder

WORKDIR /app

# Copy the entire workspace
COPY . .

# Build the zk-verification-service binary
RUN cargo build --release --bin zk-verification-service

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /app/target/release/zk-verification-service /usr/local/bin/

# Expose gRPC port
EXPOSE 50051

# Run the service
CMD ["zk-verification-service"]
