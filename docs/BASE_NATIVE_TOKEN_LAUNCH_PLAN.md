# NeuralMinds Native Token Launch Plan (Base)

Last updated: February 26, 2026
Owner: Protocol + Backend + Frontend + Ops
Status: Planning (execution-ready)

## 1. Goal

Launch a production-grade native `NEURA` token on Base with:
- deterministic deployment and verification,
- secure role/permission management,
- controlled liquidity bootstrap,
- operational controls for incident response,
- clear go/no-go launch gates.

This plan assumes `NEURA` is the native protocol token on Base, and that market settlement collateral is a stable asset (USDC), not `NEURA`.

## 2. Research Findings (Primary Sources)

1. Base recommends either launch platforms (fast) or custom ERC-20 + Foundry + OpenZeppelin (full control).
Source: https://docs.base.org/get-started/launch-token

2. Base network config for production must use Base mainnet chain id (`8453`) and robust RPC infra; Base docs explicitly note public nodes can fail under high load and are not intended for production systems.
Source: https://docs.base.org/base-chain/network-information/chain-settings-and-endpoints

3. Contract verification is first-class for trust and transparency on Base.
Source: https://docs.base.org/base-chain/tools/block-explorers/basescan/verify-smart-contract

4. Token visibility in wallets/discovery is a post-launch task (token profiles/listings).
Source: https://docs.base.org/base-chain/quickstart/tokens-in-wallet

5. Official Base ecosystem docs expose canonical DEX contract references (Uniswap V3/V4, etc.), useful for safe integration and allowlist checks.
Source: https://docs.base.org/base-chain/quickstart/contracts

6. Official Circle documentation lists native USDC on Base mainnet as `0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913`.
Source: https://developers.circle.com/stablecoins/usdc-contract-addresses

7. ERC-20 and ERC-2612 (permit) remain the baseline token standards for compatibility and gas-efficient approvals.
Sources:
- https://eips.ethereum.org/EIPS/eip-20
- https://eips.ethereum.org/EIPS/eip-2612

8. OpenZeppelin recommends robust RBAC and delayed admin operations for production systems (AccessControl/AccessManager/TimelockController).
Sources:
- https://docs.openzeppelin.com/contracts/5.x/access-control
- https://docs.openzeppelin.com/contracts/5.x/api/governance

## 3. Hard Recommendation

1. Do not use `NEURA` as primary market collateral.
Reason: volatile collateral destabilizes pricing, risk controls, and UX.

2. Use native USDC on Base for collateral, fees, and settlement.
Reason: stable quote currency, deeper liquidity, safer risk envelope.

3. Keep `NEURA` as governance/utility/incentive token (staking, fee rebates, reward emissions, governance rights), not core settlement money.

## 4. Current State (Repo)

`evm/src/NeuraToken.sol` already includes:
- ERC-20,
- cap,
- pause,
- permit (EIP-2612),
- RBAC roles (`DEFAULT_ADMIN_ROLE`, `MINTER_ROLE`, `PAUSER_ROLE`).

Gaps to close before mainnet token launch:
- role handoff automation to multisig/timelock,
- deployer privilege revocation runbook + script,
- explicit fixed-supply vs emissions policy lock,
- expanded adversarial/fuzz test coverage,
- post-deploy monitoring and launch circuit-breaker playbook.

## 5. Launch Architecture

## 5.1 Control Plane

- `DEFAULT_ADMIN_ROLE`: timelock (or at minimum multisig).
- `MINTER_ROLE`: timelock-controlled emissions module, or fully revoked after genesis mint.
- `PAUSER_ROLE`: security multisig with narrow incident authority.

Minimum signer model:
- treasury/ops multisig: 3/5 or 4/7,
- independent security signer set,
- no long-lived EOA admin power.

## 5.2 Supply Policy

Pick one and lock it before testnet rehearsal:

Option A: Fixed supply
- `cap == initialSupply`
- revoke `MINTER_ROLE` at TGE
- simplest trust model

Option B: Programmatic emissions
- hard cap remains immutable
- emissions executed only via timelock
- publish schedule and max mint per epoch

## 5.3 Liquidity Strategy

Primary pool target:
- `NEURA/USDC` on a Base-native DEX (Uniswap V3 and/or Aerodrome after contract/address validation)

