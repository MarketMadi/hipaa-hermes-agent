# Deployment epics — local, dev, prod

Roadmap for taking HIPAA Hermes from **laptop demo** → **dev droplet** → **cloud prod**, without claiming HIPAA certification.

**Current state (v4.3):** Rust gateway, de-ID v3, RBAC via `X-Role-Key` **or OIDC JWT**, **SQLite (local) / Postgres (dev/prod) audit**, optional **Vault** for local secrets, HTTP on `:8090`, Docker Compose observability + BioMistral + Presidio + optional Keycloak/Postgres/Vault.

---

## Three environments

| Dimension | **Local** | **Dev** (e.g. DO droplet) | **Prod** (cloud) |
|-----------|-----------|---------------------------|------------------|
| **Purpose** | Sales demo, feature work | Integration testing, buyer sandbox, staging | Regulated workload (pilot → GA) |
| **PHI** | Synthetic only | Synthetic / anonymized test data | Real or de-identified per contract |
| **Hermes API** | `localhost:8090` | `https://hermes-dev.example.com` | `https://hermes.example.com` (private or VPN) |
| **TLS** | Optional (mkcert / Caddy) | **Required** — Let's Encrypt | **Required** — LE or cloud LB cert |
| **Reverse proxy** | Optional Caddy in Compose | **Caddy or nginx** — sole public ingress | ALB/nginx/Caddy + WAF optional |
| **Auth** | `X-Role-Key` (keep for scripts) | **OIDC** + optional break-glass keys | **OIDC + MFA** enforced |
| **Audit DB** | SQLite (current) | **Postgres** (encrypted volume) | **Postgres** (encrypted, HA, backups) |
| **Secrets** | `.env` (gitignored) | **Vault Agent** → env at runtime | **Vault** or cloud SM (no `.env` on disk) |
| **LLM** | BioMistral on laptop | BioMistral on GPU droplet | Dedicated GPU node / on-prem link; cloud LLM only with BAA |
| **Grafana** | `localhost:3000` open | HTTPS + **IP allowlist / VPN** | VPN or SSO-only; not public internet |
| **Presidio / inference runtime** | Host network, local only | **Not exposed** — internal Docker network | Internal only; no public ports |
| **Deploy mechanism** | `./scripts/run.sh` | Compose + systemd **or** single VM image | Compose, K8s, or managed containers |
| **IaC** | None | Terraform/Ansible for droplet + DNS | Full IaC + CI/CD gates |

```text
                    ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
  Clinician/EHR ──► │ TLS proxy   │ ──► │   Hermes    │ ──► │ BioMistral  │
                    │ (Caddy/LB)  │     │  gateway    │     │  (private)  │
                    └─────────────┘     └──────┬──────┘     └─────────────┘
                           │                  │
                     OIDC / JWT          Audit DB
                           │             (encrypted)
                     ┌─────▼─────┐       ┌──────▼──────┐
                     │   IdP     │       │ Vault / SM  │
                     └───────────┘       └─────────────┘
```

---

## Epic list (recommended order)

Work top-to-bottom. Each epic notes **L / D / P** = what changes per environment.

### Epic 1 — Environment model & config foundation

**Why first:** Every later epic keys off `HERMES_ENV` and validated config.

| Task | L | D | P |
|------|---|---|---|
| Add `HERMES_ENV=local\|dev\|prod` | default `local` | `dev` | `prod` |
| Split env templates: `.env.example`, `.env.dev.example`, `.env.prod.example` | ✓ | ✓ | ✓ |
| Startup validation (fail prod if `ADMIN_SECRET=change-me-*`, HTTP-only bind, etc.) | warn | error | error |
| Compose profiles: `local`, `dev`, `prod` overlays | base | +proxy +vault | +proxy +vault +stricter |

**Deliverables:** `config.rs` env checks, `deploy/compose/` layout, docs in this file.

---

### Epic 2 — TLS + reverse proxy

**Why:** HIPAA transmission security; prod cannot ship plain HTTP. Dev droplet needs HTTPS for realistic OIDC redirects.

| Task | L | D | P |
|------|---|---|---|
| Add **Caddy** (or nginx) as single ingress | optional profile | required | required |
| Hermes binds `127.0.0.1:8090` only when proxy enabled | ✓ | ✓ | ✓ |
| TLS certs | mkcert / self-signed | Let's Encrypt (`hermes-dev.*`) | LE or cloud-managed |
| Route map: `/` → Hermes, `/grafana` → Grafana (subpath or subdomain) | same | subdomain preferred | `grafana.internal.*` VPN-only |
| HSTS, modern TLS (1.2+), security headers | optional | ✓ | ✓ |
| Health checks through proxy (`/health`) | ✓ | ✓ | ✓ |

