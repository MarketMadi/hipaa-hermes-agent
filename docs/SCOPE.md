# v1 scope — §4a.4

**Inference platform v1.** If scope creeps, point here.

---

## In scope (v1)

| Item | Deliverable |
|------|-------------|
| Rust inference gateway | `crates/hermes` — Axum API |
| Anthropic Claude inference | `llm.rs` + `policy.rs` PHI gate |
| Append-only hashed audit log | `audit/` — SHA-256 per entry, **no full hash-chain** |
| Two RBAC roles | `operator` / `auditor` via `X-Role-Key` |
| Observability stack | Prometheus, Grafana, Loki, Promtail — `deploy/docker-compose.yml` |
| Grafana dashboard | `deploy/grafana/hipaa-hermes-observability.json` |
| Terse README | Root `README.md` |
| Demo video plan | `docs/DEMO_VIDEO.md` |
| Reference architecture diagram | `docs/ARCHITECTURE.md` |

---

## v3 (done)

| Item | Deliverable |
|------|-------------|
| Safe Harbor rule engine | `deid/safe_harbor.rs` — 18 category mapping |
| Residual risk scoring | `deid/risk.rs` — low/medium/high + warnings |
| Validation fixtures | `tests/deid_safe_harbor.rs` |
| Presidio hybrid (optional) | `DEID_MODE=hybrid` + Presidio on `:3001` |
| On-prem BioMistral | `biomistral-hermes`, `docs/MODELS.md` |
| Clinician demo UI | `http://localhost:8090/demo/` |

---

## v4 production hardening (epics 1–5 done)

See **[DEPLOYMENT_EPICS.md](./DEPLOYMENT_EPICS.md)** for the full roadmap. Shipped through **v4.3**:

| Epic | Version | Status | Deliverable |
|------|---------|--------|-------------|
| 1 — Environment model | v4.0 | ✅ | `HERMES_ENV`, config validation, env templates |
| 2 — TLS + proxy | v4.0 | ✅ | Optional Caddy overlay (`run-with-tls.sh`) |
| 3 — OIDC / SSO | v4.1 | ✅ | JWT validation, Keycloak dev IdP, group → role |
| 4 — Postgres audit | v4.2 | ✅ | SQLite (local) + Postgres (dev/prod), migration tool |
| 5 — Vault secrets | v4.3 | ✅ | Local Vault + Agent, `run-with-vault.sh` |
| 6 — Containerize Hermes | v4.4+ | Planned | Dockerfile, Compose, `deploy-dev.sh` |
| 7 — Prod topology | v5.0 | Planned | VPC, HA Postgres, DR, CI/CD gates |
| 8 — Compliance packet | v5.0 | Planned | Control mapping, BAA checklist |

**Full local stack guide:** [LOCAL_STACK.md](./LOCAL_STACK.md)

---

## Explicitly cut from v1

| Cut item | Why deferred |
|----------|--------------|
| **Multi-env CI/CD** | Epic 7 — prod deploy pipeline |
| **Case-study doc** | Employment narrative stays in Loom/Upwork |
| Full hash-chain / blockchain audit | Per-entry hash only |
| Slack gateway / live Hermes port | Separate track |
| Public log tunnels | HIPAA violation pattern |
| HIPAA certification claim | Reference architecture only |

---

## Scope creep response

> "That's §4a.4 cut for v1 — multi-env CI / case-study doc are v2+. v1 is Rust gateway, inference, audit log, two roles, observability stack. v4 epics 1–5 are done locally; Epic 6+ is droplet/prod deploy."
