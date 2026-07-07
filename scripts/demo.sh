#!/usr/bin/env bash
# Quick smoke test for v1 API + RBAC
set -euo pipefail
cd "$(dirname "$0")/.."
export PYTHONPATH=src
source .venv/bin/activate 2>/dev/null || true

BASE="${BASE:-http://localhost:8090}"
OP="${ADMIN_SECRET:-change-me-operator}"
AUD="${AUDITOR_SECRET:-change-me-auditor}"

echo "== health =="
curl -sf "$BASE/health" | jq .

echo "== operator inference =="
curl -sf -X POST "$BASE/v1/inference" \
  -H "X-Role-Key: $OP" \
  -H "Content-Type: application/json" \
  -d '{"prompt":"de-identified clinical note summary","skill":"vault-answer"}' | jq .

echo "== auditor read audit =="
curl -sf "$BASE/v1/audit" -H "X-Role-Key: $AUD" | jq '.entries | length'

echo "== operator denied export (expect 403) =="
code=$(curl -s -o /dev/null -w "%{http_code}" "$BASE/v1/audit/export" -H "X-Role-Key: $OP")
echo "HTTP $code"

echo "== metrics =="
curl -sf "$BASE/metrics" | jq .

echo "OK"
