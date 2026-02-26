# Base Pivot Plan

## Objective
Pivot NeuralMinds from Solana-first architecture to Base-first architecture:
- Native protocol token deployed on Base
- Core protocol contracts deployed on Base
- Frontend/backend stack migrated from Solana wallet/program flows to EVM wallet/contract flows

## Research Summary (Primary Sources)
1. Base network configuration:
- Base Mainnet chain ID: `8453`
- Base Sepolia chain ID: `84532`
- Public RPC endpoints (`https://mainnet.base.org`, `https://sepolia.base.org`) are rate-limited and not recommended for production.

2. Base deployment workflow:
- Foundry is a first-class deployment path on Base.
- Recommended secure key flow uses Foundry keystore (`cast wallet import ...`) instead of raw private keys in shell history.

3. Token implementation guidance:
- Base docs recommend custom ERC-20 development with Foundry + OpenZeppelin contracts for teams needing full control.
- OpenZeppelin ERC-20 extensions cover `ERC20Capped`, `ERC20Burnable`, `ERC20Pausable`, etc., which align with production token controls.

4. Auth migration guidance:
- Base Account docs expose SIWE (`signInWithEthereum`, EIP-4361).
- Nonce freshness and backend signature verification are required to prevent replay.

5. Cross-chain transition path:
- Base-Solana bridge supports token/message bridging if we need staged migration/liquidity continuity.
- For a full pivot, bridge can be optional; direct Base-native launch avoids bridge operational complexity.

## Current-State Impact Inventory
The current repo is deeply Solana-coupled in four layers:
1. Onchain: Anchor programs under `programs/*`
2. Backend: Solana SDK/client services, Solana auth validation, Solana env vars
3. Frontend: Solana wallet adapter + Solana RPC readers/hooks
4. DevOps/docs/scripts: Solana deploy/readiness commands and assumptions

## Target Architecture
1. Onchain (Base)
- `NeuraToken` (ERC-20): capped supply, role-based mint controls, pause switch
- `MarketCore` (EVM): market lifecycle, resolution, claims
- `OrderBook` (EVM): orders, fills, cancellation, settlement hooks
- `Treasury/CollateralVault` (EVM): collateral custody + payout routing

2. Backend
- Replace Solana RPC services with EVM RPC client layer
- Replace Ed25519 wallet auth with SIWE verification
- Replace Solana tx/account parsers with EVM event/log indexers
- Keep DB schema, add chain-agnostic IDs and EVM address formats

3. Frontend
- Replace Solana wallet stack with wagmi/viem connectors (Base + Base Sepolia)
- Move from account polling to contract reads + indexed event queries
- Update all user-visible network/token text from Solana/USDC assumptions to Base-native configuration

## Migration Strategy
### Phase 0 - Foundation (Now)
- Introduce EVM contracts workspace (Foundry)
- Add Base chain env/config scaffolding
- Keep Solana path untouched so deployment risk stays controlled

### Phase 1 - Token + Core Contract Skeleton
- Implement `NeuraToken` contract
- Implement first `MarketCore` interfaces and storage layout
- Add deploy scripts for Base Sepolia and Base Mainnet
- Add tests for token controls and market lifecycle invariants

### Phase 2 - Backend Dual-Stack
- Add EVM RPC + log indexing services alongside Solana services
- Add SIWE endpoints and verification
- Add feature flags to route selected endpoints to Base contracts

### Phase 3 - Frontend Dual-Stack
- Add Base wallet provider and chain switch UX
- Add Base reads/writes behind feature flags
- Gradually replace Solana hooks per surface area (markets, portfolio, auth)

### Phase 4 - Cutover
- Freeze Solana write paths
- Migrate canonical contract addresses and env
- Promote Base path to default
- Remove Solana-only code after parity validation window

## Cutover Gates
1. Functional parity
- Market creation, order placement/cancel, resolution, claims

2. Security parity
- SIWE replay protection, role access controls, pause paths, invariant tests

3. Operational parity
- Deploy, monitor, rollback runbooks for Base
- End-to-end health checks against Base RPC and contracts

4. Performance
- Indexing lag and API response latency within existing SLOs

## Execution Backlog (Initial)
1. Create Foundry workspace and baseline contract suite
2. Add deploy scripts and Base env templates
3. Implement token contract + tests
4. Define `MarketCore` interface + storage layout and invariants
5. Add migration board in docs for Solana -> Base replacement tasks

