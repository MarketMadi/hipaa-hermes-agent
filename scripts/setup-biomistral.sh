#!/usr/bin/env bash
# Pull BioMistral-7B and create the biomistral-hermes local model.
set -euo pipefail
cd "$(dirname "$0")/.."

MODEL="${OLLAMA_MODEL:-biomistral-hermes}"
BIOMISTRAL_BASE="${BIOMISTRAL_BASE:-adrienbrault/biomistral-7b:Q4_K_M}"
COMPOSE="docker compose -f deploy/docker-compose.yml"

echo "HIPAA Hermes — BioMistral setup"
echo "==============================="
echo ""
echo "Model: $MODEL (~4.4 GB, clinical on-prem)"
echo "See docs/MODELS.md for alternatives."
echo ""

echo "Starting local inference runtime..."
$COMPOSE up -d ollama

echo "Waiting for inference API..."
for _ in $(seq 1 30); do
  if curl -sf --max-time 2 http://127.0.0.1:11434/api/tags >/dev/null 2>&1; then
    break
  fi
  sleep 1
done

if ! curl -sf --max-time 2 http://127.0.0.1:11434/api/tags >/dev/null 2>&1; then
  echo "Inference runtime did not become ready on :11434"
  exit 1
fi

RUNTIME_CID="$($COMPOSE ps -q ollama)"

if [[ "$MODEL" == "biomistral-hermes" ]]; then
  echo ""
  echo "Pulling BioMistral-7B weights (first run may take several minutes)..."
  docker exec "$RUNTIME_CID" ollama pull "$BIOMISTRAL_BASE"
  echo ""
  echo "Creating biomistral-hermes (fixed clinical chat template)..."
  docker cp deploy/ollama/Modelfile.biomistral "$RUNTIME_CID:/tmp/Modelfile.biomistral"
  docker exec "$RUNTIME_CID" ollama create biomistral-hermes -f /tmp/Modelfile.biomistral
else
  echo ""
  echo "Pulling model: $MODEL..."
  docker exec "$RUNTIME_CID" ollama pull "$MODEL"
fi

echo ""
echo "Add to .env (if not already):"
echo "  LLM_PROVIDER=ollama"
echo "  OLLAMA_MODEL=$MODEL"
echo ""
echo "Then restart: ./scripts/run.sh"
