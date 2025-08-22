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

# Build the release version
RUN cargo build --release --package dxid-node

# Create data directory
RUN mkdir -p /app/dxid-data

# Expose the port
EXPOSE 8545

# Set the entrypoint
ENTRYPOINT ["./target/release/dxid-node", "--no-discovery"]
