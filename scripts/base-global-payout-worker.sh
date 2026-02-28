#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
AUTO_CLAIMER_SCRIPT="$ROOT_DIR/scripts/base-auto-claimer.sh"

NETWORK="mainnet"
RPC_URL=""
API_URL="${API_URL:-http://127.0.0.1:8080/v1}"
INTERVAL_SEC="${AUTO_CLAIMER_POLL_INTERVAL_SEC:-30}"
BATCH_SIZE="${AUTO_CLAIMER_CANDIDATE_SCAN_LIMIT:-1000}"
AGENT_SCAN_LIMIT="${AUTO_CLAIMER_AGENT_SCAN_LIMIT:-1000}"
ORDER_SCAN_WINDOW="${AUTO_CLAIMER_ORDER_SCAN_WINDOW:-2500}"
MAX_CLAIMS_PER_CYCLE="${AUTO_CLAIMER_MAX_CLAIMS_PER_CYCLE:-200}"
AUTO_WITHDRAW="${AUTO_CLAIMER_AUTO_WITHDRAW:-false}"
OWNER_KEYS_FILE="${AUTO_CLAIMER_OWNER_KEYS_FILE:-}"
OWNER_KEYS_JSON="${AUTO_CLAIMER_OWNER_KEYS_JSON:-}"
DRY_RUN=0
ONCE=0

AGENT_SCAN_LIMIT_SET=0

usage() {
  cat <<USAGE
Usage: scripts/base-global-payout-worker.sh [options]

Compatibility wrapper around scripts/base-auto-claimer.sh.

Options:
  --network mainnet|sepolia         Target chain (default: mainnet)
  --rpc-url <url>                   RPC URL override
  --api-url <url>                   API base URL (default: http://127.0.0.1:8080/v1)
  --interval-sec <seconds>          Poll interval (default: 30)
  --batch-size <count>              Candidate API page size (default: 1000)
  --agent-scan-limit <count>        Agent scan page size (default: 1000)
  --order-scan-window <count>       Number of latest orders scanned for candidate owners (default: 2500)
  --max-claims-per-cycle <count>    Claim tx cap per cycle, 0 = unlimited (default: 200)
  --auto-withdraw true|false        Optional managed withdraw from vault to owner wallet (default: false)
  --owner-keys-file <path>          JSON map owner->private_key for managed withdraw
  --owner-keys-json <json>          Inline JSON map owner->private_key for managed withdraw
  --once                            Run one cycle then exit
  --dry-run                         Print actions without sending transactions
  -h|--help                         Show this help
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --network)
      NETWORK="${2:-}"
      shift 2
      ;;
    --network=*)
      NETWORK="${1#*=}"
      shift
      ;;
    --rpc-url)
      RPC_URL="${2:-}"
      shift 2
      ;;
    --rpc-url=*)
      RPC_URL="${1#*=}"
      shift
      ;;
    --api-url)
      API_URL="${2:-}"
      shift 2
      ;;
    --api-url=*)
      API_URL="${1#*=}"
      shift
      ;;
    --interval-sec)
      INTERVAL_SEC="${2:-}"
      shift 2
      ;;
    --interval-sec=*)
      INTERVAL_SEC="${1#*=}"
      shift
      ;;
    --batch-size)
      BATCH_SIZE="${2:-}"
      if [[ "$AGENT_SCAN_LIMIT_SET" -eq 0 ]]; then
        AGENT_SCAN_LIMIT="$BATCH_SIZE"
      fi
      shift 2
      ;;
    --batch-size=*)
      BATCH_SIZE="${1#*=}"
      if [[ "$AGENT_SCAN_LIMIT_SET" -eq 0 ]]; then
        AGENT_SCAN_LIMIT="$BATCH_SIZE"
      fi
      shift
      ;;
    --agent-scan-limit)
      AGENT_SCAN_LIMIT="${2:-}"
      AGENT_SCAN_LIMIT_SET=1
      shift 2
      ;;
    --agent-scan-limit=*)
      AGENT_SCAN_LIMIT="${1#*=}"
      AGENT_SCAN_LIMIT_SET=1
      shift
      ;;
    --order-scan-window)
      ORDER_SCAN_WINDOW="${2:-}"
      shift 2
      ;;
    --order-scan-window=*)
      ORDER_SCAN_WINDOW="${1#*=}"
      shift
      ;;
    --max-claims-per-cycle)
      MAX_CLAIMS_PER_CYCLE="${2:-}"
      shift 2
      ;;
    --max-claims-per-cycle=*)
      MAX_CLAIMS_PER_CYCLE="${1#*=}"
      shift
      ;;
    --auto-withdraw)
      AUTO_WITHDRAW="${2:-}"
      shift 2
      ;;
    --auto-withdraw=*)
      AUTO_WITHDRAW="${1#*=}"
      shift
      ;;
    --owner-keys-file)
      OWNER_KEYS_FILE="${2:-}"
      shift 2
      ;;
    --owner-keys-file=*)
      OWNER_KEYS_FILE="${1#*=}"
      shift
      ;;
    --owner-keys-json)
      OWNER_KEYS_JSON="${2:-}"
      shift 2
      ;;
    --owner-keys-json=*)
      OWNER_KEYS_JSON="${1#*=}"
      shift
      ;;
    --once)
      ONCE=1
      shift
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if [[ "$NETWORK" != "mainnet" && "$NETWORK" != "sepolia" ]]; then
  echo "--network must be mainnet or sepolia" >&2
  exit 1
