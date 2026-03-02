# Base Global Payout Worker Runbook

## Purpose

`scripts/base-global-payout-worker.sh` runs continuous payout claiming for resolved Base markets and delegates execution to `scripts/base-auto-claimer.sh`.

The worker now covers users and agent owners by combining three candidate sources each cycle:

1. Backend payout candidates: `GET /v1/evm/payouts/candidates`
2. Active agent owners from `GET /v1/evm/agents`
3. Recent onchain order makers from `OrderBook.orders`

For each claimable candidate it:

1. Calls `OrderBook.claimFor(owner, marketId)`
2. Strictly validates the claim transaction and receipt (target, selector, args, status, `Claimed` event topics)
3. Optionally auto-withdraws from `CollateralVault` to the owner wallet (managed mode) with strict withdraw tx validation

## Required Environment

- `API_URL` (or `--api-url`)
- `ORDER_BOOK_ADDRESS`
- Claimer key (one of):
  - `AUTO_CLAIMER_PRIVATE_KEY`
  - `BASE_GLOBAL_CLAIMER_PRIVATE_KEY`
  - `BASE_AGENT_RUNTIME_OPERATOR_PRIVATE_KEY`
- `BASE_RPC_URL` (mainnet) or `BASE_SEPOLIA_RPC_URL` (sepolia)

Managed auto-withdraw (optional):

- `COLLATERAL_VAULT_ADDRESS`
- Owner key map via one of:
  - `AUTO_CLAIMER_OWNER_KEYS_FILE`
  - `AUTO_CLAIMER_OWNER_KEYS_JSON`

Owner key map format:

```json
{
  "0xabc...": "0xowner_private_key_hex",
  "0xdef...": "0xowner_private_key_hex"
}
```

## Local Commands

Mainnet:

```bash
npm run payouts:worker
```

Sepolia:

```bash
npm run payouts:worker:sepolia
```

Dry-run single cycle:

```bash
npm run payouts:worker:dry
```

Managed auto-withdraw (mainnet):

```bash
npm run payouts:worker:auto-withdraw
```

Managed auto-withdraw (sepolia):

```bash
npm run payouts:worker:auto-withdraw:sepolia
```

## Useful Flags

- `--interval-sec <seconds>` polling interval (default `30`)
- `--batch-size <count>` payout candidate API scan limit (default `1000`)
- `--agent-scan-limit <count>` agent scan limit (default `1000`)
- `--order-scan-window <count>` latest order ids scanned for owners (default `2500`)
- `--max-claims-per-cycle <count>` claim tx cap per cycle (`0` = unlimited, default `200`)
- `--auto-withdraw true|false` managed withdraw mode (default `false`)
- `--owner-keys-file <path>` owner private key map file
- `--owner-keys-json <json>` inline owner private key map
- `--once` run one cycle and exit

## Process Model

Run this as a dedicated long-lived worker process (Render background service, systemd unit, or tmux/screen session).
Do not rely on web request traffic to drive claims.

## Failure Modes

- `missing required binary: curl|jq|cast`
  Install curl, jq, and Foundry.
- `failed to query payout candidates from API`
  Check `API_URL` and backend connectivity.
- `failed claim ... reason=validation_failed`
  Claimer tx submitted but failed strict receipt checks (target/method/args/logs).
- `skip auto-withdraw ... reason=missing_owner_key`
  Managed withdraw enabled but owner key missing from key map.
- `failed auto-withdraw ... reason=validation_failed`
  Withdraw tx submitted but failed strict withdraw validation.
