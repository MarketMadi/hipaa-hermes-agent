# v1 scope — §4a.4

**Inference platform v1.** If scope creeps, point here.

---

## In scope (v1)

| Item | Deliverable |
|------|-------------|
| Rust inference gateway | `crates/hermes` — Axum API |
| Anthropic Claude inference | `llm.rs` + `policy.rs` PHI gate |
| Append-only hashed audit log | `audit.rs` — SHA-256 per entry, **no full hash-chain** |
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

## v4+ (production hardening)

See **[DEPLOYMENT_EPICS.md](./DEPLOYMENT_EPICS.md)** — TLS/proxy, OIDC, encrypted audit DB, Vault, local / dev droplet / prod cloud.

---

## Explicitly cut from v1

| Cut item | Why deferred |
|----------|--------------|
| **Multi-env CI/CD** | Single-path local demo |
| **Case-study doc** | Employment narrative stays in Loom/Upwork |
| Full hash-chain / blockchain audit | Per-entry hash only |
| Slack gateway / live Hermes port | Separate track |
| Public log tunnels | HIPAA violation pattern |
| Encrypted Postgres / secrets vault | v2 production hardening |

---

## Scope creep response

> "That's §4a.4 cut for v1 — multi-env CI / case-study doc are v2. v1 is Rust gateway, Claude inference, audit log, two roles, observability stack."
