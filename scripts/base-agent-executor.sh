#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

NETWORK="mainnet"
RPC_URL=""
API_URL="${API_URL:-http://127.0.0.1:8080/v1}"
LIMIT="100"
INTERVAL_SEC="20"
DRY_RUN=0
ONCE=0

AGENT_RUNTIME_ADDRESS="${AGENT_RUNTIME_ADDRESS:-${NEXT_PUBLIC_AGENT_RUNTIME_ADDRESS:-}}"
EXECUTOR_PRIVATE_KEY="${BASE_AGENT_RUNTIME_OPERATOR_PRIVATE_KEY:-}"

usage() {
  cat <<USAGE
Usage: scripts/base-agent-executor.sh [options]

Continuously executes due agents from AgentRuntime using a signer wallet.

Options:
  --network mainnet|sepolia     Target chain (default: mainnet)
  --rpc-url <url>               RPC URL override
  --api-url <url>               API base URL (default: http://127.0.0.1:8080/v1)
  --limit <count>               Max active agents to scan each cycle (default: 100)
  --interval-sec <seconds>      Polling interval (default: 20)
  --once                        Run one cycle then exit
  --dry-run                     Print actions without sending transactions
  -h|--help                     Show this help

Required environment for live execution:
  AGENT_RUNTIME_ADDRESS
  BASE_AGENT_RUNTIME_OPERATOR_PRIVATE_KEY
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
    --limit)
      LIMIT="${2:-}"
      shift 2
      ;;
    --limit=*)
      LIMIT="${1#*=}"
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

if ! [[ "$LIMIT" =~ ^[0-9]+$ ]] || [[ "$LIMIT" -lt 1 ]]; then
  echo "--limit must be a positive integer" >&2
  exit 1
fi

if ! [[ "$INTERVAL_SEC" =~ ^[0-9]+$ ]] || [[ "$INTERVAL_SEC" -lt 1 ]]; then
  echo "--interval-sec must be a positive integer" >&2
  exit 1
fi

if [[ -f "$ROOT_DIR/.env" ]]; then
  set -a
  # shellcheck disable=SC1091
  source "$ROOT_DIR/.env"
  set +a
fi

if [[ -f "$ROOT_DIR/.env.secrets.local" ]]; then
  set -a
  # shellcheck disable=SC1091
  source "$ROOT_DIR/.env.secrets.local"
  set +a
fi

if [[ -z "$RPC_URL" ]]; then
  if [[ "$NETWORK" == "sepolia" ]]; then
    RPC_URL="${BASE_SEPOLIA_RPC_URL:-https://sepolia.base.org}"
  else
    RPC_URL="${BASE_RPC_URL:-https://mainnet.base.org}"
  fi
fi

AGENT_RUNTIME_ADDRESS="${AGENT_RUNTIME_ADDRESS:-${NEXT_PUBLIC_AGENT_RUNTIME_ADDRESS:-}}"
EXECUTOR_PRIVATE_KEY="${EXECUTOR_PRIVATE_KEY:-${BASE_AGENT_RUNTIME_OPERATOR_PRIVATE_KEY:-}}"

if [[ "$API_URL" == */v1 ]]; then
  API_URL="${API_URL%/}"
else
  API_URL="${API_URL%/}/v1"
fi

require_bin() {
  local missing=0
  for bin in curl jq cast; do
    if ! command -v "$bin" >/dev/null 2>&1; then
      echo "missing required binary: $bin" >&2
      missing=1
    fi
  done
  if [[ "$missing" -eq 1 ]]; then
    exit 1
  fi
}

poll_and_execute() {
  local payload
  if ! payload="$(curl -fsS "$API_URL/evm/agents?active=true&limit=$LIMIT")"; then
    echo "failed to query active agents from $API_URL" >&2
    return 1
  fi

  mapfile -t due_agents < <(echo "$payload" | jq -r '.agents[] | select(.active == true and .can_execute == true) | .id')

  if [[ "${#due_agents[@]}" -eq 0 ]]; then
    echo "no due agents ($(date -u +"%Y-%m-%dT%H:%M:%SZ"))"
    return 0
  fi

  echo "due agents: ${due_agents[*]} ($(date -u +"%Y-%m-%dT%H:%M:%SZ"))"

  local successes=0
  local failures=0

  for agent_id in "${due_agents[@]}"; do
    if [[ "$DRY_RUN" -eq 1 ]]; then
      echo "dry-run executeAgent($agent_id)"
      continue
    fi

    local output
    if output="$(cast send --async --rpc-url "$RPC_URL" --private-key "$EXECUTOR_PRIVATE_KEY" "$AGENT_RUNTIME_ADDRESS" "executeAgent(uint256)" "$agent_id" 2>&1)"; then
      successes=$((successes + 1))
      echo "executed agent=$agent_id tx=$output"
    else
      failures=$((failures + 1))
      echo "failed agent=$agent_id error=$output" >&2
    fi
  done

  echo "cycle result success=$successes failure=$failures"
  return 0
}

require_bin

if [[ "$DRY_RUN" -eq 0 ]]; then
  if [[ -z "$AGENT_RUNTIME_ADDRESS" ]]; then
    echo "AGENT_RUNTIME_ADDRESS is required" >&2
    exit 1
  fi

  if [[ -z "$EXECUTOR_PRIVATE_KEY" ]]; then
    echo "BASE_AGENT_RUNTIME_OPERATOR_PRIVATE_KEY is required" >&2
    exit 1
  fi
fi

echo "agent executor started network=$NETWORK api=$API_URL interval=${INTERVAL_SEC}s dry_run=$DRY_RUN"

while true; do
  if ! poll_and_execute; then
    if [[ "$ONCE" -eq 1 ]]; then
      exit 1
    fi
    sleep "$INTERVAL_SEC"
    continue
  fi

  if [[ "$ONCE" -eq 1 ]]; then
    break
  fi

  sleep "$INTERVAL_SEC"
done
