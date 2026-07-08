#!/usr/bin/env bash
# One-way migration: SQLite audit log → Postgres.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

SQLITE_PATH="${1:-data/hipaa_hermes.db}"
DATABASE_URL="${DATABASE_URL:-postgres://hermes:hermes@127.0.0.1:5433/hermes_audit}"

if [ ! -f "$SQLITE_PATH" ]; then
  echo "SQLite audit DB not found: $SQLITE_PATH" >&2
  exit 1
fi

cargo build --release -p hermes --bin migrate-audit
DATABASE_URL="$DATABASE_URL" ./target/release/migrate-audit "$SQLITE_PATH" "$DATABASE_URL"
