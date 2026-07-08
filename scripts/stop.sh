#!/usr/bin/env bash
# Stop observability containers and any API on :8090.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if command -v fuser >/dev/null 2>&1; then
  fuser -k 8090/tcp 2>/dev/null || true
fi
pkill -f "./target/release/hermes" 2>/dev/null || true
pkill -f "target/release/hermes" 2>/dev/null || true

docker compose -f deploy/docker-compose.yml -f deploy/docker-compose.proxy.yml down 2>/dev/null || true
docker compose -f deploy/docker-compose.yml -f deploy/docker-compose.vault.yml down 2>/dev/null || true
docker compose -f deploy/docker-compose.yml down
echo "Stopped Vault, TLS proxy, Prometheus, Grafana, Loki, Promtail, BioMistral runtime, and API."
