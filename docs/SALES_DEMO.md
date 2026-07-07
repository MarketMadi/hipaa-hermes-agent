# Sales demo playbook — HIPAA Hermes

## What is this, in hospital terms?

**This is not an EHR and not a HIPAA certificate.**

It emulates the **AI gateway** a hospital would deploy **behind** Epic/Cerner/Athena:

```
Clinician in EHR  →  [Hermes gateway]  →  Local LLM (Ollama, on-prem)
                         │
                    de-ID + policy (minimized payload)
                    RBAC (who can run AI vs read audit)
                    append-only audit log
                    metrics + logs (no raw chart text)
```

**Why local-first for hospitals:** after de-identification, inference runs on infrastructure you control — no PHI crosses to a cloud LLM vendor, no inference-time BAA with OpenAI/Anthropic, works air-gapped.

**Today you can touch:**
- **http://localhost:8090/demo/** — clinician-style UI with synthetic discharge notes and lab vignettes
- **Grafana** — prove audit, logs, and metrics for compliance/ops buyers

**Production would add:** SSO, real EHR integration, encrypted Postgres, contracted BAA, formal risk assessment.

---

**Audience:** Technical buyers (CTO, platform lead, compliance engineer)  
**Duration:** 5–8 minutes live, or 2–3 min recorded  
**You need:** Laptop with Docker, this repo cloned, `.env` configured once by engineering

---

## Before the call (5 min setup)

### One-time (ask Dave if stuck)

```bash
cd ~/hipaa-hermes-agent
cp .env.example .env
./scripts/setup-ollama.sh   # one-time: pulls local model
chmod +x scripts/*.sh
```

### Every demo

**Terminal 1** — start everything:

```bash
./scripts/run.sh
```

Wait until you see `listening addr=0.0.0.0:8090`.

**Terminal 2** — verify ready:

```bash
./scripts/check-demo.sh
```

If all green, open Grafana: http://localhost:3000/d/hipaa-hermes-obs/hipaa-hermes-observability  
Login: `admin` / `admin`

**After the call:**

```bash
./scripts/stop.sh
```

---

## Credentials cheat sheet (safe to show on screen)

| What | Value |
|------|-------|
| Grafana | http://localhost:3000 — `admin` / `admin` |
| Operator key | `change-me-operator` |
| Auditor key | `change-me-auditor` |
| Dashboard | HIPAA Hermes Observability |

Do **not** show: `.env` file, API keys, real patient data.

---

## The story (memorize this arc)

> *"This is not a HIPAA certification — it's a reference gateway that shows how regulated AI workloads should be controlled: policy before the model, every action audited, roles separated, ops visibility without PHI in logs."*

---

## Act 1 — Clinician UI (90 sec) — start here

**Open:** http://localhost:8090/demo/

1. Pick **"Discharge summary"** → click **Ask AI**
2. Show the response + `audit_id` at the bottom
3. Pick **"Policy demo — contains SSN"** → Ask AI → **blocked**

**Say:**

> *"This is what sits behind the EHR button. Chart text is de-identified inside the gateway, then inference runs on a local model — nothing leaves the hospital network. Every request is policy-checked and audit-logged without storing the raw prompt."*

**Show Grafana:** audit table + Loki logs.

---

## Act 2 — Architecture (30 sec) — optional if short on time

**Show:** `docs/ARCHITECTURE.md` or the trust-zone diagram in Grafana/README.

**Say:**

> *"PHI stays in the private zone. De-identified text goes to a model running on-prem — not a cloud API. Ops metrics and logs live in a separate zone with no patient content."*

---

## Act 3 — Happy path inference via terminal (optional)

**Run** (Terminal 2):

```bash
./scripts/sales-demo.sh
```

Or manually:

```bash
curl -s -X POST http://localhost:8090/v1/inference \
  -H "X-Role-Key: change-me-operator" \
  -H "Content-Type: application/json" \
  -d '{"prompt":"Summarize this de-identified discharge note: patient improved and was discharged.","skill":"vault-answer"}' | jq
```

