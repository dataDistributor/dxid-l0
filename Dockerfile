# Use the official Rust image as a base
FROM rust:1.75-slim as builder

# Set working directory
WORKDIR /app

# Copy the entire project
COPY . .

# Build the release version
RUN cargo build --release --package dxid-node

# Create a new stage with a minimal runtime image
FROM debian:bookworm-slim

# Install necessary runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN useradd -m -u 1000 dxid

# Set working directory
WORKDIR /app

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/dxid-node /app/dxid-node

# Create data directory
RUN mkdir -p /app/dxid-data && chown -R dxid:dxid /app

# Switch to non-root user
USER dxid

# Expose the port
EXPOSE 8545

# Set the entrypoint
ENTRYPOINT ["/app/dxid-node", "--no-discovery"]