**Suggested layout:**

```text
deploy/
  caddy/
    Caddyfile.local      # localhost TLS optional
    Caddyfile.dev        # ACME for dev hostname
    Caddyfile.prod       # prod hostnames + stricter headers
  compose/
    docker-compose.yml          # current stack (internal)
    docker-compose.proxy.yml    # Caddy overlay
```

**Cloud note:** On AWS/GCP/DO you can terminate TLS at the **load balancer** instead of Caddy; Hermes stays private. Caddy on the droplet is fine for dev; prod may use ALB + target group.

---

### Epic 3 — OIDC / SSO ✅ (v4.1)

**Why:** Shared `X-Role-Key` is not workforce identity; required for any real hospital pilot.

| Task | L | D | P |
|------|---|---|---|
| JWT validation middleware (issuer, audience, expiry) | mock IdP in Compose | Auth0 / Keycloak / Okta dev tenant | Hospital IdP or enterprise SSO |
| Map IdP groups → `operator` / `auditor` | config map | config map | IAM-owned group sync |
| Keep `X-Role-Key` as break-glass (dev only, disabled in prod) | ✓ | optional | **off** |
| Audit log records `sub` / email from JWT, not shared key | ✓ | ✓ | ✓ |
| Demo UI: Bearer token from localStorage | ✓ | ✓ | ✓ |

**Delivered:** `crates/hermes/src/oidc.rs`, Keycloak overlay (`deploy/docker-compose.oidc.yml`), `scripts/setup-keycloak.sh`, `scripts/get-oidc-token.sh`, [OIDC.md](OIDC.md).

**Local dev IdP:** `./scripts/setup-keycloak.sh` → Keycloak on `:8180`.

**Env vars:**

```bash
OIDC_ENABLED=1
OIDC_ISSUER=http://127.0.0.1:8180/realms/hermes
OIDC_AUDIENCE=hermes-api
OIDC_OPERATOR_GROUPS=hermes-operator
OIDC_AUDITOR_GROUPS=hermes-auditor
OIDC_ALLOW_ROLE_KEY=1   # prod: 0
```

---

### Epic 4 — Encrypted audit database ✅ (v4.2)

**Why:** Integrity + confidentiality at rest (Security Rule); SQLite on disk is a demo gap.

| Task | L | D | P |
|------|---|---|---|
| **Phase A:** SQLCipher-backed SQLite (`DATABASE_URL` + key from Vault) | optional | ✓ | transitional |
| **Phase B:** Postgres for dev/prod | — | DO Managed PG / droplet PG | RDS/Cloud SQL, encryption at rest **on** |
| Migration tool: SQLite → Postgres one-way | — | ✓ | ✓ |
| Backup: encrypted snapshots, retention policy | — | daily | daily + PITR |
| Connection pooling (e.g. `sqlx` + Postgres) | — | ✓ | ✓ |
| No PHI in audit metadata (already true) | ✓ | ✓ | ✓ |

**Delivered:** `sqlx` Postgres backend, SQLite for local, `migrate-audit` binary, `deploy/docker-compose.postgres.yml`, [AUDIT_DB.md](AUDIT_DB.md).

**Recommendation:** Stay on SQLite for **local** speed; **require Postgres** for dev and prod. Encryption at rest comes free with managed Postgres.

---

### Epic 5 — Vault (secrets management) ✅ (v4.3 local)

**Why:** `.env` on a droplet or cloud VM is a finding in any security review.

| Task | L | D | P |
|------|---|---|---|
| Vault dev server in Compose (local only) | optional profile | — | — |
| Vault Agent sidecar: inject `ADMIN_SECRET`, DB URL, `ANTHROPIC_API_KEY` | ✓ | ✓ | ✓ |
| App reads secrets from env at startup (no change to call sites) | ✓ | ✓ | ✓ |
| Policy: path `secret/hermes/{env}/*` | `local` | `dev` | `prod` |
| Rotation runbook for API keys and DB passwords | — | doc | automated |
| **Alternative on cloud:** DO Secrets, AWS SM, GCP SM instead of self-hosted Vault | — | acceptable | acceptable |

**Delivered (local):** `deploy/docker-compose.vault.yml`, Vault Agent → `data/vault/hermes.env`, `scripts/setup-vault.sh`, `scripts/run-with-vault.sh`, [VAULT.md](VAULT.md).

