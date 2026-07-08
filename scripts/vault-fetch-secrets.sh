#!/usr/bin/env bash
# Copy rendered secrets from Vault Agent container to a host-owned file.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

COMPOSE="docker compose -f deploy/docker-compose.yml -f deploy/docker-compose.vault.yml"
OUT="data/vault/hermes.env"

mkdir -p data/vault
CID=$($COMPOSE ps -q vault-agent 2>/dev/null || true)
if [ -z "$CID" ]; then
  echo "vault-agent container not running — run ./scripts/setup-vault.sh" >&2
  exit 1
fi

docker exec "$CID" cat /vault/secrets/hermes.env > "${OUT}.tmp"
mv "${OUT}.tmp" "$OUT"
chmod 600 "$OUT"
