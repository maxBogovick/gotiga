# ── Build stage ────────────────────────────────────────────────────────────────
FROM rust:1.90-slim-bookworm AS builder

WORKDIR /usr/src/app
COPY . .

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

RUN cargo build --release

# ── Runtime stage ──────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends libssl3 ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/gotiga-server /usr/local/bin/gotiga-server
COPY --from=builder /usr/src/app/migrations ./migrations

RUN mkdir -p /app/uploads

ENV HOST=0.0.0.0
ENV PORT=3000
ENV RUST_LOG=info

EXPOSE 3000

CMD ["gotiga-server"]
