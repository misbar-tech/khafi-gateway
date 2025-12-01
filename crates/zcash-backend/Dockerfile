# Build stage
FROM rust:1.85-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev

WORKDIR /app

# Copy workspace Cargo files
COPY Cargo.toml Cargo.lock ./

# Copy all workspace crates (needed for workspace resolution)
COPY crates ./crates

# Build the application
RUN cargo build --release -p zcash-backend

# Runtime stage
FROM alpine:latest

# Install runtime dependencies
RUN apk add --no-cache ca-certificates

# Create app user
RUN addgroup -S app && adduser -S app -G app

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/zcash-backend /usr/local/bin/zcash-backend

# Switch to app user
USER app

# Expose port
EXPOSE 8081

# Set default environment
ENV BACKEND_HOST=0.0.0.0
ENV BACKEND_PORT=8081
ENV RUST_LOG=info

CMD ["zcash-backend"]
