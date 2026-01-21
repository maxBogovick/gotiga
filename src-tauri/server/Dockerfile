# Builder Stage
FROM rust:1.75-slim-bookworm as builder

WORKDIR /usr/src/app
COPY . .

# Install dependencies (if needed, e.g. libssl-dev)
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Build Release
RUN cargo build --release

# Runtime Stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy binary
COPY --from=builder /usr/src/app/target/release/gotiga-server /usr/local/bin/gotiga-server

# Create upload directory
RUN mkdir -p /app/uploads

# Env vars (defaults)
ENV HOST=0.0.0.0
ENV PORT=3000
ENV RUST_LOG=info

EXPOSE 3000

CMD ["gotiga-server"]
