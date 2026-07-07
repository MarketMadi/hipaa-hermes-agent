# Demo video plan — 2–3 minutes (v1)

Record after Rust API + observability stack run locally. **Not** a case-study doc — screen proof only.

---

## Setup (before record)

```bash
cd ~/hipaa-hermes-agent
cp .env.example .env
# Set ANTHROPIC_API_KEY in .env for real Claude output
chmod +x scripts/run.sh scripts/demo.sh
./scripts/run.sh
```

Second terminal: `./scripts/demo.sh`

---

## Beat sheet

| Time | Shot | Say |
|------|------|-----|
| **0:00–0:20** | `docs/ARCHITECTURE.md` trust zones | *"HIPAA-aligned inference gateway — PHI stays in zone C, ops metrics in zone D with no PHI."* |
| **0:20–0:50** | Terminal: `curl` health + inference as operator | *"Real Claude inference behind a PHI policy gate — blocked patterns never reach the model."* |
| **0:50–1:20** | Auditor read audit; operator 403 on export | *"Two roles: operator runs inference, auditor reads audit only."* |
| **1:20–1:50** | Grafana — metrics, Loki logs, audit table | *"Prometheus metrics, Loki app logs, and compliance audit trail — one dashboard."* |
| **1:50–2:20** | README + SCOPE.md | *"Rust gateway, not a certification claim — the skeleton you harden."* |
| **2:20–2:40** | Close | *"Next: encrypted Postgres, OIDC, BAA vendor config. This proves policy, audit, and access control first."* |

---

## curl script (b-roll)

```bash
# Operator — inference (real Claude if ANTHROPIC_API_KEY set)
curl -s -X POST http://localhost:8090/v1/inference \
  -H "X-Role-Key: $ADMIN_SECRET" \
  -H "Content-Type: application/json" \
  -d '{"prompt":"summarize de-identified note","skill":"vault-answer"}' | jq

# Policy block demo
curl -s -w "\nHTTP %{http_code}\n" -X POST http://localhost:8090/v1/inference \
  -H "X-Role-Key: $ADMIN_SECRET" \
  -H "Content-Type: application/json" \
  -d '{"prompt":"patient SSN 123-45-6789","skill":"vault-answer"}'

# Auditor — read audit
curl -s http://localhost:8090/v1/audit \
  -H "X-Role-Key: $AUDITOR_SECRET" | jq '.entries | length'
```

---

## Do not include in video

- Client names (follow [PRE_CLIENT.md](./PRE_CLIENT.md))
- Production credentials or real PHI
- Claims of HIPAA certification

---

## Output

- [ ] 2–3 min Loom or MP4
- [ ] Thumbnail: architecture trust-zone slide or Grafana panel
- [ ] Link from Upwork portfolio when repo is public
