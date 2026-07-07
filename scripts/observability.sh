#!/usr/bin/env bash
# Start observability containers only (no API). Prefer ./scripts/run.sh for the full stack.
set -euo pipefail
cd "$(dirname "$0")/.."

mkdir -p logs

echo "Starting Prometheus, Grafana, Loki, and Promtail..."
docker compose -f deploy/docker-compose.yml up -d

echo ""
echo "URLs:"
echo "  Grafana:    http://localhost:3000  (admin / admin)"
echo "  Prometheus: http://localhost:9090"
echo "  Loki:       http://localhost:3100"
echo ""
echo "Start API + log shipping: ./scripts/run.sh"
echo "Stop stack:               ./scripts/stop.sh"
