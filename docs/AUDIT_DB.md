# Audit database (Epic 4)

Hermes stores append-only audit entries in **SQLite** (local) or **Postgres** (dev/prod). Managed Postgres provides encryption at rest; the app never stores PHI in audit metadata.

---

## Backends

| Environment | Backend | Config |
|-------------|---------|--------|
| **local** | SQLite (default) | `DATABASE_PATH=data/hipaa_hermes.db` |
| **dev / prod** | Postgres (required) | `DATABASE_URL` + `AUDIT_BACKEND=postgres` |

`AUDIT_BACKEND` auto-selects Postgres when `DATABASE_URL` is set.

---

## Environment variables

| Variable | Default | Description |
|----------|---------|-------------|
| `AUDIT_BACKEND` | auto | `sqlite` or `postgres` |
| `DATABASE_PATH` | `data/hipaa_hermes.db` | SQLite file (local) |
| `DATABASE_URL` | — | Postgres connection string (dev/prod) |

Example Postgres URL:

Local Docker Postgres listens on **:5433** when host :5432 is already taken.

```bash
DATABASE_URL=postgres://hermes:hermes@127.0.0.1:5433/hermes_audit
```

---

## Local Postgres (optional)

```bash
./scripts/setup-postgres.sh
```

Add to `.env`:

```bash
AUDIT_BACKEND=postgres
DATABASE_URL=postgres://hermes:hermes@127.0.0.1:5432/hermes_audit
```

Restart `./scripts/run.sh`. `/health` reports `"audit_backend": "postgres"`.

---

## Migrate SQLite → Postgres

One-way copy preserving entry IDs and hashes:

```bash
./scripts/setup-postgres.sh
./scripts/migrate-audit-to-postgres.sh data/hipaa_hermes.db
```

Or manually:

```bash
cargo run --release -p hermes --bin migrate-audit -- data/hipaa_hermes.db "$DATABASE_URL"
```

Duplicate `entry_hash` values are skipped (`ON CONFLICT DO NOTHING`).

---

## Schema

Both backends use the same logical schema:

- `id` — monotonic row id (returned as `audit_id` in API responses)
- `ts` — RFC3339 timestamp (part of hash input)
- `actor`, `role`, `action`, `resource`, `outcome`
- `metadata_json` — JSON blob (no raw PHI)
- `entry_hash` — SHA-256 of canonical entry fields (unique)

---

## Dev / prod validation

`HERMES_ENV=dev` or `prod` fails startup unless:

- `AUDIT_BACKEND=postgres`
- `DATABASE_URL` is set

Local demos keep SQLite for zero-setup speed.

---

## Backups (operational)

- **Postgres:** use managed DB snapshots / `pg_dump` with encrypted storage
- **SQLite:** copy `DATABASE_PATH` file while Hermes is stopped

See [DEPLOYMENT_EPICS.md](DEPLOYMENT_EPICS.md) for retention and DR targets.
