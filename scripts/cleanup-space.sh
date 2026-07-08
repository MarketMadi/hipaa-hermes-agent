#!/usr/bin/env bash
# Reclaim disk/memory for HIPAA Hermes on constrained machines.
# Run from repo root. Review output before answering "yes" to destructive steps.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "HIPAA Hermes — space cleanup"
echo "============================"
echo ""

echo "Disk:"
df -h / | tail -1
echo ""
echo "Memory:"
free -h | head -2
echo ""

echo "Hermes footprint:"
du -sh target .venv data 2>/dev/null || true
if docker ps --format '{{.Names}}' 2>/dev/null | grep -q '^deploy-ollama-1$'; then
  echo ""
  echo "BioMistral models (keep only .env OLLAMA_MODEL):"
  docker exec deploy-ollama-1 ollama list 2>/dev/null || true
fi
echo ""

echo "Safe Hermes cleanups (this script):"
echo "  1) cargo clean          — Rust build cache (~2 GB, rebuilds on ./scripts/run.sh)"
echo "  2) Remove extra models not matching .env OLLAMA_MODEL"
echo ""

read -r -p "Run safe Hermes cleanups? [y/N] " ans
if [[ "${ans,,}" == "y" ]]; then
  cargo clean
  if [[ -f .env ]] && docker ps --format '{{.Names}}' 2>/dev/null | grep -q '^deploy-ollama-1$'; then
  WANTED="$(grep -E '^OLLAMA_MODEL=' .env | cut -d= -f2- | tr -d '"' || true)"
  if [[ -n "$WANTED" ]]; then
    while read -r name _rest; do
      [[ -z "$name" || "$name" == NAME ]] && continue
      if [[ "$name" != "$WANTED" ]]; then
        echo "Removing unused model: $name"
        docker exec deploy-ollama-1 ollama rm "$name" || true
      fi
    done < <(docker exec deploy-ollama-1 ollama list 2>/dev/null)
  fi
  fi
  echo "Done."
fi

echo ""
echo "Other high-impact cleanups (run manually if needed):"
echo "  docker builder prune -f              # Docker build cache (~3 GB)"
echo "  docker image prune -a                  # Unused images (~8+ GB; review first)"
echo "  rm ~/lightning*.tar.gz                 # ~3 GB if archives not needed"
echo "  pip cache purge                        # ~3 GB in ~/.cache/pip"
echo ""
echo "Memory while demoing on 16 GB RAM:"
echo "  Keep biomistral-hermes only (~4.4 GB disk, ~5 GB RAM when loaded)"
echo "  DEID_MODE=rules — skip Presidio (~750 MB RAM)"
echo "  Stop unrelated stacks: BTCPay, Neo4j/nexus, Jitsi images if idle"
echo ""
echo "See docs/MODELS.md#hardware for sizing."
