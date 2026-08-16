#!/usr/bin/env bash
# Local site: Postgres (Docker :5434) + Rust API (:3000) + Vite (:1420).
#
#   ./start_all.sh
#   ./start_all.sh --restore                         # latest deploy/backups archive
#   ./start_all.sh --restore path/to/gotiga-backup-*.tar.gz
#   ./start_all.sh --restore --yes                   # skip the restore confirm prompt
#
# --restore loads a backup from the panel («Бэкап → на мой Mac») into this
# local stack, then boots as usual. Overwrites local DB + src-tauri/server/uploads.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SERVER_DIR="$ROOT_DIR/src-tauri/server"

DB_HOST="${DB_HOST:-127.0.0.1}"
DB_PORT="${DB_PORT:-5434}"
SERVER_PORT="${SERVER_PORT:-3000}"
FRONTEND_PORT="${FRONTEND_PORT:-1420}"
SERVER_HEALTH_URL="${SERVER_HEALTH_URL:-http://127.0.0.1:${SERVER_PORT}/api/v1/health}"
FRONTEND_URL="${FRONTEND_URL:-http://127.0.0.1:${FRONTEND_PORT}}"

SERVER_PID=""
FRONTEND_PID=""
RESTORE_ARCHIVE=""
RESTORE_YES=""

log() {
  printf '[gotiga] %s\n' "$*"
}

fail() {
  printf '[gotiga] ERROR: %s\n' "$*" >&2
  exit 1
}

usage() {
  cat <<EOF
Usage: $0 [--restore [archive]] [--yes]

  --restore [archive]  Load a panel backup into local Postgres + uploads, then boot.
                       With no path, uses the newest deploy/backups/*/gotiga-backup-*.tar.gz
  --yes                Skip the restore confirmation prompt
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    --yes)
      RESTORE_YES=1
      shift
      ;;
    --restore)
      if [[ "${2:-}" != "" && "${2:-}" != --* ]]; then
        RESTORE_ARCHIVE="$2"
        shift 2
      else
        RESTORE_ARCHIVE="__latest__"
        shift
      fi
      ;;
    *)
      fail "Unknown argument: $1 (try --help)"
      ;;
  esac
done

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "Required command not found: $1"
}

# Kill every process listening on a TCP port, wait until the port is free.
kill_port() {
  local port="$1"
  local pids
  pids=$(lsof -ti tcp:"$port" 2>/dev/null || true)
  if [[ -n "$pids" ]]; then
    log "Freeing port ${port} (PIDs: $(echo "$pids" | tr '\n' ' '))..."
    # SIGTERM first, give 2 s, then SIGKILL
    echo "$pids" | xargs kill -TERM 2>/dev/null || true
    local i=0
    while nc -z 127.0.0.1 "$port" >/dev/null 2>&1 && (( i < 20 )); do
      sleep 0.1
      (( i++ )) || true
    done
    if nc -z 127.0.0.1 "$port" >/dev/null 2>&1; then
      echo "$pids" | xargs kill -KILL 2>/dev/null || true
      sleep 0.5
    fi
  fi
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
require_cmd lsof

[[ -d "$SERVER_DIR" ]] || fail "Server directory not found: $SERVER_DIR"
[[ -f "$SERVER_DIR/.env" ]] || fail "Server .env not found: $SERVER_DIR/.env"

log "Starting Postgres on ${DB_HOST}:${DB_PORT}..."
(cd "$SERVER_DIR" && docker compose up -d db)
(cd "$SERVER_DIR" && docker compose stop app >/dev/null 2>&1 || true)

# Stop a leftover API before restore — open connections make pg_restore --clean fail
# and sqlx migrate then creates empty tables (site up, catalog empty).
if [[ -n "$RESTORE_ARCHIVE" ]]; then
  log "Clearing port ${SERVER_PORT} so restore can take the database..."
  kill_port "$SERVER_PORT"
fi

if [[ -n "$RESTORE_ARCHIVE" ]]; then
  if [[ "$RESTORE_ARCHIVE" == "__latest__" ]]; then
    RESTORE_ARCHIVE="$(find "$ROOT_DIR/deploy/backups" -name 'gotiga-backup-*.tar.gz' -type f 2>/dev/null | sort | tail -1 || true)"
    [[ -n "$RESTORE_ARCHIVE" ]] || fail "No gotiga-backup-*.tar.gz under deploy/backups/. Run the panel backup first."
  fi
  [[ -f "$RESTORE_ARCHIVE" ]] || fail "Backup archive not found: $RESTORE_ARCHIVE"
  [[ -x "$ROOT_DIR/deploy/restore-local.sh" ]] || fail "deploy/restore-local.sh is missing or not executable."
  log "Restoring local DB + uploads from ${RESTORE_ARCHIVE}..."
  restore_args=("$ROOT_DIR/deploy/restore-local.sh" "$RESTORE_ARCHIVE")
  [[ -n "$RESTORE_YES" ]] && restore_args+=(--yes)
  "${restore_args[@]}"
else
  if ! nc -z "$DB_HOST" "$DB_PORT" >/dev/null 2>&1; then
    sleep 2
    if (cd "$SERVER_DIR" && docker compose logs db 2>/dev/null | grep -q 'incompatible with server'); then
      fail "Local Postgres volume is PG15, image is now 16 (needed for prod backups). Recreate the empty local DB:  cd src-tauri/server && docker compose down -v && docker compose up -d db   — or run  ./start_all.sh --restore  which does this for you."
    fi
  fi
  wait_for_port "$DB_HOST" "$DB_PORT" 60
fi

if [[ ! -d "$ROOT_DIR/node_modules" ]]; then
  log "Installing frontend dependencies..."
  (cd "$ROOT_DIR" && npm install)
fi

log "Clearing port ${SERVER_PORT}..."
kill_port "$SERVER_PORT"

log "Starting Rust API server..."
(cd "$SERVER_DIR" && cargo run) &
SERVER_PID=$!
wait_for_http "$SERVER_HEALTH_URL" 90

log "Clearing port ${FRONTEND_PORT}..."
kill_port "$FRONTEND_PORT"

log "Starting Vite frontend..."
(cd "$ROOT_DIR" && npm run dev) &
FRONTEND_PID=$!

log "Server:   ${SERVER_HEALTH_URL}"
log "Frontend: ${FRONTEND_URL}"
log "Press Ctrl+C to stop."

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
