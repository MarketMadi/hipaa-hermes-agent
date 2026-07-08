#!/usr/bin/env bash
# Start Keycloak with the Hermes dev realm (operator / auditor test users).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

KC_URL="${KC_URL:-http://127.0.0.1:8180}"
REALM="${KC_REALM:-hermes}"

echo "Starting Keycloak (dev mode) on :8180..."
docker compose -f deploy/docker-compose.yml -f deploy/docker-compose.oidc.yml up -d keycloak

echo "Waiting for Keycloak..."
for _ in $(seq 1 60); do
  if curl -sf --max-time 2 "$KC_URL/realms/$REALM" >/dev/null 2>&1; then
    echo ""
    echo "Keycloak ready."
    echo ""
    echo "  Admin console: $KC_URL/admin/  (admin / admin)"
    echo "  Issuer:        $KC_URL/realms/$REALM"
    echo ""
    echo "Test users (password grant — local dev only):"
    echo "  operator / operator  → hermes-operator"
    echo "  auditor  / auditor   → hermes-auditor"
    echo ""
    echo "Enable OIDC in Hermes (.env):"
    echo "  OIDC_ENABLED=1"
    echo "  OIDC_ISSUER=$KC_URL/realms/$REALM"
    echo "  OIDC_AUDIENCE=hermes-api"
    echo "  OIDC_ALLOW_ROLE_KEY=1"
    echo ""
    echo "Get a token:"
    echo "  ./scripts/get-oidc-token.sh operator operator"
    exit 0
  fi
  sleep 2
done

echo "Keycloak did not become ready in time. Check: docker logs \$(docker ps -qf name=keycloak)"
exit 1
