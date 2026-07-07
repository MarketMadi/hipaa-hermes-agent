# Q12 — Publication gate (answer before exporting the diagram)

**Question:** Can the HIPAA Hermes architecture diagram be published **anonymized** from the employment story, or does confidentiality require a **reference-architecture** version only?

**This gates Day 2.** Do not push a client-specific diagram to a public repo until this is answered.

---

## What the employment story contains (sensitive)

From Dave's recorded narrative (Loom / Upwork history), the real deployment story includes:

- Regulated healthcare platform replatform (legacy monolith → containerized AWS)
- HIPAA data-protection controls (encryption, least privilege, malware scan, audit)
- Concrete ops metrics (restore under 5 min, deploy 60→10 min, query reduction ~80%)
- A client relationship where some operational detail is true but **not for public attribution**

Even with the client name removed, **stack + metrics + timeline** can identify the engagement to insiders.

---

## Option A — Anonymized employment diagram

**Publish:** One diagram labeled "Healthcare platform — anonymized deployment (2020–2024)"

| Keep | Strip / generalize |
|------|-------------------|
| Architecture pattern (gateway, agent, encrypted store, audit) | Company name, product name |
| Generic "legacy PHP monolith" | Exact LOC count (8M) |
| "Sub-5-minute restore" as a range | Exact before/after deploy minutes if unique |
| HIPAA control categories | Vendor contract details |

**Risk:** Medium. A former colleague or client may still recognize the story from the metric combo.

**Use when:** Client has given written OK, or engagement is fully closed with no NDA conflict.

---

## Option B — Reference architecture only (recommended default)

**Publish:** `docs/ARCHITECTURE.md` as **"HIPAA-aligned Hermes — reference architecture"**

- No employment metrics, no war stories, no timeline tied to a real client
- Patterns only: trust zones, BAA boundaries, PHI guard, append-only audit
- Footnote: *"Informed by regulated healthcare deployments; not a depiction of any single client system."*

**Risk:** Low. Portfolio proves judgment without leaking confidential detail.

**Use when:** No explicit client sign-off (default for v1).

---

## Decision (fill in with Vino)

| Field | Value |
|-------|-------|
| **Chosen option** | ☐ A — Anonymized employment  ☑ **B — Reference only** |
| **Public repo gets** | `docs/ARCHITECTURE.md` + `docs/diagrams/*.svg` (reference version) |
| **Signed off by** | Dave — 2026-07-07 |

---

## Rule after decision

- **If B (reference):** Employment story stays in Loom/Upwork narrative only; GitHub diagram is generic.
- **If A (anonymized):** Run the [anonymization checklist](#anonymization-checklist) before any commit to `main`.

### Anonymization checklist (Option A only)

- [ ] No client, product, or investor names
- [ ] No unique metric triplets (restore + deploy + query % together)
- [ ] No screenshots from production systems
- [ ] No internal hostnames, IPs, or repo URLs
- [ ] Vino review on the exported PNG/SVG
