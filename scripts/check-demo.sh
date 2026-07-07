#!/usr/bin/env bash
# Pre-flight check before a sales demo.
set -euo pipefail
cd "$(dirname "$0")/.."

BASE="${BASE:-http://localhost:8090}"
OP="${ADMIN_SECRET:-change-me-operator}"
FAIL=0

pass() { echo "  OK  $1"; }
fail() { echo "  FAIL $1"; FAIL=1; }

echo "HIPAA Hermes — demo pre-flight"
echo "=============================="
echo ""

echo "Tools:"
command -v docker >/dev/null 2>&1 && pass "docker" || fail "docker (install Docker)"
command -v curl >/dev/null 2>&1 && pass "curl" || fail "curl"
command -v jq >/dev/null 2>&1 && pass "jq" || fail "jq (sudo apt install jq)"

echo ""
echo "API:"
if curl -sf --max-time 3 "$BASE/health" | jq -e '.status == "ok"' >/dev/null 2>&1; then
  pass "API health ($BASE)"
else
  fail "API not running — run ./scripts/run.sh in another terminal"
fi

echo ""
echo "Observability:"
curl -sf --max-time 3 http://localhost:3000/api/health >/dev/null 2>&1 \
  && pass "Grafana :3000" || fail "Grafana not running — ./scripts/run.sh starts it"
curl -sf --max-time 3 http://localhost:9090/-/ready >/dev/null 2>&1 \
  && pass "Prometheus :9090" || fail "Prometheus not running"
curl -sf --max-time 3 http://localhost:3100/ready >/dev/null 2>&1 \
  && pass "Loki :3100" || fail "Loki not running"

echo ""
echo "RBAC smoke:"
code=$(curl -s -o /dev/null -w "%{http_code}" --max-time 3 \
  "$BASE/v1/audit/export" -H "X-Role-Key: $OP")
[[ "$code" == "403" ]] && pass "operator denied export (403)" || fail "RBAC check got HTTP $code"

echo ""
if [[ -f .env ]] && grep -qE '^LLM_DISABLED=1' .env 2>/dev/null; then
  echo "  WARN LLM_DISABLED=1 — stub mode"
elif [[ -f .env ]] && grep -qE '^LLM_PROVIDER=ollama' .env 2>/dev/null; then
  if curl -sf --max-time 2 http://127.0.0.1:11434/api/tags >/dev/null 2>&1; then
    pass "Ollama running — real local inference"
  else
    fail "LLM_PROVIDER=ollama but Ollama not running — ./scripts/setup-ollama.sh"
  fi
elif [[ -f .env ]] && grep -qE '^ANTHROPIC_API_KEY=.+$' .env 2>/dev/null; then
  pass "ANTHROPIC_API_KEY configured — Claude cloud inference"
else
  echo "  WARN stub mode — run ./scripts/setup-ollama.sh for real local AI"
fi

echo ""
if [[ "$FAIL" -eq 0 ]]; then
  echo "Ready for demo. Next:"
  echo "  ./scripts/sales-demo.sh     guided walkthrough"
  echo "  ./scripts/open-demo.sh      open Grafana in browser"
  echo "  docs/SALES_DEMO.md          talk track"
  exit 0
else
  echo "Fix failures above before the call."
  exit 1
fi
