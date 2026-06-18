# ── Build stage ────────────────────────────────────────────────────────────────
FROM rust:1.90-slim-bookworm AS builder

WORKDIR /usr/src/app
COPY . .

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

RUN cargo build --release

# ── GeoIP stage: optionally bake an offline IP→city database into the image ────
# DB-IP "City Lite" (MMDB format, CC-BY) needs no account/license key and is read
# by the maxminddb crate via the same geoip2 schema as MaxMind GeoLite2-City.
# Pinned to the build month, with a fallback to the previous month near the
# boundary (DB-IP publishes a new file at the start of each month).
#   --build-arg GEOIP_ENABLED=        → skip the DB (lighter image, geo disabled)
#   --build-arg GEOIP_MONTH=YYYY-MM   → pin a specific month
# Rebuild the image to refresh the data.
FROM debian:bookworm-slim AS geoip
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

# ── Runtime stage ──────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends libssl3 ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/gotiga-server /usr/local/bin/gotiga-server
COPY --from=builder /usr/src/app/migrations ./migrations
COPY --from=geoip /geoip/city.mmdb /app/geoip/city.mmdb

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

EXPOSE 3000

CMD ["gotiga-server"]
