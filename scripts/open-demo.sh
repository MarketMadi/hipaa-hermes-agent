#!/usr/bin/env bash
# Open demo URLs in the default browser.
set -euo pipefail

GRAFANA_DASH="http://localhost:3000/d/hipaa-hermes-obs/hipaa-hermes-observability?orgId=1&from=now-15m&to=now"
ARCH="file://$(cd "$(dirname "$0")/.." && pwd)/docs/ARCHITECTURE.md"

open_url() {
  xdg-open "$1" 2>/dev/null || sensible-browser "$1" 2>/dev/null || echo "  Open manually: $1"
}

echo "Opening demo tabs..."
open_url "http://localhost:8090/demo/"
sleep 0.5
open_url "$GRAFANA_DASH"
sleep 0.5
open_url "http://localhost:3000/explore?orgId=1&left=%7B%22datasource%22:%22loki%22,%22queries%22:%5B%7B%22refId%22:%22A%22,%22expr%22:%22%7Bjob%3D%5C%22hipaa-hermes%5C%22%7D%22%7D%5D,%22range%22:%7B%22from%22:%22now-15m%22,%22to%22:%22now%22%7D%7D"
echo ""
echo "Grafana login: admin / admin"
echo "Architecture:  docs/ARCHITECTURE.md"
