#!/usr/bin/env bash
# Start local Vault (dev mode) and seed secret/hermes/local from .env.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VAULT_ADDR="${VAULT_ADDR:-http://127.0.0.1:8200}"
VAULT_TOKEN="${VAULT_TOKEN:-hermes-dev-root}"
COMPOSE="docker compose -f deploy/docker-compose.yml -f deploy/docker-compose.vault.yml"

mkdir -p data/vault
[ -f .env ] || cp .env.example .env

printf '%s' "${VAULT_TOKEN:-hermes-dev-root}" > data/vault/dev-token
chmod 644 data/vault/dev-token

# shellcheck disable=SC1091
set -a
source .env
set +a

echo "Starting Vault dev server on :8200..."
$COMPOSE up -d vault

echo "Waiting for Vault..."
for _ in $(seq 1 30); do
  if curl -sf "$VAULT_ADDR/v1/sys/health" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done

if ! curl -sf "$VAULT_ADDR/v1/sys/health" >/dev/null 2>&1; then
  echo "Vault did not become ready. Check: docker logs deploy-vault-1" >&2
  exit 1
fi

VAULT_CID=$($COMPOSE ps -q vault)
if [ -z "$VAULT_CID" ]; then
  echo "Vault container not found" >&2
  exit 1
fi

echo "Seeding secret/hermes/local from .env..."
docker exec \
  -e VAULT_ADDR="$VAULT_ADDR" \
  -e VAULT_TOKEN="$VAULT_TOKEN" \
  "$VAULT_CID" \
  vault kv put secret/hermes/local \
    admin_secret="${ADMIN_SECRET:-change-me-operator}" \
    auditor_secret="${AUDITOR_SECRET:-change-me-auditor}" \
    database_url="${DATABASE_URL:-}" \
    anthropic_api_key="${ANTHROPIC_API_KEY:-}"

echo "Starting Vault Agent (renders data/vault/hermes.env)..."
$COMPOSE up -d vault-agent

echo "Waiting for rendered secrets file..."
for _ in $(seq 1 30); do
  if [ -s data/vault/hermes.env ] || docker compose -f deploy/docker-compose.yml -f deploy/docker-compose.vault.yml ps -q vault-agent >/dev/null 2>&1; then
    ./scripts/vault-fetch-secrets.sh
  fi
  if [ -s data/vault/hermes.env ]; then
    echo ""
    echo "Vault ready."
    echo ""
    echo "  Vault UI/API:  $VAULT_ADDR  (token: $VAULT_TOKEN — local dev only)"
    echo "  Secrets file:  data/vault/hermes.env  (gitignored)"
    echo ""
    echo "Run Hermes with injected secrets:"
    echo "  ./scripts/run-with-vault.sh"
    exit 0
  fi
  sleep 1
done

echo "Vault Agent did not render data/vault/hermes.env in time." >&2
echo "Check: docker logs deploy-vault-agent-1" >&2
exit 1
