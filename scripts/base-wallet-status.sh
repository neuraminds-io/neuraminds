#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NETWORK="both"

usage() {
  cat <<USAGE
Usage: scripts/base-wallet-status.sh [--network sepolia|mainnet|both]
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

case "$NETWORK" in
  sepolia|mainnet|both) ;;
  *)
    echo "Invalid --network value: $NETWORK" >&2
    exit 1
    ;;
esac

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

default_if_missing() {
  local value="$1"
  local fallback="$2"
  if [[ -n "$value" ]]; then
    echo "$value"
  else
    echo "$fallback"
  fi
}

BASE_SEPOLIA_RPC_URL="$(default_if_missing "${BASE_SEPOLIA_RPC_URL:-}" "https://sepolia.base.org")"
BASE_RPC_URL="$(default_if_missing "${BASE_RPC_URL:-}" "https://mainnet.base.org")"

# If COLLATERAL_TOKEN_ADDRESS is set and non-zero, it overrides per-network token settings.
ZERO_ADDR="0x0000000000000000000000000000000000000000"
COLLATERAL_TOKEN_GLOBAL="${COLLATERAL_TOKEN_ADDRESS:-$ZERO_ADDR}"
COLLATERAL_TOKEN_BASE_MAINNET="${COLLATERAL_TOKEN_BASE_MAINNET:-$ZERO_ADDR}"
COLLATERAL_TOKEN_BASE_SEPOLIA="${COLLATERAL_TOKEN_BASE_SEPOLIA:-$ZERO_ADDR}"

if [[ "$COLLATERAL_TOKEN_GLOBAL" != "$ZERO_ADDR" ]]; then
  COLLATERAL_TOKEN_BASE_MAINNET="$COLLATERAL_TOKEN_GLOBAL"
  COLLATERAL_TOKEN_BASE_SEPOLIA="$COLLATERAL_TOKEN_GLOBAL"
fi

BASE_ADMIN="${BASE_ADMIN:-}"
BASE_TREASURY="${BASE_TREASURY:-}"
BASE_DEPLOYER="${BASE_DEPLOYER:-}"
BASE_MARKET_CREATOR="${BASE_MARKET_CREATOR:-}"
BASE_PAUSER="${BASE_PAUSER:-}"
BASE_RESOLVER="${BASE_RESOLVER:-}"
BASE_MATCHER="${BASE_MATCHER:-}"
BASE_OPERATOR="${BASE_OPERATOR:-}"

if [[ -z "$BASE_DEPLOYER" && -f "$ROOT_DIR/keys/base-role-wallets.local" ]]; then
  BASE_DEPLOYER="$(awk -F= '/^BASE_DEPLOYER=/{print $2}' "$ROOT_DIR/keys/base-role-wallets.local" | tail -n 1 || true)"
fi

add_wallet() {
  local label="$1"
  local address="$2"
  if [[ -z "$address" ]]; then
    return 0
  fi
  WALLET_LABELS+=("$label")
  WALLET_ADDRESSES+=("$address")
}

WALLET_LABELS=()
WALLET_ADDRESSES=()
add_wallet "admin" "$BASE_ADMIN"
add_wallet "treasury" "$BASE_TREASURY"
add_wallet "deployer" "$BASE_DEPLOYER"
add_wallet "market_creator" "$BASE_MARKET_CREATOR"
add_wallet "pauser" "$BASE_PAUSER"
add_wallet "resolver" "$BASE_RESOLVER"
add_wallet "matcher" "$BASE_MATCHER"
add_wallet "operator" "$BASE_OPERATOR"

if [[ ${#WALLET_ADDRESSES[@]} -eq 0 ]]; then
  echo "No wallet addresses found in environment." >&2
  exit 1
fi

print_network() {
  local name="$1"
  local rpc="$2"
  local collateral_token="$3"
  local title
  title="$(echo "$name" | tr '[:lower:]' '[:upper:]')"

  echo ""
  echo "== ${title} =="
  echo "RPC: $rpc"

  for idx in "${!WALLET_ADDRESSES[@]}"; do
    local label="${WALLET_LABELS[$idx]}"
    local address="${WALLET_ADDRESSES[$idx]}"

    local wei
    wei="$(cast balance --rpc-url "$rpc" "$address")"
    local eth
    eth="$(cast --to-unit "$wei" ether)"

    printf "ETH %-14s %-42s %s\n" "[$label]" "$address" "$eth"
  done

  if [[ "$collateral_token" == "$ZERO_ADDR" || -z "$collateral_token" ]]; then
    echo "Collateral token: <not configured>"
    return 0
  fi

  echo "Collateral token: $collateral_token"
  for idx in "${!WALLET_ADDRESSES[@]}"; do
    local label="${WALLET_LABELS[$idx]}"
    local address="${WALLET_ADDRESSES[$idx]}"

    local raw
    raw="$(cast call --rpc-url "$rpc" "$collateral_token" "balanceOf(address)(uint256)" "$address")"
    local human
    human="$(cast --to-unit "$raw" 6)"

    printf "USDC %-13s %-42s %s\n" "[$label]" "$address" "$human"
  done
}

if [[ "$NETWORK" == "sepolia" || "$NETWORK" == "both" ]]; then
  print_network "base sepolia" "$BASE_SEPOLIA_RPC_URL" "$COLLATERAL_TOKEN_BASE_SEPOLIA"
fi

if [[ "$NETWORK" == "mainnet" || "$NETWORK" == "both" ]]; then
  print_network "base mainnet" "$BASE_RPC_URL" "$COLLATERAL_TOKEN_BASE_MAINNET"
fi