**Local:** `.env` for non-secret config; secrets in Vault. **Dev/prod:** AppRole + managed secrets (future).

---

### Epic 6 — Multi-env Compose & droplet bootstrap

**Why:** Repeatable dev deploy on a DigitalOcean droplet (or similar).

| Task | L | D | P |
|------|---|---|---|
| `scripts/deploy-dev.sh` — pull, compose up, migrate DB | — | ✓ | — |
| Terraform: droplet, firewall (22 from your IP, 443 public, 8090 **closed**), DNS A record | — | ✓ | optional |
| Firewall: inference runtime `:11434`, Presidio `:3001`, Prometheus **not** public | — | ✓ | ✓ |
| Systemd unit for Hermes binary (restart on failure) | optional | ✓ | ✓ |
| `HERMES_ENV=dev` injected by deploy script | — | ✓ | ✓ |

**Droplet sizing (dev):** 8 GB RAM minimum for BioMistral CPU inference; 16 GB preferred. GPU droplet if you want faster clinical demos.

---

### Epic 7 — Prod cloud topology

**Why:** Prod is not “bigger dev” — different network, HA, and compliance expectations.

| Task | L | D | P |
|------|---|---|---|
| Separate VPC / private subnet for Hermes + BioMistral | — | — | ✓ |
| LLM: on-prem GPU **or** cloud GPU in same VPC as Hermes | — | optional | ✓ |
| Cloud LLM (Anthropic) only with **signed BAA** + `LLM_PROVIDER=anthropic` | — | test | contract-gated |
| Grafana/Loki: private endpoint or managed observability | — | VPN | SSO + no PHI in logs |
| CI/CD: build image, scan, deploy to prod with approval gate | — | auto from `main` | manual promote |
| Disaster recovery: RPO/RTO targets, restore drill | — | — | ✓ |

**Deployment patterns (pick one):**

1. **Single hardened VM** (simplest pilot) — Caddy + Compose, like dev but bigger + HA Postgres.
2. **K8s** (EKS/GKE/DO KS) — Hermes Deployment, BioMistral StatefulSet with GPU node pool, Ingress + cert-manager.
3. **Split plane** — Hermes in cloud, BioMistral on hospital on-prem via private link (best HIPAA story for inference).

---

### Epic 8 — Compliance & docs (no certification claims)

| Task | When |
|------|------|
| Update `ARCHITECTURE.md` trust zones for 3-env diagram | After Epic 2 |
| One-pager: control mapping + known gaps per environment | Before prod pilot |
| BAA checklist (BioMistral on-prem N/A; Anthropic/cloud host/IdP) | Before prod |
| Incident response / breach notification pointer (customer-owned) | Prod |
| Refresh `SCOPE.md` — v3 done, epics 1–8 = v4 production hardening | Now |

---

## Suggested phasing

| Phase | Epics | Target |
|-------|-------|--------|
| **v4.0** | 1, 2 | TLS proxy works local + dev droplet |
| **v4.1** | 3 | OIDC on dev; break-glass keys documented |
| **v4.2** | 4, 5 | Postgres audit + Vault on dev |
| **v4.3** | 6 | One-command dev droplet deploy |
| **v5.0** | 7, 8 | Prod topology + compliance packet |

---

## What stays the same across all envs

- De-ID before LLM (rules + optional Presidio hybrid)
- Policy gate (hard-block patterns, skill ACL)
- Append-only audit semantics (no delete API)
- RBAC semantics (`operator` vs `auditor`) — only the **credential mechanism** changes
- Local-first BioMistral story for demos; prod may add on-prem link
- **Not a HIPAA certificate** in any environment

---

## Quick reference — ports (after Epic 2)

| Service | Internal | Public (dev/prod) |
|---------|----------|-------------------|
| Hermes API | `127.0.0.1:8090` | via proxy `:443` only |
| Grafana | `:3000` | `grafana-dev.*` or VPN |
| Presidio | `:3001` | **never** |
| Inference runtime | `:11434` | **never** |
| Prometheus / Loki | `:9090` / `:3100` | **never** (VPN or SSH tunnel) |
| Vault | `:8200` | **never** |

---

## Related docs

- [ARCHITECTURE.md](./ARCHITECTURE.md) — trust zones & control mapping
- [SCOPE.md](./SCOPE.md) — what's in / out of v1–v3
- [SALES_DEMO.md](./SALES_DEMO.md) — local demo playbook
- [MODELS.md](./MODELS.md) — BioMistral sizing per hardware