fi

for value in "$INTERVAL_SEC" "$BATCH_SIZE" "$AGENT_SCAN_LIMIT" "$ORDER_SCAN_WINDOW"; do
  if ! [[ "$value" =~ ^[0-9]+$ ]] || [[ "$value" -lt 1 ]]; then
    echo "numeric options must be positive integers" >&2
    exit 1
  fi
done

if ! [[ "$MAX_CLAIMS_PER_CYCLE" =~ ^[0-9]+$ ]]; then
  echo "--max-claims-per-cycle must be a non-negative integer" >&2
  exit 1
fi

if [[ "$AUTO_WITHDRAW" != "true" && "$AUTO_WITHDRAW" != "false" ]]; then
  echo "--auto-withdraw must be true or false" >&2
  exit 1
fi

if [[ ! -f "$AUTO_CLAIMER_SCRIPT" ]]; then
  echo "missing required script: $AUTO_CLAIMER_SCRIPT" >&2
  exit 1
fi

cmd=(
  bash "$AUTO_CLAIMER_SCRIPT"
  --network "$NETWORK"
  --api-url "$API_URL"
  --poll-interval-sec "$INTERVAL_SEC"
  --candidate-scan-limit "$BATCH_SIZE"
  --agent-scan-limit "$AGENT_SCAN_LIMIT"
  --order-scan-window "$ORDER_SCAN_WINDOW"
  --max-claims-per-cycle "$MAX_CLAIMS_PER_CYCLE"
  --auto-withdraw "$AUTO_WITHDRAW"
)

if [[ -n "$RPC_URL" ]]; then
  cmd+=(--rpc-url "$RPC_URL")
fi
if [[ -n "$OWNER_KEYS_FILE" ]]; then
  cmd+=(--owner-keys-file "$OWNER_KEYS_FILE")
fi
if [[ -n "$OWNER_KEYS_JSON" ]]; then
  cmd+=(--owner-keys-json "$OWNER_KEYS_JSON")
fi
if [[ "$DRY_RUN" -eq 1 ]]; then
  cmd+=(--dry-run)
fi
if [[ "$ONCE" -eq 1 ]]; then
  cmd+=(--once)
fi

echo "global payout worker delegating to auto-claimer network=$NETWORK api=$API_URL interval=${INTERVAL_SEC}s batch_size=$BATCH_SIZE agent_scan_limit=$AGENT_SCAN_LIMIT order_scan_window=$ORDER_SCAN_WINDOW max_claims_per_cycle=$MAX_CLAIMS_PER_CYCLE auto_withdraw=$AUTO_WITHDRAW dry_run=$DRY_RUN"

exec "${cmd[@]}"
