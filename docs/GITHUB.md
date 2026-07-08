# GitHub repo checklist

Use this when polishing the public repo for discoverability.

## Repository settings (github.com → Settings)

**About** (top-right of repo home):

| Field | Suggested value |
|-------|-----------------|
| Description | Open-source HIPAA-aligned AI gateway — de-ID, policy, audit, on-prem BioMistral (Rust) |
| Website | `https://github.com/MarketMadi/hipaa-hermes-agent#readme` |
| Topics | `hipaa` `healthcare` `biomistral` `llm` `rust` `de-identification` `audit-log` `compliance` `inference-gateway` `presidio` `local-ai` `medical-ai` |

**Social preview:** Upload a screenshot of http://localhost:8090/demo/ (Settings → General → Social preview).

## What makes people star

1. **README hook in 5 seconds** — problem + local-first solution (done in root README)
2. **Works locally in &lt;10 min** — `./scripts/setup-biomistral.sh && ./scripts/run.sh`
3. **Green CI badge** — `.github/workflows/ci.yml`
4. **Honest scope** — not claiming HIPAA certification builds trust
5. **Visual demo** — clinician UI + architecture diagram in README
6. **Clear roadmap** — `docs/DEPLOYMENT_EPICS.md` shows momentum

## Suggested first posts

- r/LocalLLaMA — on-prem clinical gateway + BioMistral
- r/rust — Rust rewrite, audit log, policy layer
- Hacker News — "Show HN: open-source gateway for regulated healthcare AI"
- LinkedIn — hospital AI governance angle (no PHI in post)

## Before announcing

- [ ] `cargo test` passes
- [ ] README quick start verified on a clean machine
- [ ] No secrets in git history (`.env` gitignored)
- [ ] LICENSE file present
- [ ] Demo uses synthetic data only