Liquidity controls:
- start with conservative treasury-provided depth,
- avoid extreme initial FDV assumptions,
- predefine intervention policy (what triggers rebalancing, who can execute it).

## 6. Execution Plan

## Phase 0: Parameter Freeze (must complete first)

Decisions to lock:
- token name/symbol/decimals (and metadata URI policy),
- total cap and initial circulating supply,
- fixed supply vs emissions,
- treasury allocation splits and vesting timetable,
- initial LP budget and launch range policy,
- multisig addresses and signer roster.

Deliverables:
- signed token parameter spec (`docs/TOKEN_SPEC_BASE.md`),
- signer policy doc,
- public tokenomics summary.

Go/No-Go gate:
- no unresolved parameter ambiguity.

## Phase 1: Contract Hardening

Tasks:
1. Add deployment scripts for deterministic token-only and full core deploy paths.
2. Add role-handoff script:
- grant target multisig/timelock roles,
- revoke deployer roles,
- assert final role graph onchain.
3. Add invariants/tests:
- cap invariant,
- pause behavior,
- permit domain separator and replay safety checks,
- unauthorized mint/pause/admin action coverage.
4. Add static analysis in CI (`slither`, `forge test -vvv`, fuzz targets).

Deliverables:
- passing CI with security checks,
- deploy + role handoff scripts,
- role graph verification script outputs.

Go/No-Go gate:
- 0 critical/high findings unresolved.

## Phase 2: Base Sepolia Rehearsal

Tasks:
1. Deploy token and protocol contracts to Base Sepolia.
2. Verify contracts on BaseScan.
3. Execute full role handoff and privilege revocation.
4. Rehearse launch sequence exactly as mainnet:
- deploy,
- verify,
- configure backend/frontend env,
- create LP,
- run smoke checks.

Deliverables:
- rehearsal report with tx hashes,
- role snapshots,
- failed/edge case log and fixes.

Go/No-Go gate:
- two consecutive clean rehearsals.

## Phase 3: Mainnet Readiness

Tasks:
1. Freeze release SHA and env manifests.
2. Validate production RPC and failover RPC.
3. Pre-sign emergency pause and key rotation runbooks.
4. Confirm legal/compliance sign-off and market communication timeline.

Deliverables:
- launch checklist signed by protocol, security, ops,
- incident commander assigned,
- rollback command set tested.

Go/No-Go gate:
- all checklists green and role checks reproducible.

## Phase 4: Base Mainnet Launch

Sequence:
1. Deploy token contract(s).
2. Verify on BaseScan.
3. Execute role handoff and revoke deployer powers.
4. Publish contract addresses and role owners.
5. Seed `NEURA/USDC` liquidity using predefined budget/range.
6. Enable app integration and production traffic.
7. Run 60-minute war room with live metrics.

Launch acceptance criteria:
- contracts verified,
- final role graph matches spec,
- LP live and priced within expected band,
- auth/trading/swap paths healthy,
- no P0/P1 incidents in first hour.

## Phase 5: Post-Launch (T+30 days)

Tasks:
- wallet/discovery profile completion,
- monitoring threshold tuning,
- treasury and LP policy retrospective,
- publish transparency report (supply, roles, treasury movements).

## 7. Risk Register

1. Admin key compromise
- Mitigation: multisig + timelock + role separation + emergency pause

2. Liquidity shock at launch
- Mitigation: conservative initial depth and predefined intervention limits

3. Contract config mismatch
- Mitigation: automated post-deploy assertions and mandatory rehearsal parity

4. Centralization criticism (if mint retained)
- Mitigation: transparent emissions policy and timelock-protected mint controls

5. RPC/provider outage
- Mitigation: primary + failover RPCs and runtime health probes

## 8. Immediate Build Queue (recommended next execution)

1. Create `evm/script/DeployToken.s.sol` and `evm/script/HandoffRoles.s.sol`.
2. Add `forge` test suite for role revocation and supply invariants.
3. Add CI job for `slither` + fuzz test profile.
4. Add `docs/TOKEN_SPEC_BASE.md` template with locked parameters.
5. Add `scripts/launch/verify_roles.sh` to assert role graph from chain state.

## 9. Open Decisions (blocking)

1. Fixed supply vs controlled emissions.
2. Initial circulating supply at TGE.
3. Treasury + liquidity budget and vesting schedule.
4. Multisig signer set and threshold.
5. Mainnet launch date and freeze window.
