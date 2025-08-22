# Use the official Rust image as a base
FROM rust:1.75-slim as builder

# Set working directory
WORKDIR /app

# Clear cargo cache to force clean build
RUN rm -rf /usr/local/cargo/registry/cache /usr/local/cargo/git/db

# Copy the entire project
COPY . .

# Add build timestamp to force rebuild
RUN echo "Build timestamp: $(date)" > /app/build-info.txt

# Build the release version with clean cache
RUN cargo clean && cargo build --release --package dxid-node

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
COPY --from=builder /app/build-info.txt /app/build-info.txt

# Create data directory
RUN mkdir -p /app/dxid-data && chown -R dxid:dxid /app

# Switch to non-root user
USER dxid

# Expose the port
EXPOSE 8545

# Set the entrypoint
ENTRYPOINT ["/app/dxid-node", "--no-discovery"]
