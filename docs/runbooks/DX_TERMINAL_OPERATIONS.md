# DX Terminal Operations Runbook

Last updated: 2026-02-26

## Purpose

Use DX Terminal as an external operator and signal surface during Base mainnet launch:

- Monitor vault-level activity, swaps, strategy logs, and leaderboard context.
- Run optional strategy/funding actions for a separate DX vault.
- Export periodic snapshots for incident review and postmortems.

This is **not** the NeuraMinds core execution path. Core market create/match/resolve remains in NeuraMinds contracts and services.

## Prerequisites

- `cast`, `curl`, and `jq` installed.
- `scripts/dx-terminal-pro.sh` executable.
- Env loaded from `.env` and/or `.env.secrets.local`.

Required for write actions:

- `DX_TERMINAL_PRIVATE_KEY`

Optional but useful:

- `DX_TERMINAL_OWNER_ADDRESS`
- `DX_TERMINAL_VAULT_ADDRESS`
- `DX_TERMINAL_API_URL` (default `https://api.terminal.markets`)
- `DX_TERMINAL_RPC_URL` (default `https://mainnet.base.org`)

## Fast Start

Read-only checks:

```bash
npm run dx:vault
npm run dx:positions
npm run dx:swaps
npm run dx:logs
npm run dx:leaderboard
npm run dx:snapshot
```

Direct command usage:

```bash
bash scripts/dx-terminal-pro.sh help
bash scripts/dx-terminal-pro.sh tokens true
bash scripts/dx-terminal-pro.sh pnl-history
bash scripts/dx-terminal-pro.sh deposits-withdrawals 50
```

Integrated launch flow:

```bash
npm run launch:readiness:strict:dx
```

`launch-readiness.sh` now auto-captures DX snapshots when DX env is present. Use:

- `--require-dx-snapshot` to fail if snapshot cannot be captured.
- `--skip-dx-snapshot` to disable DX capture.
- `--dx-snapshot-out=<path>` to override report output path.

## Write Flows (Optional)

Use writes only if you are actively operating a DX vault alongside NeuraMinds.

Dry run first:

```bash
bash scripts/dx-terminal-pro.sh update-settings 5000 200 3 3 3 3 3 --dry-run
```

Then execute:

```bash
bash scripts/dx-terminal-pro.sh update-settings 5000 200 3 3 3 3 3
bash scripts/dx-terminal-pro.sh add-strategy 2 0 "Keep 20% ETH idle, rotate into highest momentum leaders"
bash scripts/dx-terminal-pro.sh disable-strategy 1
bash scripts/dx-terminal-pro.sh deposit 0.05
bash scripts/dx-terminal-pro.sh withdraw 50000000000000000
```

## Operational Cadence

During launch window:

1. Generate snapshot every 30-60 minutes:
   - `npm run dx:snapshot`
2. Compare swaps/logs against NeuraMinds order flow for anomaly correlation.
3. If needed, tune DX strategy sliders using `update-settings` and record change rationale in launch notes.

## Guardrails

- Never store real private keys in tracked files.
- Keep `DX_TERMINAL_PRIVATE_KEY` only in local secret env sources.
- Treat DX as auxiliary intelligence/ops tooling, not a dependency for NeuraMinds settlement correctness.
