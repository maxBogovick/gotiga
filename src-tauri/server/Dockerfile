# syntax=docker/dockerfile:1
# ── Builder: native-arch cross-compile + dependency caching ────────────────────
# Pinned to $BUILDPLATFORM so the Rust compiler runs on the build host's NATIVE
# arch (e.g. arm64 on Apple Silicon) instead of under QEMU emulation, then
# cross-compiles to x86_64 (the deploy server is amd64). cargo-chef caches the
# dependency build in its own layer, so editing app source no longer recompiles
# every crate. BuildKit cache mounts persist the cargo registry + target dir
# across builds. The project is rustls-only (no OpenSSL), so the cross toolchain
# is just the GNU x86_64 cross-gcc — no C TLS libraries to wrangle.
FROM --platform=$BUILDPLATFORM rust:1.90-slim-bookworm AS chef
WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends gcc-x86-64-linux-gnu g++-x86-64-linux-gnu \
    && rm -rf /var/lib/apt/lists/* \
    && rustup target add x86_64-unknown-linux-gnu \
    && cargo install cargo-chef --locked
ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc \
    CC_x86_64_unknown_linux_gnu=x86_64-linux-gnu-gcc \
    CXX_x86_64_unknown_linux_gnu=x86_64-linux-gnu-g++ \
    AR_x86_64_unknown_linux_gnu=x86_64-linux-gnu-ar

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build & cache dependencies only — this layer is reused until Cargo.toml/lock change.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo chef cook --release --target x86_64-unknown-linux-gnu --recipe-path recipe.json
# Now the app itself. target/ is a cache mount, so copy the binary out of it.
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --target x86_64-unknown-linux-gnu \
    && cp target/x86_64-unknown-linux-gnu/release/gotiga-server /gotiga-server \
    && x86_64-linux-gnu-strip /gotiga-server

# ── GeoIP stage: optionally bake an offline IP→city database into the image ────
# DB-IP "City Lite" (MMDB format, CC-BY) needs no account/license key and is read
# by the maxminddb crate via the same geoip2 schema as MaxMind GeoLite2-City.
# Pinned to the build month, with a fallback to the previous month near the
# boundary (DB-IP publishes a new file at the start of each month).
#   --build-arg GEOIP_ENABLED=        → skip the DB (lighter image, geo disabled)
#   --build-arg GEOIP_MONTH=YYYY-MM   → pin a specific month
# Built on $BUILDPLATFORM (the .mmdb is arch-independent data) to avoid emulation.
FROM --platform=$BUILDPLATFORM debian:bookworm-slim AS geoip
ARG GEOIP_ENABLED=true
ARG GEOIP_MONTH
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /geoip
RUN set -eu; \
    if [ -n "$GEOIP_ENABLED" ]; then \
        this_month="${GEOIP_MONTH:-$(date -u +%Y-%m)}"; \
        prev_month="$(date -u -d "${this_month}-01 -1 month" +%Y-%m)"; \
        for m in "$this_month" "$prev_month"; do \
            url="https://download.db-ip.com/free/dbip-city-lite-$m.mmdb.gz"; \
            echo "Fetching GeoIP database: $url"; \
            if curl -fsSL --retry 3 --retry-delay 2 "$url" -o city.mmdb.gz; then break; fi; \
        done; \
        test -f city.mmdb.gz || { echo "ERROR: could not download DB-IP City Lite"; exit 1; }; \
        gunzip city.mmdb.gz; \
        test -s city.mmdb; \
    else \
        echo "GEOIP_ENABLED empty — building WITHOUT a geo database"; \
        : > city.mmdb; \
    fi

# ── Depth-Anything-V2 weights ("Living Daguerreotype" depth maps) ──────────────
# Baked in so the in-process candle inference needs no network at runtime. Two
# safetensors: DINOv2-small backbone + the Depth-Anything head (~150 MB total).
# Build with DEPTH_ENABLED="" to skip — the depth endpoint then disables itself.
FROM --platform=$BUILDPLATFORM debian:bookworm-slim AS depthweights
ARG DEPTH_ENABLED=true
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /depth-weights
RUN set -eu; \
    if [ -n "$DEPTH_ENABLED" ]; then \
        curl -fsSL --retry 3 --retry-delay 2 \
            "https://huggingface.co/lmz/candle-dino-v2/resolve/main/dinov2_vits14.safetensors" \
            -o dinov2_vits14.safetensors; \
        curl -fsSL --retry 3 --retry-delay 2 \
            "https://huggingface.co/jeroenvlek/depth-anything-v2-safetensors/resolve/main/depth_anything_v2_vits.safetensors" \
            -o depth_anything_v2_vits.safetensors; \
        test -s dinov2_vits14.safetensors && test -s depth_anything_v2_vits.safetensors; \
    else \
        echo "DEPTH_ENABLED empty — building WITHOUT depth weights (endpoint disabled)"; \
    fi

# ── multilingual-e5-small weights (semantic search, "Хранитель") ───────────────
# On-device multilingual text embeddings (candle/XLM-RoBERTa). Three files:
# model.safetensors (~470 MB f32) + tokenizer.json + config.json. Baked in so the
# in-process inference needs no network at runtime. Build with EMBED_ENABLED="" to
# skip — the /search endpoint then returns empty (feature disables itself).
FROM --platform=$BUILDPLATFORM debian:bookworm-slim AS embedweights
ARG EMBED_ENABLED=true
ARG EMBED_MODEL_REPO=intfloat/multilingual-e5-small
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /embed-weights
RUN set -eu; \
    if [ -n "$EMBED_ENABLED" ]; then \
        base="https://huggingface.co/${EMBED_MODEL_REPO}/resolve/main"; \
        for f in config.json tokenizer.json model.safetensors; do \
            curl -fsSL --retry 3 --retry-delay 2 "$base/$f" -o "$f"; \
        done; \
        test -s config.json && test -s tokenizer.json && test -s model.safetensors; \
    else \
        echo "EMBED_ENABLED empty — building WITHOUT embedding weights (/search disabled)"; \
    fi

# ── Runtime stage (target arch) ────────────────────────────────────────────────
FROM debian:bookworm-slim

WORKDIR /app

# No libssl: the binary is rustls-only. curl is needed for the compose healthcheck.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /gotiga-server /usr/local/bin/gotiga-server
COPY migrations ./migrations
COPY --from=geoip /geoip/city.mmdb /app/geoip/city.mmdb
COPY --from=depthweights /depth-weights /app/depth-weights
COPY --from=embedweights /embed-weights /app/embed-weights

# Run as an unprivileged user; own the uploads dir it needs to write.
RUN useradd --system --uid 10001 --user-group --no-create-home app \
    && mkdir -p /app/uploads \
    && chown -R app:app /app

USER app

ENV HOST=0.0.0.0
ENV PORT=3000
ENV RUST_LOG=info
# Offline geolocation DB baked above; override to a mounted file if you prefer
# fresher data without rebuilding.
ENV GEOIP_DB_PATH=/app/geoip/city.mmdb
# Bundled Depth-Anything weights for on-demand depth maps (admin "Generate depth").
ENV DEPTH_WEIGHTS_DIR=/app/depth-weights
# Bundled multilingual-e5-small weights for semantic search ("Хранитель").
# EMBED_MODEL_NAME must match the repo baked above so stored vectors stay valid.
ENV EMBED_WEIGHTS_DIR=/app/embed-weights
ENV EMBED_MODEL_NAME=intfloat/multilingual-e5-small

EXPOSE 3000

CMD ["gotiga-server"]
