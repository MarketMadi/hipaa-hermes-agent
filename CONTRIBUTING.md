# Contributing

Thanks for your interest in HIPAA Hermes. This project aims to be the clearest open-source reference for **regulated AI inference gateways** — help us get there.

## Good first issues

- De-ID rule coverage and fixture tests (`crates/hermes/tests/deid_safe_harbor.rs`)
- Documentation clarity (README, architecture diagrams)
- Observability dashboards (`deploy/grafana/`)
- BioMistral setup and notes (`docs/MODELS.md`)

## Development setup

```bash
git clone https://github.com/MarketMadi/hipaa-hermes-agent.git
cd hipaa-hermes-agent
cp .env.example .env
./scripts/setup-biomistral.sh   # optional — needed for live LLM tests
cargo test
./scripts/run.sh
```

## Pull requests

1. Fork and branch from `main`
2. Run `cargo test` and `cargo fmt`
3. Keep PRs focused — one concern per PR
4. Update docs if you change behavior or env vars
5. Do not commit `.env`, databases, or API keys

## Code style

- Match existing Rust patterns in `crates/hermes`
- Prefer small, testable modules over large files
- Comments only for non-obvious business logic (Safe Harbor rules, policy edge cases)

## Security

See [SECURITY.md](SECURITY.md) for reporting vulnerabilities.

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
