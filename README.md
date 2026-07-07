# HIPAA Hermes Agent

Inference gateway for regulated environments — Rust API with **on-prem LLM** (Ollama), de-identification, audit log, RBAC, and observability.

**Architecture:** [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) · [diagram (SVG)](docs/diagrams/hipaa-hermes-architecture.svg)

**Not a HIPAA certification.** Reference architecture + control-plane primitives you harden for production.

## Includes

- Rust API (`crates/hermes`) — Axum gateway on `:8090`
- **On-prem inference** — Ollama in Docker; de-identified text never leaves the host
- Optional cloud fallback — Anthropic Claude (`LLM_PROVIDER=anthropic`)
- **v2 de-identification** — rule-based scrub before LLM; audit logs redaction counts only
- Append-only hashed audit log (SHA-256 per entry, SQLite)
- RBAC: `operator` (inference + audit read) · `auditor` (audit read/export only)
- Prometheus + Grafana + Loki + Promtail (`deploy/docker-compose.yml`)

## Prerequisites

- [Rust](https://rustup.rs) (stable)
- Docker (for observability stack)
- `jq`, `curl`

## Quick start

```bash
cp .env.example .env
./scripts/setup-ollama.sh   # pulls local model into Docker (one-time)
chmod +x scripts/run.sh scripts/stop.sh scripts/demo.sh
./scripts/run.sh
```

In another terminal:

```bash
./scripts/demo.sh
./scripts/stop.sh   # stop containers + API
```

| URL | Purpose |
|-----|---------|
| http://localhost:8090 | API |
| http://localhost:3000 | Grafana (`admin` / `admin`) |
| http://localhost:9090 | Prometheus |
| http://localhost:3100 | Loki |

## Secrets (local `.env`)

| Variable | Purpose |
|----------|---------|
| `LLM_PROVIDER` | `ollama` (default, on-prem) or `anthropic` (cloud) |
| `OLLAMA_MODEL` | Default `llama3.2:1b` — run `./scripts/setup-ollama.sh` first |
| `ANTHROPIC_API_KEY` | Optional — only if `LLM_PROVIDER=anthropic` |
| `ADMIN_SECRET` | Operator `X-Role-Key` |
| `AUDITOR_SECRET` | Auditor `X-Role-Key` |
| `CLAUDE_MODEL` | Default `claude-sonnet-4-20250514` |
| `LLM_DISABLED=1` | Force stub inference |

## Tests

```bash
cargo test
```

## Demo video

[docs/DEMO_VIDEO.md](docs/DEMO_VIDEO.md) — 2–3 min beat sheet.

## Sales demo (for live calls)

**Clinician UI (best for buyers):** http://localhost:8090/demo/  
Synthetic chart → **de-ID** → policy gate → **local model** → audit.

Your colleague can run a guided 5–8 minute demo without reading the code:

```bash
# Terminal 1 — start stack (once per session)
./scripts/run.sh

# Terminal 2 — before the call
./scripts/check-demo.sh      # all green?
./scripts/sales-demo.sh      # guided walkthrough with talk track
./scripts/open-demo.sh       # open Grafana tabs
```

Full playbook with what to say: **[docs/SALES_DEMO.md](docs/SALES_DEMO.md)**
