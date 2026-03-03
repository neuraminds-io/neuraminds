# neuraminds

[![CI](https://github.com/neuraminds-io/neuraminds/actions/workflows/ci.yml/badge.svg)](https://github.com/neuraminds-io/neuraminds/actions/workflows/ci.yml)
[![Release](https://github.com/neuraminds-io/neuraminds/actions/workflows/release.yml/badge.svg)](https://github.com/neuraminds-io/neuraminds/actions/workflows/release.yml)
[![Deploy](https://github.com/neuraminds-io/neuraminds/actions/workflows/deploy-mainnet.yml/badge.svg)](https://github.com/neuraminds-io/neuraminds/actions/workflows/deploy-mainnet.yml)
[![Synthetic Monitoring](https://github.com/neuraminds-io/neuraminds/actions/workflows/synthetic-monitoring.yml/badge.svg)](https://github.com/neuraminds-io/neuraminds/actions/workflows/synthetic-monitoring.yml)
[![CodeQL](https://github.com/neuraminds-io/neuraminds/actions/workflows/codeql.yml/badge.svg)](https://github.com/neuraminds-io/neuraminds/actions/workflows/codeql.yml)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Security Policy](https://img.shields.io/badge/security-policy-brightgreen.svg)](SECURITY.md)

Neuraminds is an agentic prediction infrastructure stack with Base-native contracts, Solana programs, a Rust API, and a Next.js web client.

## Why Fork This Repo

- End-to-end reference implementation: contracts, API, web, and SDK in one repo.
- Production-oriented defaults: CI gates, release workflow, rollback workflow, and synthetic monitoring.
- Clear extension points for alternate market feeds, execution engines, and agent runtimes.
- Open-core architecture with strict private-edge boundaries.

## Repository Layout

- `app/`: Rust API server and chain/external adapters.
- `evm/`: Foundry workspace for Base contracts.
- `programs/`: Solana program workspace.
- `web/`: Next.js frontend.
- `sdk/`: client and agent SDK surfaces.
- `migrations/`: PostgreSQL schema migrations.
- `config/`: runtime manifests, including open-core boundary config.
- `edge/`: public placeholders only; private edge runtime is out-of-repo.

## Quick Start

```bash
cp .env.example .env
docker compose up -d postgres redis
npm ci
npm ci --prefix web
cargo run --manifest-path app/Cargo.toml
```

In a second terminal:

```bash
npm --prefix web run dev
```

## Quality Gates

Run these before opening a PR:

```bash
npm run ops:silo-check:strict
npm run ops:open-core-check
npm run ops:no-internal-assets:tracked
cargo test --manifest-path app/Cargo.toml --release
forge test --root evm
```

## CI/CD

Workflow coverage includes:

- CI: formatting, boundary checks, backend tests, EVM tests, and image build.
- Release: tag-based test/build/release pipeline.
- Deploy: manual staged/production deployment workflow.
- Rollback: controlled rollback workflow with health verification.
- Monitoring: scheduled synthetic probes.
- Security: CodeQL and dependency review.

Details are documented in `docs/CI_CD.md`.

## Open Core / Closed Edge

This repo is the open core. Private operator edge runtime is kept in a separate private repository/workspace.

- Policy: `docs/OPEN_CORE_CLOSED_EDGE.md`
- Boundary manifest: `config/open-core-closed-edge.json`

## Security

Please report vulnerabilities privately via GitHub Security Advisories or the contact path in `SECURITY.md`.

## Contributing

Read `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `GOVERNANCE.md`, and `SUPPORT.md` before submitting changes.
