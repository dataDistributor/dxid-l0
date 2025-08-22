# Explicit Dockerfile for Railway - force it to use this
FROM rust:1.76.0 as builder

# Install system dependencies that might be needed
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy the entire project
COPY . .

# Build just the dxid-node binary
RUN cargo build --release --bin dxid-node

# Create a minimal runtime image
FROM debian:bookworm-slim as runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /app/target/release/dxid-node /usr/local/bin/dxid-node

# Create data directory
RUN mkdir -p /app/dxid-data

# Set working directory
WORKDIR /app

# Expose the port
EXPOSE 8545

# Set the entrypoint
ENTRYPOINT ["/usr/local/bin/dxid-node", "--no-discovery"]
