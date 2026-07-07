#!/usr/bin/env bash
# Start Ollama in Docker and pull a model for real local inference.
set -euo pipefail
cd "$(dirname "$0")/.."

MODEL="${OLLAMA_MODEL:-llama3.2:1b}"
COMPOSE="docker compose -f deploy/docker-compose.yml"

echo "HIPAA Hermes — Ollama setup (Docker)"
echo "===================================="
echo ""

echo "Starting Ollama container..."
$COMPOSE up -d ollama

echo "Waiting for Ollama API..."
for _ in $(seq 1 30); do
  if curl -sf --max-time 2 http://127.0.0.1:11434/api/tags >/dev/null 2>&1; then
    break
  fi
  sleep 1
done

if ! curl -sf --max-time 2 http://127.0.0.1:11434/api/tags >/dev/null 2>&1; then
  echo "Ollama did not become ready on :11434"
  exit 1
fi

echo "Pulling model: $MODEL (first run may take a few minutes)..."
docker exec "$(docker compose -f deploy/docker-compose.yml ps -q ollama)" ollama pull "$MODEL"

echo ""
echo "Add to .env (if not already):"
echo "  LLM_PROVIDER=ollama"
echo "  OLLAMA_MODEL=$MODEL"
echo ""
echo "Then restart: ./scripts/run.sh"
