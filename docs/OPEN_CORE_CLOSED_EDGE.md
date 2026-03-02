# Neuraminds Open Core / Closed Edge

## Boundary

Open core is public protocol and product infrastructure.
Closed edge is proprietary execution and operator logic.

## Open Core Scope

- `app/` core API and adapters
- `evm/` contracts and tests
- `programs/` onchain programs
- `sdk/` client packages
- `web/` user-facing product surfaces
- `migrations/`, `tests/`, and public config/docs

## Closed Edge Scope

- `edge/` proprietary modules
- `services/xmtp-bridge/` private operator bridge logic
- `scripts/dx-terminal-pro.sh` operator execution script
- `docs/runbooks/DX_TERMINAL_OPERATIONS.md` private ops runbook

## Enforcement

- Boundary manifest: `config/open-core-closed-edge.json`
- Verifier: `scripts/verify-open-core-boundary.mjs`
- Required checks:
  - `npm run ops:silo-check:strict`
  - `npm run ops:open-core-check`

## Import Policy

- Allowed: closed edge -> open core
- Forbidden: open core -> closed edge

Any cross-reference from open-core source to closed-edge paths fails CI.

## Copy/Paste Intake Policy

When importing code from private repos:
1. Copy code directly into this repo.
2. Rename project-specific identifiers for public use.
3. Remove private hostnames, endpoints, and credentials.
4. Run `npm run ops:silo-check:strict` and `npm run ops:open-core-check` before push.
