# Full local stack (v4.3)

Guide for running the **complete** HIPAA Hermes local environment: BioMistral inference, hybrid de-ID (Presidio), OIDC (Keycloak), Postgres audit, and Vault secrets — the same control-plane path used for dev/prod, on your laptop.

> **Not required for a quick demo.** For a minimal start, see the [root README](../README.md#quick-start). Use this guide when you want the full v4.3 stack verified end-to-end.

---

## What's included

| Component | Port | Purpose |
|-----------|------|---------|
| Hermes API | `:8090` | Rust gateway — de-ID, policy, RBAC, audit |
| BioMistral (Ollama) | `:11434` | On-prem clinical LLM |
| Presidio analyzer | `:3001` | Hybrid NER de-ID (`DEID_MODE=hybrid`) |
| Keycloak | `:8180` | OIDC IdP for JWT auth |
| Postgres | `:5433` | Append-only audit store (host `:5432` may be in use) |
| Vault | `:8200` | Local secrets injection via Vault Agent |
| Grafana | `:3000` | Dashboards (`admin` / `admin`) |
| Prometheus | `:9090` | Metrics |
| Loki | `:3100` | Logs (no PHI) |

`/health` reports the active backends:

```json
{
  "status": "ok",
  "version": "0.3.0",
  "audit_backend": "postgres",
  "secrets_source": "vault"
}
```

---

## One-time setup

**Prerequisites:** [Rust](https://rustup.rs), Docker, `curl`, `jq`

```bash
git clone https://github.com/MarketMadi/hipaa-hermes-agent.git
cd hipaa-hermes-agent
cp .env.example .env
chmod +x scripts/*.sh
```

Edit `.env` for the full stack (or uncomment the block in `.env.example`):

```bash
HERMES_ENV=local

# Postgres audit (local demo of dev/prod path)
AUDIT_BACKEND=postgres
DATABASE_URL=postgres://hermes:hermes@127.0.0.1:5433/hermes_audit

# Hybrid de-ID
DEID_MODE=hybrid
DEID_NER_URL=http://127.0.0.1:3001
DEID_BLOCK_ON_HIGH_RISK=1

# OIDC
OIDC_ENABLED=1
OIDC_ISSUER=http://127.0.0.1:8180/realms/hermes
OIDC_AUDIENCE=hermes-api
OIDC_ALLOW_ROLE_KEY=1

# BioMistral
LLM_PROVIDER=ollama
OLLAMA_MODEL=biomistral-hermes
LLM_FALLBACK_STUB=1
```

Bootstrap each service (order matters on first run):

```bash
./scripts/setup-biomistral.sh    # pulls BioMistral (~4 GB, one-time)
./scripts/setup-postgres.sh      # Postgres on :5433 + schema
./scripts/setup-keycloak.sh      # Keycloak on :8180 + realm import
./scripts/setup-vault.sh         # Vault :8200, seeds secrets from .env
```

If you have existing SQLite audit data, migrate once:

```bash
./scripts/migrate-audit-to-postgres.sh
```

---

## Every session

```bash
./scripts/run-with-vault.sh      # API + observability + BioMistral + Presidio
```

In another terminal:

```bash
./scripts/check-demo.sh          # pre-flight — all checks should be green
```

Open **http://localhost:8090/demo/** for the clinician UI.

First BioMistral response on CPU may take **45–90 seconds** after a cold start.

Stop everything:

```bash
./scripts/stop.sh
```

---

## Verify everything works

### Automated

```bash
cargo test                       # 26 tests (unit + API integration)
./scripts/check-demo.sh          # live stack pre-flight
```

### Manual smoke tests

```bash
# Health
curl -s http://localhost:8090/health | jq .

# OIDC RBAC — operator denied export
TOKEN=$(./scripts/get-oidc-token.sh operator operator)
curl -s -o /dev/null -w "%{http_code}\n" \
  http://localhost:8090/v1/audit/export -H "Authorization: Bearer $TOKEN"
# → 403

# OIDC RBAC — auditor allowed export
AUDITOR=$(./scripts/get-oidc-token.sh auditor auditor)
curl -s -o /dev/null -w "%{http_code}\n" \
  http://localhost:8090/v1/audit/export -H "Authorization: Bearer $AUDITOR"
# → 200

# Policy gate — SSN blocked
curl -s -o /dev/null -w "%{http_code}\n" -X POST http://localhost:8090/v1/inference \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"prompt":"Patient SSN 123-45-6789","skill":"clinical_summary"}'
# → 403

# Audit hash integrity
curl -s "http://localhost:8090/v1/audit?limit=3" \
  -H "Authorization: Bearer $AUDITOR" | jq '.entries[] | {id, action, hash_valid}'
# → hash_valid: true for each entry
```

---

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| `check-demo.sh` fails on Presidio | `docker restart deploy-presidio-analyzer-1` — gunicorn can hang after long uptime |
| Postgres connection refused on `:5432` | Use `:5433` (see `deploy/docker-compose.postgres.yml`) |
| `secrets_source` is not `vault` | Run `./scripts/setup-vault.sh` then `./scripts/run-with-vault.sh` |
| BioMistral very slow | Expected on CPU; first request after restart is slowest |
| API not responding | Check nothing else is on `:8090`; `./scripts/stop.sh` then restart |
| OIDC token fetch fails | `./scripts/setup-keycloak.sh` and wait for Keycloak on `:8180` |

---

## Known quirks

1. **Presidio health** — Docker may show `unhealthy` after ~1 hour even though the process is running. Restart the container; consider adding a restart policy in Compose for long demos.

2. **OIDC + break-glass** — With `OIDC_ENABLED=1`, a valid operator `X-Role-Key` on auditor-only endpoints may return **401** instead of **403**. JWT auth path is correct; only affects break-glass scripts.

3. **Export vs list audit** — `/v1/audit/export` returns a slim format without `hash_valid`; `/v1/audit` includes per-entry hash verification.

4. **Vault dev mode** — Fixed root token, in-memory storage. **Never** use bundled Vault config outside local laptops.

---

## Related docs

- [OIDC.md](OIDC.md) — JWT / SSO setup
- [AUDIT_DB.md](AUDIT_DB.md) — Postgres audit, migration
- [VAULT.md](VAULT.md) — Secrets injection
- [SALES_DEMO.md](SALES_DEMO.md) — Live demo talk track
- [DEPLOYMENT_EPICS.md](DEPLOYMENT_EPICS.md) — Roadmap (Epic 6+)
