#!/usr/bin/env bash
# Enable local HTTPS: Hermes on 127.0.0.1:8090 + Caddy on https://localhost:8443
set -euo pipefail
cd "$(dirname "$0")/.."

touch .env
if grep -qE '^HERMES_BEHIND_PROXY=' .env 2>/dev/null; then
  sed -i 's/^HERMES_BEHIND_PROXY=.*/HERMES_BEHIND_PROXY=1/' .env
else
  echo "HERMES_BEHIND_PROXY=1" >> .env
fi
if grep -qE '^BIND_HOST=' .env 2>/dev/null; then
  sed -i 's/^BIND_HOST=.*/BIND_HOST=127.0.0.1/' .env
else
  echo "BIND_HOST=127.0.0.1" >> .env
fi

echo "HERMES_BEHIND_PROXY=1 and BIND_HOST=127.0.0.1 set in .env"
exec ./scripts/run.sh
