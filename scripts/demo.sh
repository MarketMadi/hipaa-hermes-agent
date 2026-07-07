#!/usr/bin/env bash
# Quick smoke test for API + RBAC
set -euo pipefail
cd "$(dirname "$0")/.."

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

echo "== prometheus metrics =="
curl -sf "$BASE/metrics" | grep -E "hipaa_hermes|http_requests" | head -10

echo "== json stats =="
curl -sf "$BASE/api/stats" | jq .

echo "OK"
