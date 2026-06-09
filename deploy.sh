#!/usr/bin/env bash
# Run on the remote Linux server.
# First time:  ./deploy.sh
# Update:      ./deploy.sh   (pulls new images, recreates containers)
set -euo pipefail

COMPOSE_FILE="${COMPOSE_FILE:-docker-compose.prod.yml}"

require_cmd() { command -v "$1" >/dev/null 2>&1 || { echo "ERROR: $1 not found"; exit 1; }; }
require_cmd docker

[[ -f "$COMPOSE_FILE" ]] || { echo "ERROR: $COMPOSE_FILE not found in $(pwd)"; exit 1; }
[[ -f ".env" ]]          || { echo "ERROR: .env not found — copy .env.example and fill it in"; exit 1; }

echo "[gotiga] Logging in to GHCR (anonymous pull — no token needed for public packages)"
echo "[gotiga] Pulling latest images..."
docker compose -f "$COMPOSE_FILE" pull

echo "[gotiga] Starting services..."
docker compose -f "$COMPOSE_FILE" up -d --remove-orphans

echo "[gotiga] Status:"
docker compose -f "$COMPOSE_FILE" ps
