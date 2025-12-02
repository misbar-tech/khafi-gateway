# Build Service Dockerfile
# This image includes the full RISC Zero toolchain for building guest programs

# Build stage
FROM rust:1.85 AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install rzup (RISC Zero toolchain manager)
RUN curl -L https://risczero.com/install | bash
ENV PATH="/root/.risc0/bin:${PATH}"

# Install RISC Zero Rust toolchain
RUN rzup install rust
RUN rzup install cpp

WORKDIR /app

# Copy workspace Cargo files
COPY Cargo.toml Cargo.lock ./

# Copy all workspace crates
COPY crates ./crates

# Build the build-service (this doesn't need guest programs)
ENV RISC0_SKIP_BUILD=1
RUN cargo build --release -p build-service

# Runtime stage - includes RISC Zero toolchain for runtime builds
FROM rust:1.85-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install rzup (RISC Zero toolchain manager)
RUN curl -L https://risczero.com/install | bash
ENV PATH="/root/.risc0/bin:${PATH}"

# Install RISC Zero Rust toolchain
RUN rzup install rust
RUN rzup install cpp

# Create directories
RUN mkdir -p /app/builds

WORKDIR /app

# Copy the build-service binary
COPY --from=builder /app/target/release/build-service /usr/local/bin/build-service

# Copy the logic-compiler crate (needed for code generation at runtime)
COPY --from=builder /app/crates/logic-compiler /app/crates/logic-compiler
COPY --from=builder /app/crates/common /app/crates/common
COPY --from=builder /app/crates/sdk-template /app/crates/sdk-template

# Expose port
EXPOSE 8085

# Environment variables
ENV BUILD_HOST=0.0.0.0
ENV BUILD_PORT=8085
ENV BUILD_DIR=/app/builds
ENV RUST_LOG=info

CMD ["build-service"]
