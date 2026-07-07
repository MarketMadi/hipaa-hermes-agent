#!/usr/bin/env bash
# Start API + full observability stack (Prometheus, Grafana, Loki, Promtail).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

mkdir -p logs data
touch logs/api.log
[ -f .env ] || cp .env.example .env

if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
fi

BASE="${BASE:-http://127.0.0.1:8090}"
OP="${ADMIN_SECRET:-change-me-operator}"

api_inference_ok() {
  local body
  body=$(curl -s --max-time 90 -X POST "$BASE/v1/inference" \
    -H "Content-Type: application/json" \
    -H "X-Role-Key: $OP" \
    -d '{"prompt":"health ping","skill":"vault-answer"}' 2>/dev/null) || return 1
  echo "$body" | jq -e '.audit_id or .detail' >/dev/null 2>&1
}

# Already running — verify inference works (wedged API can still pass /health)
if curl -sf --max-time 2 "$BASE/health" | jq -e '.status == "ok"' >/dev/null 2>&1; then
  if api_inference_ok; then
    echo "API already running."
    echo ""
    echo "  Clinician demo:  $BASE/demo/"
    echo "  Grafana:         http://localhost:3000  (admin / admin)"
    echo "  Stop everything: ./scripts/stop.sh"
    exit 0
  fi
  echo "API on :8090 but inference wedged — restarting..."
  fuser -k 8090/tcp 2>/dev/null || true
  pkill -f "./target/release/hermes" 2>/dev/null || true
  sleep 1
fi

# Stale process holding the port
if command -v fuser >/dev/null 2>&1 && fuser 8090/tcp >/dev/null 2>&1; then
  echo "Port 8090 busy — stopping stale process..."
  fuser -k 8090/tcp 2>/dev/null || true
  sleep 1
fi

echo "Building hermes (release)..."
cargo build --release -p hermes

echo "Starting observability stack (Prometheus, Grafana, Loki, Promtail, Ollama)..."
docker compose -f deploy/docker-compose.yml up -d

echo ""
echo "URLs:"
echo "  Clinician demo: http://localhost:8090/demo/   <-- hospital-style UI"
echo "  API:            http://localhost:8090"
echo "  Grafana:        http://localhost:3000  (admin / admin)"
echo "  Prometheus:     http://localhost:9090"
echo "  Loki:           http://localhost:3100"
echo "  Ollama:         http://localhost:11434  (when LLM_PROVIDER=ollama)"
echo ""
echo "Dashboard: HIPAA Hermes Observability"
echo "App logs:  logs/api.log → Promtail → Loki"
echo ""
echo "Press Ctrl+C to stop the API (containers keep running)."
echo "Stop everything: ./scripts/stop.sh"
echo ""

# tee duplicates stdout/stderr to the file Promtail tails
exec ./target/release/hermes 2>&1 | tee -a logs/api.log
