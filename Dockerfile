# Simple single-stage Dockerfile for Railway
FROM rust:1.74.0

# Set working directory
WORKDIR /app

# Copy the entire project
COPY . .

# Show Rust and Cargo versions for debugging
RUN rustc --version && cargo --version

# Show directory contents for debugging
RUN ls -la

# Check workspace structure
RUN ls -la dxid-node/

# Show dependencies
RUN cargo tree --package dxid-node || echo "cargo tree failed"

# Build the release version with verbose output
RUN cargo build --release --package dxid-node --verbose

# Create data directory
RUN mkdir -p /app/dxid-data

# Expose the port
EXPOSE 8545

# Set the entrypoint
ENTRYPOINT ["./target/release/dxid-node", "--no-discovery"]
