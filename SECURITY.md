# Security policy

## Supported versions

| Version | Supported |
|---------|-----------|
| `main` branch | Yes |
| Older tags | Best effort |

## Reporting a vulnerability

**Please do not open public GitHub issues for security problems.**

Email the maintainers with:

- Description of the issue
- Steps to reproduce
- Impact assessment (especially if PHI exposure is possible)

We will acknowledge within 72 hours and work on a fix before public disclosure when appropriate.

## Scope notes

This repository is a **reference / demo gateway**, not a certified HIPAA product. Production deployments require additional controls (TLS, SSO, encrypted storage, formal risk assessment). See [docs/DEPLOYMENT_EPICS.md](docs/DEPLOYMENT_EPICS.md).
