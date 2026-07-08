# HIPAA Hermes Agent

**The open-source AI gateway for regulated healthcare** — de-identify clinical text, enforce policy, audit every inference, and run **BioMistral-7B on-prem**. No PHI leaves your machine.

[![CI](https://github.com/MarketMadi/hipaa-hermes-agent/actions/workflows/ci.yml/badge.svg)](https://github.com/MarketMadi/hipaa-hermes-agent/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://rustup.rs)

[Quick start](#quick-start) · [Demo UI](#demo) · [OIDC](#oidc--sso) · [Architecture](#architecture) · [Docs](#documentation) · [Roadmap](docs/DEPLOYMENT_EPICS.md)

---

## Why Hermes?

Hospitals want AI in the EHR. Compliance wants **control**. Hermes sits between them:

```
Clinician → [Hermes gateway] → BioMistral (local, on-prem)
              ├─ Safe Harbor de-ID (v3)
              ├─ Policy gate (block SSN, email, phone)
              ├─ RBAC (JWT / OIDC or X-Role-Key)
              └─ Append-only audit log + Grafana
```

**Local-first:** after de-identification, **BioMistral** runs on infrastructure you control — not a cloud API. Optional Anthropic fallback when you have a BAA.

> **Not a HIPAA certification.** Reference architecture + control-plane primitives you harden for production. [Honest scope →](docs/SCOPE.md)

---

## Demo

**Clinician UI:** http://localhost:8090/demo/ (after `./scripts/run.sh`)

Synthetic discharge notes → de-ID preview → policy check → **BioMistral** → `audit_id` in the response.

![Architecture](docs/diagrams/hipaa-hermes-architecture.svg)

---

## Features

| Feature | What you get |
|---------|----------------|
| **BioMistral on-prem** | Clinical LLM via `biomistral-hermes` (~4.4 GB) — [setup guide](docs/MODELS.md) |
| **De-ID v3** | Safe Harbor–oriented 18-category rules, residual risk scoring, optional [Presidio](https://microsoft.github.io/presidio/) hybrid |
| **Policy layer** | Hard-block SSN / email / phone; skill allowlist before any model call |
| **Audit log** | Append-only SQLite (local) or **Postgres** (dev/prod), SHA-256 per entry |
| **RBAC** | `operator` (inference) · `auditor` (audit export) via **OIDC JWT** or `X-Role-Key` |
| **OIDC / SSO** | Keycloak for local dev; map IdP groups → roles — [OIDC.md](docs/OIDC.md) |
| **Environment model** | `HERMES_ENV=local\|dev\|prod` with startup validation |
| **Optional local TLS** | Caddy reverse proxy → `https://localhost:8443` |
| **Observability** | Prometheus, Grafana, Loki — metrics and logs **without PHI** |
| **Rust gateway** | Axum API, single binary, built for regulated environments |

---

## Quick start

**Prerequisites:** [Rust](https://rustup.rs), Docker, `curl`, `jq`

```bash
git clone https://github.com/MarketMadi/hipaa-hermes-agent.git
cd hipaa-hermes-agent
cp .env.example .env
./scripts/setup-biomistral.sh   # pulls BioMistral (~4 GB, one-time)
./scripts/run.sh                 # API + observability + BioMistral
```

Open **http://localhost:8090/demo/** — pick a scenario, click **Ask AI**.

First BioMistral response on CPU may take **45–90 seconds**.

```bash
./scripts/check-demo.sh        # pre-flight before a demo
./scripts/stop.sh              # stop everything
```

| URL | Purpose |
|-----|---------|
| http://localhost:8090/demo/ | Clinician demo UI |
| http://localhost:8090 | API |
| http://localhost:3000 | Grafana (`admin` / `admin`) |
| http://localhost:3000/d/hipaa-hermes-obs/hipaa-hermes-observability | Observability dashboard |
| http://localhost:8180 | Keycloak (when OIDC enabled) |

### Optional: local HTTPS

```bash
./scripts/run-with-tls.sh      # https://localhost:8443/demo/
```

---

## OIDC / SSO

Default auth uses `X-Role-Key` (fine for laptop demos). For workforce identity:

```bash
./scripts/setup-keycloak.sh    # Keycloak on :8180
```

Add to `.env`:

```bash
OIDC_ENABLED=1
OIDC_ISSUER=http://127.0.0.1:8180/realms/hermes
OIDC_AUDIENCE=hermes-api
```

Restart `./scripts/run.sh`, then:

```bash
TOKEN=$(./scripts/get-oidc-token.sh operator operator)
curl -s http://localhost:8090/v1/inference \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"prompt":"de-identified note","skill":"vault-answer"}' | jq .
```

Full guide: [docs/OIDC.md](docs/OIDC.md)

---

## Architecture

```text
┌──────────────┐     ┌─────────────────────────────────────┐     ┌──────────────┐
│  Demo / EHR  │────►│  Hermes (Rust)                      │────►│  BioMistral  │
│  integration │     │  de-ID → policy → RBAC → audit      │     │  (on-prem)   │
└──────────────┘     └──────────────────┬──────────────────┘     └──────────────┘
                                        │
              ┌─────────────────────────┼─────────────────────────┐
              ▼                         ▼                         ▼
         Audit log                 Prometheus               Keycloak (OIDC)
         (SQLite)                  Grafana Loki             Presidio (hybrid de-ID)
```

Full reference: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) · [request flow diagram](docs/diagrams/hipaa-hermes-request-flow.svg)

---

## Configuration

Copy `.env.example` → `.env`. Templates for dev/prod: `.env.dev.example`, `.env.prod.example`.

| Variable | Default | Purpose |
|----------|---------|---------|
| `HERMES_ENV` | `local` | `local` \| `dev` \| `prod` — controls validation strictness |
| `OLLAMA_MODEL` | `biomistral-hermes` | BioMistral-7B — see [docs/MODELS.md](docs/MODELS.md) |
| `LLM_PROVIDER` | `ollama` | Local BioMistral (`ollama`) or `anthropic` (cloud + BAA) |
| `DEID_MODE` | `rules` | `hybrid` adds Presidio NER on `:3001` |
| `DEID_BLOCK_ON_HIGH_RISK` | `0` | Set `1` to block inference when de-ID risk is high |
| `ADMIN_SECRET` | — | Operator `X-Role-Key` (break-glass when OIDC on) |
| `AUDIT_BACKEND` | auto | `sqlite` (local) or `postgres` (dev/prod) — [AUDIT_DB.md](docs/AUDIT_DB.md) |
| `DATABASE_URL` | — | Postgres connection string (required for dev/prod) |
| `OIDC_ENABLED` | `0` | Set `1` for JWT auth — [docs/OIDC.md](docs/OIDC.md) |
| `HERMES_BEHIND_PROXY` | `0` | Set `1` with Caddy TLS overlay |

---

## Tests

```bash
cargo test
```

Includes Safe Harbor de-ID fixture tests for all 18 identifier categories and OIDC role-mapping unit tests.

---

## Documentation

| Doc | Contents |
|-----|----------|
| [MODELS.md](docs/MODELS.md) | BioMistral setup, hardware, troubleshooting |
| [OIDC.md](docs/OIDC.md) | JWT / SSO setup (Keycloak, prod IdP) |
| [AUDIT_DB.md](docs/AUDIT_DB.md) | Postgres audit store, SQLite migration |
| [ARCHITECTURE.md](docs/ARCHITECTURE.md) | Trust zones, HIPAA control mapping |
| [SALES_DEMO.md](docs/SALES_DEMO.md) | 5–8 min live demo talk track |
| [DEPLOYMENT_EPICS.md](docs/DEPLOYMENT_EPICS.md) | Local / dev / prod roadmap |
| [GITHUB.md](docs/GITHUB.md) | Repo polish checklist, topics, launch tips |
| [SCOPE.md](docs/SCOPE.md) | What's shipped vs cut |

---

## Roadmap

| Version | Status | Highlights |
|---------|--------|------------|
| **v3** | ✅ Done | Safe Harbor de-ID, Presidio hybrid, BioMistral, clinician demo UI |
| **v4.0** | ✅ Done | `HERMES_ENV`, config validation, optional local TLS (Caddy) |
| **v4.1** | ✅ Done | OIDC / JWT SSO, Keycloak dev IdP, group → role mapping |
| **v4.2** | ✅ Done | Postgres audit DB (SQLite local), migration tool |
| **v4.3+** | Planned | Vault secrets, dev droplet bootstrap |

Details: [docs/DEPLOYMENT_EPICS.md](docs/DEPLOYMENT_EPICS.md)

---

## Contributing

Contributions welcome — especially de-ID rules, tests, docs, and observability.

See [CONTRIBUTING.md](CONTRIBUTING.md) and [SECURITY.md](SECURITY.md).

If this project helps your regulated-AI work, **consider starring the repo** — it helps others find it.

---

## License

[MIT](LICENSE) — use freely, harden before production PHI.
