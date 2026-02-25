# NeuralMinds Co-Developer Onboarding Guide

Last updated: February 25, 2026
Audience: engineers joining as active contributors to the NeuralMinds codebase

## 1. Purpose

This document is the operational handoff for new co-developers.

It explains:
- What NeuralMinds is building right now
- How the repository is structured
- How to run the stack locally
- How to ship safely without slowing down the Base pivot
- What work is in scope next

If you read only one doc before coding, read this one.

## 2. Project State Snapshot

NeuralMinds is in an active Solana to Base migration.

Current reality:
- Legacy Solana stack still exists and remains partially wired
- Base contracts are now present in `evm/` and tested
- Backend and frontend have Base read paths and SIWE auth flow support
- Full Base write parity (create/cancel/claim/trade indexing) is not complete yet

Primary tracking docs:
- `docs/BASE_PIVOT_PLAN.md`
- `docs/BASE_MIGRATION_BOARD.md`

Use those as source-of-truth for migration status before starting any chain-related work.

## 3. Repository Map

Top-level areas you will work in most:

- `app/`
  - Rust backend API (Actix Web)
  - Auth, market/order/position APIs
  - Redis + Postgres integration
  - Chain integration surfaces (legacy Solana + Base pivot endpoints)

- `web/`
  - Next.js frontend
  - Wallet/auth UX (Solana + Base/SIWE paths)
  - Market/order/portfolio UI

- `evm/`
  - Foundry workspace for Base-native contracts
  - `NeuraToken`, `MarketCore`, `OrderBook`, `CollateralVault`
  - Unit tests + deployment script

- `programs/`
  - Legacy Solana Anchor programs (still present)

- `migrations/`
  - SQL migrations used by backend

- `scripts/`
  - Launch/readiness checks
  - Synthetic monitoring and production gate reporting

- `docs/`
  - Architecture, migration, readiness, and runbooks

## 4. Toolchain Requirements

Required:
- Rust stable toolchain
- Node.js + npm
- Docker + Docker Compose
- Foundry (`forge`, `cast`)

Optional (legacy Solana work only):
- Solana CLI
- Anchor CLI

## 5. Local Setup (First Run)

### 5.1 Clone and install

```bash
git clone <repo-url>
cd neuralminds
npm install
npm --prefix web install
```

### 5.2 Environment files

```bash
cp .env.example .env
cp app/.env.example app/.env
cp web/.env.example web/.env.local
```

Recommended local defaults for Base development in `.env`:

```env
EVM_ENABLED=true
SOLANA_ENABLED=false

BASE_RPC_URL=https://sepolia.base.org
BASE_WS_URL=wss://sepolia.base.org
BASE_CHAIN_ID=84532
SIWE_DOMAIN=localhost:3000

NEURA_TOKEN_ADDRESS=<set after deploy>
MARKET_CORE_ADDRESS=<set after deploy>
ORDER_BOOK_ADDRESS=<set after deploy>

DATABASE_URL=postgres://postgres:<password>@localhost:5432/polyguard
REDIS_URL=redis://localhost:6379
JWT_SECRET=<32+ char secret>
```

Recommended local `web/.env.local` (defaults from `web/.env.example`):

```env
NEXT_PUBLIC_API_URL=http://localhost:8080/v1
NEXT_PUBLIC_WS_URL=ws://localhost:8080/ws

NEXT_PUBLIC_CHAIN_MODE=base
NEXT_PUBLIC_BASE_RPC_URL=https://sepolia.base.org
NEXT_PUBLIC_BASE_CHAIN_ID=84532
NEXT_PUBLIC_SIWE_DOMAIN=localhost:3000

NEXT_PUBLIC_DATA_SOURCE=api
```

Notes:
- If you want UI-only work without backend dependency, set `NEXT_PUBLIC_DATA_SOURCE=mock`.
- If you switch between Base and Solana work, keep separate `.env` presets.

### 5.3 Start local dependencies

```bash
docker-compose up -d postgres redis
```

### 5.4 Run database migrations

```bash
cargo install sqlx-cli --no-default-features --features postgres
sqlx migrate run --source migrations
```

### 5.5 Start backend and frontend

Terminal A:

```bash
cargo run --manifest-path app/Cargo.toml --bin polyguard-api
```

Terminal B:

```bash
npm --prefix web run dev
```

Health checks:
- Backend: `http://localhost:8080/health`
- Frontend: `http://localhost:3000`

## 6. Daily Developer Workflow

1. Pick a task from `docs/BASE_MIGRATION_BOARD.md`.
2. Create a branch using `kamiyo/<short-scope>`.
3. Implement smallest safe vertical slice.
4. Run local quality gates (section 7).
5. Update docs/checklists when behavior changes.
6. Open PR with:
   - what changed
   - test evidence
   - risks and rollback note

Branching rules:
- Do not push directly to `main`.
- Keep PRs focused and reviewable.
- If refactoring, split mechanical changes from logic changes.

## 7. Quality Gates (Run Before PR)

From repo root:

