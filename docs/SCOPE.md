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

## v2 (in progress)

| Item | Deliverable |
|------|-------------|
| De-identification pipeline | `deid.rs` — scrub before LLM; audit metadata: `deid_redaction_count`, `deid_categories` |
| Hard-block vs redact | SSN/email/phone blocked on raw input; ages/dates/MRNs/names redacted for allowed requests |
| Demo transparency | `/v1/inference` returns `deidentified_prompt` for operator UI |

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
