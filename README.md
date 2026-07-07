# HIPAA Hermes Agent

Inference platform skeleton for regulated environments — v1 per [docs/SCOPE.md](docs/SCOPE.md).

**Architecture:** [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) · [diagram (SVG)](docs/diagrams/hipaa-hermes-architecture.svg)

**Not a HIPAA certification.** Reference architecture + audit/RBAC primitives you harden for production.

## v1 includes

- Append-only hashed audit log (SHA-256 per entry, no hash-chain)
- RBAC: `operator` (inference + audit read) · `auditor` (audit read/export only)
- One Grafana dashboard JSON (`deploy/grafana/hipaa-hermes-v1.json`)
- Reference architecture diagram ([docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)) — **complete [Q12](docs/Q12_PUBLICATION.md) before publishing**

## Cut from v1 (§4a.4)

Loki · multi-env CI/CD · case-study doc · full hash-chain · public log tunnels

## Quick start

```bash
python -m venv .venv && source .venv/bin/activate
pip install -r requirements.txt
cp .env.example .env
mkdir -p data
export PYTHONPATH=src
uvicorn hipaa_hermes.main:app --reload --port 8090
```

```bash
chmod +x scripts/demo.sh && ./scripts/demo.sh
```

## Before client calls

[docs/PRE_CLIENT.md](docs/PRE_CLIENT.md) — Q11 (Bayana) + role-overlap sentence must be settled with Vino.

## Demo video

[docs/DEMO_VIDEO.md](docs/DEMO_VIDEO.md) — 2–3 min beat sheet.
