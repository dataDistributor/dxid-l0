# Minimal Dockerfile for Railway - focusing on getting it to work
FROM rust:1.74.0

# Set working directory
WORKDIR /app

# Copy the entire project
COPY . .

# Simple build attempt without verbose flags to reduce noise
RUN cargo build --release --package dxid-node

# Create data directory
RUN mkdir -p /app/dxid-data

# Expose the port
EXPOSE 8545

# Set the entrypoint
ENTRYPOINT ["./target/release/dxid-node", "--no-discovery"]
