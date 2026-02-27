# Security

This document describes the active security testing strategy for Neuraminds on Base.

## Overview

The codebase has two production surfaces:

- EVM contracts in `evm/`
- Backend API in `app/`

Security posture is enforced through role-based access controls, deterministic settlement logic, extensive automated tests, and release gates.

## Contract Security (Base/EVM)

### Test stack

- Unit/integration tests: `forge test --root evm`
- Stress fuzz campaign: `./scripts/fuzz-campaign.sh`

Quick run:

```bash
./scripts/fuzz-campaign.sh
```

Deep run before launch:

```bash
./scripts/fuzz-campaign.sh --runs 5000 --iterations 5
```

Targeted campaign:

```bash
./scripts/fuzz-campaign.sh --match-contract OrderBookTest --runs 10000
```

### Core invariants validated

- Locked collateral cannot exceed deposited collateral.
- Settlement cannot execute before market resolution.
- Only designated operator/matcher roles can execute privileged actions.
- Pause controls block mutating methods.
- Fee and payout paths conserve balances under edge-case order flow.

## API Security

The backend enforces:

- SIWE + JWT auth flow
- Rate limits on mutating endpoints
- Request validation and strict wallet/tx hash formats
- Health and metrics endpoints for live operational checks

Run backend tests:

```bash
cargo test --manifest-path app/Cargo.toml
```

## Full Security Stress Check

Run end-to-end security checks (static + runtime + dependency scan):

```bash
./scripts/security-stress-test.sh
```

This includes:

- hardcoded secret scans
- dangerous Solidity primitive scans
- frontend XSS/eval checks
- `cargo audit` (if installed)
- `npm audit` for web dependencies
- `forge test` and backend test execution

## Launch Gate

Before production deployment, the minimum gate is:

```bash
npm run launch:readiness:strict
```

This validates Base-only env config, production gates, and synthetic health checks.

## Disclosure

For responsible disclosure, open a private security report through project maintainers.
