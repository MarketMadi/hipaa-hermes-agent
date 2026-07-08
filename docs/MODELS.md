# BioMistral — local clinical inference

Hermes runs **BioMistral-7B** on-prem after de-identification. The default model tag is **`biomistral-hermes`** — our repackaged build with a working Mistral chat template.

> **Implementation note:** weights are served by a local [Ollama](https://ollama.com) container (`deploy/docker-compose.yml`). You configure `OLLAMA_MODEL=biomistral-hermes`; users and buyers should think **BioMistral**, not “generic Ollama.”

## Quick start

```bash
./scripts/setup-biomistral.sh
```

In `.env`:

```bash
LLM_PROVIDER=ollama
OLLAMA_MODEL=biomistral-hermes
```

Restart: `./scripts/stop.sh && ./scripts/run.sh`

First inference on CPU often takes **45–90 seconds**.

## Why `biomistral-hermes`?

Community BioMistral tags on Ollama (e.g. `adrienbrault/biomistral-7b:Q4_K_M`) ship a **broken chat template** → empty model responses. Setup pulls those weights once, then creates **`biomistral-hermes`** with a correct `[INST]` template.

**Do not** point `.env` at the raw `adrienbrault/…` tag.

## Hardware

| | BioMistral (`biomistral-hermes`) |
|---|----------------------------------|
| **Disk** | ~4.4 GB |
| **RAM** | 8 GB+ (16 GB laptop with Presidio hybrid is tight) |
| **First request** | ~45–90 s on CPU |

GPU speeds inference significantly. On constrained laptops, set `DEID_MODE=rules` to skip Presidio (~750 MB RAM).

## Alternatives

| Use case | `OLLAMA_MODEL` | Notes |
|----------|----------------|-------|
| **Clinical on-prem (default)** | `biomistral-hermes` | BioMistral-7B |
| Fast gateway-only demo | `llama3.2:1b` | Weak clinical depth — architecture story only |
| Clinical alternative | `meditron:7b` | Official Ollama medical model |
| Cloud + BAA | `LLM_PROVIDER=anthropic` | Not on-prem |

```bash
# Optional tiny fallback model (not recommended for clinical demos)
PULL_DEMO_MODEL=1 OLLAMA_MODEL=llama3.2:1b ./scripts/setup-biomistral.sh
```

## Troubleshooting

| Problem | Fix |
|---------|-----|
| `model 'biomistral-hermes' not found` | `./scripts/setup-biomistral.sh` |
| `empty response from model` | You are on a broken raw tag — use `biomistral-hermes` |
| Slow / OOM | `DEID_MODE=rules`; stop other Docker stacks; see `./scripts/cleanup-space.sh` |
| Request timeout | Wait 90s+ on first CPU run; BioMistral is loading |

## Sales positioning

- **BioMistral** = clinical credibility for hospital pilots (medical vocabulary).
- **Hermes gateway** = de-ID, policy, RBAC, audit — the compliance story.
- **Not a medical device** — drafting assist only; not a HIPAA certification.
