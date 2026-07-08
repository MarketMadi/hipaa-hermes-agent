#!/usr/bin/env bash
# Start Postgres for Hermes audit storage (dev/prod).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

PG_URL="${DATABASE_URL:-postgres://hermes:hermes@127.0.0.1:5433/hermes_audit}"

echo "Starting Postgres on :5433..."
docker compose -f deploy/docker-compose.yml -f deploy/docker-compose.postgres.yml up -d postgres

echo "Waiting for Postgres..."
for _ in $(seq 1 30); do
  if docker compose -f deploy/docker-compose.yml -f deploy/docker-compose.postgres.yml exec -T postgres pg_isready -U hermes -d hermes_audit -p 5433 >/dev/null 2>&1; then
    echo ""
    echo "Postgres ready."
    echo ""
    echo "  Connection: $PG_URL"
    echo ""
    echo "Add to .env (dev/prod):"
    echo "  AUDIT_BACKEND=postgres"
    echo "  DATABASE_URL=$PG_URL"
    echo ""
    echo "Migrate existing SQLite audit log:"
    echo "  ./scripts/migrate-audit-to-postgres.sh"
    exit 0
  fi
  sleep 2
done

echo "Postgres did not become ready in time."
exit 1