```bash
# Backend
cargo check --manifest-path app/Cargo.toml
cargo test --manifest-path app/Cargo.toml

# Frontend
npm --prefix web run lint
npm --prefix web run build

# Contracts
npm run evm:build
npm run evm:test
```

Optional but recommended:

```bash
npm --prefix web run test:e2e
npm run launch:readiness
```

Important:
- If any command is intentionally skipped, document why in the PR.
- Do not merge code that only compiles on one side of the stack (web-only or app-only) when API contracts changed.

## 8. Architecture and Runtime Notes

### 8.1 Backend

Entrypoint: `app/src/main.rs`

Core runtime:
- Actix server with JSON APIs, WS, rate limiting, and security headers
- Services initialized for Postgres, Redis, orderbook state, metrics
- `/v1/evm/*` namespace now hosts Base read endpoints

Current Base endpoints:
- `GET /v1/evm/token/state`
- `GET /v1/evm/markets`
- `GET /v1/evm/markets/{market_id}/orderbook`

Auth:
- Legacy wallet login path still exists
- SIWE endpoints are active for Base flow:
  - `GET /v1/auth/siwe/nonce`
  - `POST /v1/auth/siwe/login`

### 8.2 Frontend

Stack:
- Next.js app router
- TanStack Query data hooks
- Solana wallet adapter (legacy)
- Wagmi + viem (Base path)

Routing behavior:
- `NEXT_PUBLIC_CHAIN_MODE=base` drives Base wallet/auth/read paths
- `NEXT_PUBLIC_CHAIN_MODE=solana` keeps legacy behavior

### 8.3 Contracts

Foundry workspace in `evm/`.

Current contract status:
- Base skeletons implemented and tested
- Not all production mechanics complete (matching and settlement hardening still pending)

Deploy commands (wrapper scripts already in root `package.json`):

```bash
npm run evm:deploy:base-sepolia
npm run evm:deploy:base
```

## 9. Environment and Secrets Policy

Rules:
- Never commit `.env` or private keys.
- Keep deployer keys in Foundry keystore, not shell history.
- Use generated secrets for JWT and webhook verification.
- Production origins must be explicit (no wildcard CORS).

Before production changes:
- Run `npm run launch:config` and `npm run launch:readiness`.
- Review generated reports under `docs/reports/`.

## 10. Deployment Model

Current deploy artifacts in repo include:
- `render.yaml` (managed deployment path)
- Dockerfiles for backend image builds
- CI workflows under `.github/workflows/`

Practical guidance for contributors:
- Assume managed deployment and avoid host-specific assumptions.
- Keep runtime behavior configurable via environment variables.
- Any infra-impacting change must include docs updates and rollback notes.

## 11. Current High-Priority Work Areas

Take tasks from the migration board in this order:

1. Base trade history/indexing path
2. Base write flows (create/cancel/claim)
3. Settlement and payout correctness hardening
4. CI parity for Base paths (including contract test job coverage)
5. Removal of dead Solana-only paths after parity window

When choosing a task, bias toward:
- parity-critical work
- correctness and observability
- minimizing migration risk

## 12. Definition of Done for Migration Tasks

A task is not done until all are true:
- Code merged for backend + frontend contract boundaries
- Tests added or updated for changed behavior
- Local quality gates passing
- Migration board status updated
- Rollback path stated in PR
- Any new env var documented in `.env.example` (and `app/.env.example` when applicable)

## 13. Common Pitfalls

- Implementing a frontend chain-mode branch without backend endpoint parity
- Returning placeholder payloads without explicit TODO tracking
- Forgetting to update migration docs after shipping behavior
- Leaving `EVM_ENABLED` off while testing Base UI and misdiagnosing auth/read failures
- Relying on public RPC endpoints for high-volume tests (rate limit noise)

## 14. Incident and Ops References

Runbooks:
- `docs/runbooks/INCIDENT_RESPONSE.md`
- `docs/runbooks/DISASTER_RECOVERY.md`

Launch and monitoring references:
- `docs/LAUNCH_COMMAND_CENTER.md`
- `docs/SYNTHETIC_MONITORING.md`
- `docs/ALERT_ROUTING_MATRIX.md`

## 15. First Week Checklist for New Co-Developers

Day 1:
- Complete local setup
- Run full quality gate command set
- Read Base pivot plan + migration board
- Shadow one PR from open to merged

Day 2-3:
- Ship one low-risk Base migration task
- Add or improve one test path
- Update docs for any new env or endpoint behavior

Day 4-5:
- Ship one medium-complexity task touching at least two layers (app + web, or app + evm)
- Participate in review on another migration PR
- Identify one risk and propose mitigation in docs or code

## 16. Documentation Maintenance Rule

If behavior changes and docs are not updated in the same PR, the change is incomplete.

Minimum docs to touch when relevant:
- `docs/BASE_MIGRATION_BOARD.md`
- `docs/BASE_PIVOT_PLAN.md`
- `docs/openapi.yaml` (for API contract changes)
- `.env.example` and `app/.env.example` (for config surface changes)

---

For any uncertainty, default to correctness and traceability over speed.
