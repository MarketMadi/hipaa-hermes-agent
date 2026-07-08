#!/usr/bin/env bash
# Start Hermes with secrets from Vault Agent (local dev).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

mkdir -p logs data/vault
touch logs/api.log
[ -f .env ] || cp .env.example .env

if [ ! -r data/vault/hermes.env ]; then
  if docker compose -f deploy/docker-compose.yml -f deploy/docker-compose.vault.yml ps -q vault-agent >/dev/null 2>&1; then
    ./scripts/vault-fetch-secrets.sh
  else
    echo "Vault secrets not rendered — running ./scripts/setup-vault.sh first..."
    ./scripts/setup-vault.sh
  fi
fi

if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
fi

# Non-secret config from .env; sensitive keys overridden by Vault render.
# shellcheck disable=SC1091
set -a
source .env
source data/vault/hermes.env
export VAULT_ENABLED=1
set +a

BEHIND_PROXY=0
if [ "${HERMES_BEHIND_PROXY:-0}" = "1" ]; then
  BEHIND_PROXY=1
fi

BASE="${BASE:-http://127.0.0.1:8090}"
if [ "$BEHIND_PROXY" -eq 1 ]; then
  BASE="https://localhost:8443"
fi
CURL_TLS=()
if [[ "$BASE" == https://* ]]; then
  CURL_TLS=(-k)
fi
OP="${ADMIN_SECRET:-change-me-operator}"

api_inference_ok() {
  local body
  body=$(curl -s "${CURL_TLS[@]}" --max-time 90 -X POST "$BASE/v1/inference" \
    -H "Content-Type: application/json" \
    -H "X-Role-Key: $OP" \
    -d '{"prompt":"health ping","skill":"vault-answer"}' 2>/dev/null) || return 1
  echo "$body" | jq -e '.audit_id or .detail' >/dev/null 2>&1
}

if curl -sf "${CURL_TLS[@]}" --max-time 2 "$BASE/health" | jq -e '.status == "ok"' >/dev/null 2>&1; then
  if api_inference_ok; then
    echo "API already running (Vault-backed secrets)."
    curl -sf "${CURL_TLS[@]}" "$BASE/health" | jq '{status, secrets_source, audit_backend}'
    exit 0
  fi
  fuser -k 8090/tcp 2>/dev/null || true
  pkill -f "./target/release/hermes" 2>/dev/null || true
  sleep 1
fi

if command -v fuser >/dev/null 2>&1 && fuser 8090/tcp >/dev/null 2>&1; then
  fuser -k 8090/tcp 2>/dev/null || true
  sleep 1
fi

echo "Building hermes (release)..."
cargo build --release -p hermes

echo "Starting observability stack..."
docker compose -f deploy/docker-compose.yml up -d

if [ "$BEHIND_PROXY" -eq 1 ]; then
  docker compose -f deploy/docker-compose.yml -f deploy/docker-compose.proxy.yml up -d caddy
fi

echo ""
echo "Hermes starting with VAULT_ENABLED=1 (secrets from data/vault/hermes.env)"
echo "  API:     $BASE"
echo "  Vault:   ${VAULT_ADDR:-http://127.0.0.1:8200}"
echo "  Demo:    $BASE/demo/"
echo ""
exec ./target/release/hermes 2>&1 | tee -a logs/api.log