**Say:**

> *"Operator role runs inference. Notice the response includes an audit ID and hash — proof the action was recorded. The prompt text is not stored in the audit log, only metadata like length and latency."*

**Show in Grafana:** Audit table panel — new row with `hash_valid: true`.

---

## Act 3 — Policy blocks PHI (60 sec) — the wow moment

```bash
curl -s -w "\nHTTP %{http_code}\n" -X POST http://localhost:8090/v1/inference \
  -H "X-Role-Key: change-me-operator" \
  -H "Content-Type: application/json" \
  -d '{"prompt":"Patient John Doe SSN 123-45-6789 needs follow-up","skill":"vault-answer"}' | jq
```

**Expected:** HTTP `403`, `"request blocked by policy: phi_pattern_detected"`

**Say:**

> *"SSN patterns never reach the LLM. The block is audited too — compliance can prove the gate fired."*

**Show in Grafana:** Audit table — row with `outcome: blocked`. Loki logs — `inference blocked by policy`.

---

## Act 4 — RBAC separation of duties (60 sec)

```bash
# Auditor reads audit — works
curl -s http://localhost:8090/v1/audit \
  -H "X-Role-Key: change-me-auditor" | jq '.entries | length'

# Operator cannot bulk-export — 403
curl -s -o /dev/null -w "HTTP %{http_code}\n" \
  http://localhost:8090/v1/audit/export \
  -H "X-Role-Key: change-me-operator"
```

**Say:**

> *"The person who runs AI cannot export the compliance trail. The auditor can export but cannot trigger inference. That's separation of duties at the API boundary."*

---

## Act 5 — Observability without PHI (60 sec)

**Show Grafana dashboard:**

1. **Metrics row** — audit count, auth failures, latency
2. **Application logs** — request paths and outcomes, no prompt content
3. **Audit table** — compliance trail

**Say:**

> *"Prometheus for metrics, Loki for operational logs, audit API for the compliance record. Three layers, none of them store raw PHI in this demo."*

---

## Act 6 — Close (30 sec)

**Say:**

> *"This is the control-plane skeleton — de-ID, policy, on-prem inference, RBAC, audit, observability. Production adds GPU sizing, model governance, encrypted Postgres, OIDC, and your org's formal risk assessment. Today we proved the pattern works with data staying local."*

**If they ask what's NOT included:** encrypted storage, SSO, Slack gateway, HIPAA certification, multi-env CI — all v2 / deployment hardening.

---

## FAQ — quick answers

| Question | Answer |
|----------|--------|
| Is this HIPAA certified? | No — reference architecture + primitives you harden. |
| What LLM? | **Ollama on-prem** by default (Docker). Optional cloud via `LLM_PROVIDER=anthropic`. |
| Where's PHI stored? | Not in audit or logs — only metadata. Inference payload stays on-host after de-ID. |
| Can we use OpenAI / Azure? | Gateway is vendor-agnostic. Local Ollama today; cloud providers are a config switch. |
| Is this production-ready? | Skeleton — proves controls; needs hardening for prod. |
| What client is this from? | Generic reference — no client names in the repo. |

---

## Troubleshooting

| Problem | Fix |
|---------|-----|
| `connection refused` on :8090 | Run `./scripts/run.sh` in Terminal 1 |
| Grafana empty | Set time range to "Last 15 minutes"; run `./scripts/sales-demo.sh` |
| Inference returns `[stub:...]` | `ANTHROPIC_API_KEY` missing — ask Dave, or demo policy/RBAC anyway |
| Docker errors | Run `docker ps` — need Docker running |
| Port in use | `./scripts/stop.sh` then `./scripts/run.sh` |

---

## Guided script

For a walkthrough with pauses and on-screen prompts:

```bash
./scripts/sales-demo.sh
```

Pre-flight check only:

```bash
./scripts/check-demo.sh
```

Open browser tabs:

```bash
./scripts/open-demo.sh
```
