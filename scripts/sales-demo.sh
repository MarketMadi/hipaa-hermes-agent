#!/usr/bin/env bash
# Guided sales demo — press Enter between acts.
set -euo pipefail
cd "$(dirname "$0")/.."

BASE="${BASE:-http://localhost:8090}"
OP="${ADMIN_SECRET:-change-me-operator}"
AUD="${AUDITOR_SECRET:-change-me-auditor}"

pause() {
  echo ""
  echo "────────────────────────────────────────"
  echo "$1"
  echo "────────────────────────────────────────"
  read -r -p "Press Enter to continue..."
  echo ""
}

banner() {
  echo ""
  echo "========================================"
  echo "  $1"
  echo "========================================"
}

if ! ./scripts/check-demo.sh 2>/dev/null; then
  echo ""
  echo "Pre-flight failed. Start the stack first:"
  echo "  Terminal 1: ./scripts/run.sh"
  echo "  Terminal 2: ./scripts/sales-demo.sh"
  exit 1
fi

banner "HIPAA Hermes — Sales Demo"
echo "Talk track: docs/SALES_DEMO.md"
echo "Grafana:    http://localhost:3000/d/hipaa-hermes-obs/hipaa-hermes-observability"
echo ""
echo "Opening Grafana..."
./scripts/open-demo.sh 2>/dev/null || true

pause "ACT 1 — Say: 'Reference gateway for regulated AI — not a certification.'"

banner "ACT 2 — Operator inference (happy path)"
echo "Running de-identified clinical prompt..."
curl -s -X POST "$BASE/v1/inference" \
  -H "X-Role-Key: $OP" \
  -H "Content-Type: application/json" \
  -d '{"prompt":"Summarize de-identified discharge note: patient stable, discharged home.","skill":"vault-answer"}' | jq .

pause "ACT 2 — Point at Grafana audit table: new row, hash_valid true. Prompt text NOT stored."

banner "ACT 3 — Policy blocks PHI (wow moment)"
echo "Sending prompt with SSN pattern — should NOT reach the model..."
curl -s -w "\nHTTP %{http_code}\n" -X POST "$BASE/v1/inference" \
  -H "X-Role-Key: $OP" \
  -H "Content-Type: application/json" \
  -d '{"prompt":"Patient SSN 123-45-6789 needs callback","skill":"vault-answer"}' | jq . 2>/dev/null || true

pause "ACT 3 — Say: 'Policy fired before the model. Block is audited.' Show outcome=blocked in Grafana."

banner "ACT 4 — RBAC separation of duties"
echo "Auditor reads audit ($(curl -sf "$BASE/v1/audit" -H "X-Role-Key: $AUD" | jq '.entries | length') entries)..."
code=$(curl -s -o /dev/null -w "%{http_code}" "$BASE/v1/audit/export" -H "X-Role-Key: $OP")
echo "Operator export attempt: HTTP $code (expect 403)"

pause "ACT 4 — Say: 'Operator runs AI; auditor exports trail. Neither can do both.'"

banner "ACT 5 — Observability"
echo "Metrics snapshot:"
curl -sf "$BASE/api/stats" | jq .
echo ""
echo "In Grafana show:"
echo "  - Audit entries (total) stat"
echo "  - Application logs panel (Loki)"
echo "  - Audit log table"

pause "ACT 5 — Say: 'Ops visibility without PHI in logs.'"

banner "ACT 6 — Close"
cat <<'EOF'

Say:

  "This proves policy, RBAC, audit, and observability first.
   Production adds encrypted storage, SSO, and BAA vendors.
   Happy to scope a pilot under NDA."

Full playbook: docs/SALES_DEMO.md

EOF

echo "Demo complete."
