#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SERVER_DIR="$ROOT_DIR/src-tauri/server"

DB_HOST="${DB_HOST:-127.0.0.1}"
DB_PORT="${DB_PORT:-5434}"
SERVER_HEALTH_URL="${SERVER_HEALTH_URL:-http://127.0.0.1:3000/api/v1/health}"
FRONTEND_URL="${FRONTEND_URL:-http://127.0.0.1:1420}"

SERVER_PID=""
FRONTEND_PID=""

log() {
  printf '[gotiga] %s\n' "$*"
}

fail() {
  printf '[gotiga] ERROR: %s\n' "$*" >&2
  exit 1
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "Required command not found: $1"
}

cleanup() {
  local code=$?
  trap - INT TERM EXIT

  if [[ -n "${FRONTEND_PID:-}" ]] && kill -0 "$FRONTEND_PID" 2>/dev/null; then
    log "Stopping frontend..."
    kill "$FRONTEND_PID" 2>/dev/null || true
  fi

  if [[ -n "${SERVER_PID:-}" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
    log "Stopping server..."
    kill "$SERVER_PID" 2>/dev/null || true
  fi

  if [[ -n "${FRONTEND_PID:-}" ]]; then
    wait "$FRONTEND_PID" 2>/dev/null || true
  fi

  if [[ -n "${SERVER_PID:-}" ]]; then
    wait "$SERVER_PID" 2>/dev/null || true
  fi

  if [[ "${STOP_DB_ON_EXIT:-0}" == "1" ]]; then
    log "Stopping Postgres..."
    (cd "$SERVER_DIR" && docker compose stop db >/dev/null)
  else
    log "Postgres remains running. Stop it with: cd src-tauri/server && docker compose down"
  fi

  exit "$code"
}

wait_for_port() {
  local host="$1"
  local port="$2"
  local attempts="${3:-60}"

  for _ in $(seq 1 "$attempts"); do
    if nc -z "$host" "$port" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done

  fail "Timed out waiting for ${host}:${port}"
}

wait_for_http() {
  local url="$1"
  local attempts="${2:-60}"

  for _ in $(seq 1 "$attempts"); do
    if curl -fsS "$url" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done

  fail "Timed out waiting for ${url}"
}

trap cleanup INT TERM EXIT

require_cmd cargo
require_cmd npm
require_cmd docker
require_cmd nc
require_cmd curl

[[ -d "$SERVER_DIR" ]] || fail "Server directory not found: $SERVER_DIR"
[[ -f "$SERVER_DIR/.env" ]] || fail "Server .env not found: $SERVER_DIR/.env"

log "Starting Postgres on ${DB_HOST}:${DB_PORT}..."
(cd "$SERVER_DIR" && docker compose up -d db)
wait_for_port "$DB_HOST" "$DB_PORT" 60
(cd "$SERVER_DIR" && docker compose stop app >/dev/null 2>&1 || true)

if [[ ! -d "$ROOT_DIR/node_modules" ]]; then
  log "Installing frontend dependencies..."
  (cd "$ROOT_DIR" && npm install)
fi

log "Starting Rust API server..."
(cd "$SERVER_DIR" && cargo run) &
SERVER_PID=$!
wait_for_http "$SERVER_HEALTH_URL" 90

log "Starting Vite frontend..."
(cd "$ROOT_DIR" && npm run dev) &
FRONTEND_PID=$!

log "Server: ${SERVER_HEALTH_URL}"
log "Frontend: ${FRONTEND_URL}"
log "Press Ctrl+C to stop app processes."

while true; do
  if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    wait "$SERVER_PID"
    exit $?
  fi

  if ! kill -0 "$FRONTEND_PID" 2>/dev/null; then
    wait "$FRONTEND_PID"
    exit $?
  fi

  sleep 1
done
