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
if [[ -f .env ]] && grep -qE '^OIDC_ENABLED=(1|true)' .env 2>/dev/null; then
  if command -v docker >/dev/null 2>&1 && curl -sf --max-time 2 http://127.0.0.1:8180/realms/hermes >/dev/null 2>&1; then
    pass "Keycloak :8180 (OIDC mode)"
    if [[ -x scripts/get-oidc-token.sh ]]; then
      if token=$(./scripts/get-oidc-token.sh operator operator 2>/dev/null) && [[ -n "$token" ]]; then
        code=$(curl -s -o /dev/null -w "%{http_code}" --max-time 5 \
          "$BASE/v1/audit/export" -H "Authorization: Bearer $token")
        [[ "$code" == "403" ]] && pass "JWT operator denied export (403)" || fail "OIDC RBAC check got HTTP $code"
      else
        fail "could not fetch OIDC token — ./scripts/setup-keycloak.sh"
      fi
    fi
  else
    fail "OIDC_ENABLED but Keycloak not on :8180 — ./scripts/setup-keycloak.sh"
  fi
else
  code=$(curl -s -o /dev/null -w "%{http_code}" --max-time 3 \
    "$BASE/v1/audit/export" -H "X-Role-Key: $OP")
  [[ "$code" == "403" ]] && pass "operator denied export (403)" || fail "RBAC check got HTTP $code"
fi

echo ""
if [[ -f .env ]] && grep -qE '^LLM_DISABLED=1' .env 2>/dev/null; then
  echo "  WARN LLM_DISABLED=1 — stub mode"
elif [[ -f .env ]] && grep -qE '^DEID_MODE=hybrid' .env 2>/dev/null; then
  if curl -sf --max-time 2 "${DEID_NER_URL:-http://127.0.0.1:3001}/health" >/dev/null 2>&1; then
    pass "Presidio analyzer running — hybrid de-ID"
  else
    fail "DEID_MODE=hybrid but Presidio not reachable — set DEID_NER_URL=http://127.0.0.1:3001"
  fi
elif [[ -f .env ]] && grep -qE '^LLM_PROVIDER=ollama' .env 2>/dev/null; then
  if curl -sf --max-time 2 http://127.0.0.1:11434/api/tags >/dev/null 2>&1; then
    pass "BioMistral runtime running — local clinical inference"
    MODEL="$(grep -E '^OLLAMA_MODEL=' .env 2>/dev/null | cut -d= -f2- | tr -d '"' || true)"
    MODEL="${MODEL:-biomistral-hermes}"
    if curl -sf --max-time 2 http://127.0.0.1:11434/api/tags \
      | jq -e --arg m "$MODEL" '.models[].name | select(. == $m)' >/dev/null 2>&1; then
      pass "BioMistral model ready: $MODEL"
    else
      fail "OLLAMA_MODEL=$MODEL not loaded — ./scripts/setup-biomistral.sh (see docs/MODELS.md)"
    fi
  else
    fail "LLM_PROVIDER=ollama but inference runtime not running — ./scripts/setup-biomistral.sh"
  fi
elif [[ -f .env ]] && grep -qE '^ANTHROPIC_API_KEY=.+$' .env 2>/dev/null; then
  pass "ANTHROPIC_API_KEY configured — Claude cloud inference"
else
  echo "  WARN stub mode — run ./scripts/setup-biomistral.sh for real local BioMistral"
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
