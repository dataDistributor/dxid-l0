# Minimal Dockerfile for Railway - focusing on getting it to work
FROM rust:1.74.0

# Set working directory
WORKDIR /app

# Copy the entire project
COPY . .

# Try building the entire workspace first to see if the issue is package-specific
RUN cargo build --release

# Create data directory
RUN mkdir -p /app/dxid-data

# Expose the port
EXPOSE 8545

# Set the entrypoint
ENTRYPOINT ["./target/release/dxid-node", "--no-discovery"]
