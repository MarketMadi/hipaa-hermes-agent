# Demo video plan — 2–3 minutes (v1)

Record after audit log + RBAC + Grafana JSON exist locally. **Not** a case-study doc — screen proof only.

---

## Setup (before record)

```bash
cd ~/hipaa-hermes-agent
python -m venv .venv && source .venv/bin/activate
pip install -r requirements.txt
cp .env.example .env
mkdir -p data
uvicorn hipaa_hermes.main:app --reload --port 8090
```

Second terminal: import Grafana dashboard from `deploy/grafana/hipaa-hermes-v1.json` (or show JSON + describe metrics).

---

## Beat sheet

| Time | Shot | Say |
|------|------|-----|
| **0:00–0:20** | `docs/ARCHITECTURE.md` trust zones | *"HIPAA-aligned inference gateway — PHI stays in zone C, ops metrics in zone D with no PHI."* |
| **0:20–0:50** | Terminal: `curl` health + audit append as operator | *"Every action hits an append-only audit log — hashed per entry, no delete path."* |
| **0:50–1:20** | Same request with auditor token → read audit; operator token denied on auditor-only path | *"Two roles: operator runs inference, auditor reads audit only."* |
| **1:20–1:50** | Grafana dashboard | *"One dashboard — audit rate, auth failures, request latency. v1 intentionally skips Loki and multi-env CI."* |
| **1:50–2:20** | README + SCOPE.md §4a.4 | *"Descoped on purpose — this is the skeleton you harden, not a certification claim."* |
| **2:20–2:40** | Close | *"Next: policy layer, encrypted Postgres, BAA vendors. This proves audit and access control first."* |

---

## curl script (b-roll)

```bash
# Operator — append inference audit event
curl -s -X POST http://localhost:8090/v1/inference \
  -H "X-Role-Key: $ADMIN_SECRET" \
  -H "Content-Type: application/json" \
  -d '{"prompt":"summarize de-identified note","skill":"vault-answer"}' | jq

# Auditor — read audit (no write)
curl -s http://localhost:8090/v1/audit \
  -H "X-Role-Key: $AUDITOR_SECRET" | jq '.entries | length'

# Operator denied on auditor export
curl -s -o /dev/null -w "%{http_code}" http://localhost:8090/v1/audit/export \
  -H "X-Role-Key: $ADMIN_SECRET"
# expect 403
```

---

## Do not include in video

- Client names (Bayana — follow [PRE_CLIENT.md](./PRE_CLIENT.md))
- Production credentials or real PHI
- Claims of HIPAA certification
- Loki / multi-env CI (cut from v1)

---

## Output

- [ ] 2–3 min Loom or MP4
- [ ] Thumbnail: architecture trust-zone slide or Grafana panel
- [ ] Link from Upwork portfolio when repo is public