Companion tracker: `docs/BASE_MIGRATION_BOARD.md`

## Risks and Decisions
1. Privacy feature parity
- Solana-specific privacy paths need redesign for EVM.
- Decision: ship market/token parity first, then re-architect privacy module.

2. Big-bang vs staged cutover
- Big-bang increases outage risk.
- Decision: staged dual-stack with feature flags.

3. Bridge usage
- Bridge adds operational overhead and relayer/security considerations.
- Decision: default to Base-native token deployment; evaluate bridge only if legacy liquidity migration is required.

## Started Execution
- [x] Research completed and consolidated
- [x] Migration plan documented
- [x] Foundry workspace scaffolded
- [x] Base contracts v0 implemented (`NeuraToken`, `MarketCore`)
- [x] Base contracts v0.1 implemented (`OrderBook`, `CollateralVault` skeletons)
- [x] Base deploy scripts and env templates added
- [x] Backend Base config stubs added (`EVM_ENABLED`, Base RPC/chain settings)
- [x] Backend auth safety gate added (reject legacy Solana login path when `EVM_ENABLED=true`)
- [x] Backend SIWE auth endpoints added (`/v1/auth/siwe/nonce`, `/v1/auth/siwe/login`)
- [x] Backend SIWE wallet validation hardened to EIP-55 checksum addresses
- [x] Backend first Base read endpoint added (`/v1/evm/token/state`)
- [x] Backend Base market read endpoint added (`/v1/evm/markets`)
- [x] Backend Base orderbook read endpoint added (`/v1/evm/markets/{id}/orderbook`)
- [x] Backend Base trades read endpoint added (`/v1/evm/markets/{id}/trades`)
- [x] Frontend chain-mode constants added for Solana/Base routing
- [x] Frontend wagmi/viem provider stack + Base wallet hook scaffold added
- [x] Frontend auth and connect flows branched for SIWE/Base (`useAuth`, `/api/auth`, header)
- [x] Frontend first Base data read wired into UI (`/settings` token state panel)
- [x] Frontend market list/detail hooks switched to Base read path (`useMarkets` + `/v1/evm/markets`)
- [x] Frontend orderbook hook switched to Base read path (`/v1/evm/markets/{id}/orderbook`)
- [x] Frontend trades hook switched to Base read path (`/v1/evm/markets/{id}/trades`)
- [x] Base launch runbook added (`docs/runbooks/BASE_MAINNET_LAUNCH.md`)
- [x] Base synthetic monitoring checks added (`scripts/synthetic-monitor.mjs --chain-mode base`)
- [x] Launch readiness script supports Base live probes (`scripts/launch-readiness.sh --api-url ... --chain-mode=base`)
- [x] Base Sepolia backend smoke script added (`scripts/base-sepolia-smoke.mjs`)
- [x] Base Sepolia frontend smoke harness added (`scripts/base-sepolia-web-smoke.mjs`, `web/e2e/base-sepolia.spec.ts`)
- [x] CI gate added for Foundry tests (`forge test --root evm`)
- [x] Contract test baseline passing (`forge test`: 24/24)
- [x] Order matching + payout claim path implemented on Base (`OrderBook.matchOrders`, `OrderBook.claim`)
- [x] Timelock governance path added (`DeployTimelock.s.sol`, `HandoffToTimelock.s.sol`, `TimelockGovernance.t.sol`)
- [x] Backend EVM RPC service + log indexer added (`app/src/services/evm_rpc.rs`, `app/src/services/evm_indexer.rs`)
- [x] Backend dual read/write toggles added (`LEGACY_*_ENABLED`, `EVM_*_ENABLED`)
- [x] Frontend Base write flows enabled (`CreateMarketForm`, `usePlaceOrder`, `useCancelOrder`, `useClaimWinnings`)
- [x] Agent SDK migrated from Solana client to Base viem client (`sdk/agent`)

## Source Links
- https://docs.base.org/base-chain/quickstart/connecting-to-base
- https://docs.base.org/get-started/deploy-smart-contracts
- https://docs.base.org/get-started/launch-token
- https://docs.base.org/base-account/reference/core/capabilities/signInWithEthereum
- https://docs.base.org/base-chain/quickstart/base-solana-bridge
- https://docs.openzeppelin.com/contracts/5.x/api/token/erc20
