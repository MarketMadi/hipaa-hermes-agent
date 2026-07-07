# v1 scope — §4a.4

**Inference platform v1 (Days 3–5).** If scope creeps, point here.

---

## In scope (v1)

| Item | Deliverable |
|------|-------------|
| Append-only hashed audit log | `src/hipaa_hermes/audit.py` — SHA-256 per entry, **no full hash-chain** |
| Two RBAC roles | `operator` (read/write inference + audit append), `auditor` (read audit only) |
| One Grafana dashboard | `deploy/grafana/hipaa-hermes-v1.json` |
| Terse README | Root `README.md` |
| Demo video plan | `docs/DEMO_VIDEO.md` |
| Reference architecture diagram | `docs/ARCHITECTURE.md` (after Q12 gate) |

---

## Explicitly cut from v1

Do **not** add these until v2 unless §4a.4 is formally revised:

| Cut item | Why deferred |
|----------|--------------|
| **Loki** | v1 uses SQLite audit + Grafana JSON import; log shipping is v2 |
| **Multi-env CI/CD** | Single-path local demo; no staging/prod pipeline in v1 |
| **Case-study doc** | Employment narrative stays in Loom/Upwork; not a repo artifact in v1 |
| Full hash-chain / blockchain audit | Per §4a.4 — hashed entries only |
| Slack gateway / live Hermes port | Separate track; this repo is inference platform skeleton |
| Public log tunnels | HIPAA violation pattern; never in this repo |

---

## Scope creep response

> "That's §4a.4 cut for v1 — Loki / multi-env CI / case-study doc are v2. v1 is audit log, two roles, one dashboard, README, demo plan."
