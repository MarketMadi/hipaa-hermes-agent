# Vault secrets (Epic 5)

Hermes reads **secrets from environment variables at startup** — no code changes per secret. For local dev, **Vault Agent** renders `data/vault/hermes.env` from `secret/hermes/local` so you don't keep API keys and DB passwords in `.env`.

> **Local only:** the bundled Vault runs in **`-dev` mode** with a fixed root token. Never use this in dev droplets or production.

---

## What goes in Vault vs `.env`

| Vault (`secret/hermes/local`) | `.env` (non-secret config) |
|-------------------------------|----------------------------|
| `ADMIN_SECRET` | `HERMES_ENV`, `LLM_PROVIDER`, `OLLAMA_*` |
| `AUDITOR_SECRET` | `DEID_*`, `OIDC_*` (non-secret), `BIND_*` |
| `DATABASE_URL` | `AUDIT_BACKEND`, `DATABASE_PATH` |
| `ANTHROPIC_API_KEY` | feature flags, URLs |

---

## Quick start (local)

```bash
cp .env.example .env          # keep non-secret settings
./scripts/setup-vault.sh      # Vault :8200 + seed secrets from .env
./scripts/run-with-vault.sh   # sources data/vault/hermes.env → starts Hermes
```

Verify:

```bash
curl -s http://localhost:8090/health | jq .
# → "secrets_source": "vault"
```

---

## Architecture

```text
.env (config) ──┐
                ├──► run-with-vault.sh ──► hermes (reads env at startup)
Vault Agent ────┘         ▲
     ▲                      │
     │ renders               │
data/vault/hermes.env       │
     ▲                      │
Vault :8200 ◄── secret/hermes/local
```

---

## Paths and policy

| Path | Purpose |
|------|---------|
| `secret/hermes/local` | Laptop demos (`HERMES_ENV=local`) |
| `secret/hermes/dev` | Dev droplet (future) |
| `secret/hermes/prod` | Production (future) |

Policy sketch: `deploy/vault/policies/hermes-local.hcl`

Vault Agent renders inside the container; `vault-fetch-secrets.sh` copies to a host-owned `data/vault/hermes.env` (mode 600).

---

Edit `.env`, re-seed Vault, restart Hermes:

```bash
./scripts/setup-vault.sh    # re-runs kv put from .env
./scripts/run-with-vault.sh
```

Or use the Vault CLI inside the container:

```bash
docker exec -e VAULT_TOKEN=hermes-dev-root -e VAULT_ADDR=http://127.0.0.1:8200 \
  $(docker compose -f deploy/docker-compose.yml -f deploy/docker-compose.vault.yml ps -q vault) \
  vault kv put secret/hermes/local admin_secret=new-value auditor_secret=...
```

---

## Rotation runbook (local)

1. Generate new secret values.
2. `vault kv put secret/hermes/local ...` (or re-run `setup-vault.sh` after updating `.env`).
3. Vault Agent re-renders `hermes.env` within ~1s.
4. Restart Hermes: `./scripts/run-with-vault.sh`.

For dev/prod: use versioned KV, AppRole auth (not root token), and automated rotation via your secrets platform.

---

## Dev / prod (not implemented here)

| Local (this doc) | Dev / prod |
|------------------|------------|
| Vault `-dev` + root token | HCP Vault or self-hosted HA cluster |
| Token in `agent.hcl` | **AppRole** + short-lived tokens |
| `setup-vault.sh` seeds from `.env` | CI/CD or Terraform writes secrets |
| Optional overlay | **Required** — no `.env` secrets on disk |

Alternatives: AWS Secrets Manager, GCP Secret Manager, DigitalOcean Secrets.

---

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| `data/vault/hermes.env` missing | `./scripts/setup-vault.sh` then `./scripts/vault-fetch-secrets.sh` |
| `permission denied` on secrets file | `chmod 600 data/vault/hermes.env` |
| Agent logs show auth errors | Ensure Vault is up: `curl $VAULT_ADDR/v1/sys/health` |
| Health shows `"secrets_source":"env"` | Use `./scripts/run-with-vault.sh`, not `./scripts/run.sh` |

---

## Related

- [DEPLOYMENT_EPICS.md](DEPLOYMENT_EPICS.md) — Epic 5 full scope
- [AUDIT_DB.md](AUDIT_DB.md) — `DATABASE_URL` in Vault
