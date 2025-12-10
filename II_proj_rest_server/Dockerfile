# Build stage
FROM rust:1.90 as builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build for release
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
# RUN apt-get update && apt-get install -y \
#     ca-certificates \
#     && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/II_proj_rest_server .

# Create directories for database files
RUN mkdir -p /app/db_data /data

# Expose port 8080
EXPOSE 8080

# Run the server
CMD ["./II_proj_rest_server"]