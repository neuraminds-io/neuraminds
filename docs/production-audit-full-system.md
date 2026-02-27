# Production Audit: full-system

**Audit Date**: 2026-02-26
**Scope**: EVM contracts, Rust backend, web frontend, SDK, deployment workflows, launch/readiness scripts
**Verdict**: SHIP WITH FIXES

## Executive Summary

The codebase is materially stronger than a demo stack and now has real launch-grade mechanics in contracts, backend writes, and frontend flow. Core protocol and app build/test gates pass. The highest residual risk is operational: production launch still blocks on real environment completeness and one upstream Rust advisory with no upstream patch. Both are now explicitly surfaced and mitigated in CI/workflows, but they are not “self-resolving.”

### Critical Findings Count

| Severity | Count | Status |
|----------|-------|--------|
| P0 | 1 | Partially mitigated |
| P1 | 3 | Fixed / mitigated |
| P2 | 3 | Open |
| P3 | 2 | Open |

## Critical Issues (P0 - Block Release)

- [ ] Production launch env is incomplete in runtime config validation.
  - Impact: `launch:config` and `launch:readiness:strict` fail in production mode until deployment env is fully populated (addresses, secrets, HTTPS origins).
  - Fix: populate production env in secret manager and deployment platform with all required Base variables (`MARKET_CORE_ADDRESS`, `ORDER_BOOK_ADDRESS`, `COLLATERAL_VAULT_ADDRESS`, `AGENT_RUNTIME_ADDRESS`, read/write toggles, frontend public vars, HTTPS-only origins).

## High Priority (P1 - Fix Before Launch)

- [x] Web dependency security audit had high vulnerabilities (Next + transitive tooling).
  - Impact: known framework/package vulnerabilities in shipped dependency graph.
  - Fix executed: upgraded web stack to patched line (`next@15.5.12`, `eslint-config-next@15.5.12`), re-ran audit and build.

- [x] Release/deploy workflows ran `cargo audit` without upstream advisory exceptions already accepted in CI.
  - Impact: tag release/deploy pipelines could fail despite known/no-fix advisory state.
  - Fix executed: aligned `release.yml` and `deploy-mainnet.yml` with explicit ignores for `RUSTSEC-2023-0071` and existing accepted advisories.

- [x] Unmaintained Rust dependencies in active backend graph (`dotenv`, `bincode`).
  - Impact: supply-chain and maintenance risk.
  - Fix executed: migrated `dotenv -> dotenvy`; removed unused `bincode` dependency.

## Medium Priority (P2 - Fix Soon After Launch)

- [ ] Upstream `RUSTSEC-2023-0071` (via `sqlx-mysql` transitive path) remains unresolved upstream.
  - Impact: accepted medium advisory remains in lockfile path.
  - Mitigation: explicit workflow ignore, no direct mysql runtime usage in this backend scope.
  - Follow-up: track sqlx/rsa upstream and remove ignore immediately when fixed.

- [ ] `launch:config` and `production:gates:strict` are both green-path tools but validate different readiness dimensions.
  - Impact: false confidence if only one is used.
  - Follow-up: enforce both gates in release checklist, and fail release on `launch:config` in controlled prod env.

- [ ] Backend strict clippy (`-D warnings`) still fails on non-correctness lint debt.
  - Impact: quality signal weaker than ideal; not currently a release blocker in CI (CI uses `-D clippy::correctness`).
  - Follow-up: burn down warning-class lint debt incrementally.

## Low Priority (P3 - Technical Debt)

- [ ] Add dedicated agent-runtime route/API smoke tests in CI.
  - Impact: regression detection for new write API and runtime paths relies on broader tests.

- [ ] Consolidate lockfile strategy for monorepo clarity.
  - Impact: tooling ambiguity risk in multi-package environments.

## Security Assessment

### Fixed
- Web dependency graph audited clean (`npm audit --omit=dev` in root and web).
- Removed unmaintained direct Rust deps (`dotenv`, `bincode`).
- Release/deploy workflows now explicitly codify accepted RustSec exceptions.
- Contract flow already hardened in prior sprint for:
  - unified vault collateral path,
  - permissionless matching,
  - richer onchain market metadata,
  - backend write-API based transaction prep.

### Residual
- `RUSTSEC-2023-0071` has no upstream fix; kept as explicit risk acceptance with CI/workflow parity.

## Performance Assessment

- Contract test suite and backend test/check pass quickly.
- Web production build succeeds after Next upgrade.
- No immediate hot-path regressions detected in this hardening pass.

## Observability Assessment

- Existing metrics/logging endpoints and production gate scripts remain operational.
- Needed: add explicit alerts/checks for write API success/error rates and agent runtime execution failures post-launch.

## Recommended Architecture Changes

1. Promote write API and agent runtime health metrics to first-class SLOs.
2. Add canary smoke suite that exercises: createMarketRich -> placeOrder -> matchOrders -> resolve -> claim.
3. Add rollout guard requiring both config readiness and production gates before deploy promotion.

## Test Coverage Gaps

- No dedicated CI test specifically targeting `/v1/evm/write/agents/*` end-to-end behavior.
- No production smoke that validates env completeness against real deployment secrets in CI/CD context.

## Action Plan

### Immediate (completed in this pass)
1. Upgrade vulnerable web dependencies and re-verify web lint/build.
2. Remove unmaintained Rust deps and re-verify backend tests/check.
3. Align release/deploy cargo-audit policy with accepted upstream advisory exceptions.
4. Extend launch config validator to include `AGENT_RUNTIME_ADDRESS` and frontend `NEXT_PUBLIC_AGENT_RUNTIME_ADDRESS`.
5. Add web env example coverage for agent runtime public address.

### Immediate (still required before mainnet cutover)
1. Populate all production env vars/secrets and HTTPS origins in deployment platform.
2. Re-run `launch:config` and `launch:readiness:strict` in production env context and require pass.

### Short-term (post-launch hardening)
1. Add write-API + agent-runtime route integration tests.
2. Add alerting dashboards for write API failure rate and runtime execution cadence gaps.
3. Burn down non-correctness clippy warnings to raise code-quality floor.

## Verification Checklist

- [x] `forge test --root evm`
- [x] `cargo check --manifest-path app/Cargo.toml`
- [x] `cargo test --manifest-path app/Cargo.toml`
- [x] `npm --prefix web run lint`
- [x] `npm --prefix web run build`
- [x] `npm --prefix sdk/agent run build`
- [x] `npm audit --omit=dev` (root)
- [x] `npm --prefix web audit --omit=dev`
- [x] `npm run production:gates:strict`
- [x] `cargo audit --ignore RUSTSEC-2024-0344 --ignore RUSTSEC-2022-0093 --ignore RUSTSEC-2023-0071`
- [ ] `npm run launch:config` with real production env/secrets
- [ ] `npm run launch:readiness:strict` with real production env/secrets

## Files Analyzed

- EVM: contracts, scripts, tests
- Backend: API, config, RPC/write paths, service wiring
- Web: route handlers, API client, build/lint config
- SDK: agent write-path integration
- CI/CD: `ci.yml`, `release.yml`, `deploy-mainnet.yml`
- Ops scripts: launch/config validation and production gates

