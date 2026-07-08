# OIDC / SSO (Epic 3)

Hermes validates **Bearer JWTs** from your IdP (Keycloak, Auth0, Okta, hospital Azure AD, etc.) and maps IdP groups or realm roles to **operator** and **auditor**.

When `OIDC_ENABLED=0` (default), auth uses **`X-Role-Key`** only — unchanged from v3.

---

## How it works

1. Client sends `Authorization: Bearer <access_token>`.
2. Hermes fetches JWKS (cached 5 minutes), validates RS256 signature, issuer, audience, expiry.
3. Groups come from JWT `groups` and/or Keycloak `realm_access.roles`.
4. First match wins: operator groups → `operator`, else auditor groups → `auditor`.
5. Audit log **actor** is JWT `email`, `preferred_username`, or `sub`.

If OIDC is enabled and `OIDC_ALLOW_ROLE_KEY=1` (default in **local** / **dev**), `X-Role-Key` still works as break-glass. In **prod**, startup fails if both OIDC and `OIDC_ALLOW_ROLE_KEY=1`.

---

## Environment variables

| Variable | Default | Description |
|----------|---------|-------------|
| `OIDC_ENABLED` | `0` | Set `1` to require JWT (with optional role-key fallback) |
| `OIDC_ISSUER` | — | Issuer URL, e.g. `http://127.0.0.1:8180/realms/hermes` |
| `OIDC_AUDIENCE` | `hermes-api` | Expected `aud` claim |
| `OIDC_JWKS_URL` | `{issuer}/protocol/openid-connect/certs` | Override JWKS endpoint |
| `OIDC_OPERATOR_GROUPS` | `hermes-operator` | Comma-separated group/role names |
| `OIDC_AUDITOR_GROUPS` | `hermes-auditor` | Comma-separated group/role names |
| `OIDC_ALLOW_ROLE_KEY` | `1` local/dev, `0` prod | Break-glass shared keys when OIDC on |

---

## Local Keycloak (Compose)

```bash
./scripts/setup-keycloak.sh
```

Add to `.env`:

```bash
OIDC_ENABLED=1
OIDC_ISSUER=http://127.0.0.1:8180/realms/hermes
OIDC_AUDIENCE=hermes-api
OIDC_ALLOW_ROLE_KEY=1
```

Restart Hermes (`./scripts/run.sh`), then:

```bash
TOKEN=$(./scripts/get-oidc-token.sh operator operator)
curl -s http://localhost:8090/v1/inference \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"prompt":"de-identified note","skill":"vault-answer"}' | jq .
```

Test users: `operator`/`operator`, `auditor`/`auditor`. Admin UI: `http://127.0.0.1:8180/admin/` (admin/admin).

---

## Demo UI

Open `http://localhost:8090/demo/`. If a token is stored in `localStorage.hermes_bearer`, requests use Bearer auth instead of `X-Role-Key`:

```javascript
localStorage.setItem('hermes_bearer', '<paste token>');
location.reload();
```

Clear: `localStorage.removeItem('hermes_bearer')`.

---

## Production IdP

Point `OIDC_ISSUER` / `OIDC_JWKS_URL` at your tenant. Map hospital AD groups to `OIDC_OPERATOR_GROUPS` / `OIDC_AUDITOR_GROUPS`. Set `HERMES_ENV=prod`, `OIDC_ENABLED=1`, `OIDC_ALLOW_ROLE_KEY=0`.

Use authorization-code flow in your EHR or portal; do **not** use password grant in production.

---

## curl examples

**Operator inference (JWT):**

```bash
curl -H "Authorization: Bearer $TOKEN" ...
```

**Auditor export:**

```bash
AUDITOR_TOKEN=$(./scripts/get-oidc-token.sh auditor auditor)
curl -H "Authorization: Bearer $AUDITOR_TOKEN" http://localhost:8090/v1/audit/export
```

**Break-glass (local/dev only when allowed):**

```bash
curl -H "X-Role-Key: $ADMIN_SECRET" ...
```
