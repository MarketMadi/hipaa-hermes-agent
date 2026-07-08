#!/usr/bin/env bash
# Fetch an access token from local Keycloak (resource-owner password — dev only).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

KC_URL="${KC_URL:-http://127.0.0.1:8180}"
REALM="${KC_REALM:-hermes}"
CLIENT_ID="${OIDC_CLIENT_ID:-hermes-api}"

USER="${1:-operator}"
PASS="${2:-operator}"

TOKEN_URL="$KC_URL/realms/$REALM/protocol/openid-connect/token"

resp=$(curl -sf --max-time 10 -X POST "$TOKEN_URL" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=password" \
  -d "client_id=$CLIENT_ID" \
  -d "username=$USER" \
  -d "password=$PASS") || {
  echo "Token request failed. Is Keycloak running? ./scripts/setup-keycloak.sh" >&2
  exit 1
}

if command -v jq >/dev/null 2>&1; then
  echo "$resp" | jq -r '.access_token'
else
  echo "$resp" | sed -n 's/.*"access_token":"\([^"]*\)".*/\1/p'
fi
