#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ZERO_ADDR="0x0000000000000000000000000000000000000000"

NETWORK=""
ACCOUNT="${FUNDING_ACCOUNT:-base-admin}"
PASSWORD_FILE="${ROOT_DIR}/keys/base-keystore-password.local"
DRY_RUN=0
ALLOW_NON_ADMIN_SIGNER=0

ETH_DEPLOYER=""
ETH_PAUSER=""
ETH_RESOLVER=""
ETH_MATCHER=""
ETH_OPERATOR=""
USDC_OPERATOR="0"

usage() {
  cat <<USAGE
Usage: scripts/base-distribute-funds.sh --network sepolia|mainnet [options]

Options:
  --account <alias>             Foundry keystore account alias (default: base-admin)
  --password-file <path>        Keystore password file
  --eth-deployer <amount>       ETH for deployer wallet
  --eth-pauser <amount>         ETH for pauser wallet
  --eth-resolver <amount>       ETH for resolver wallet
  --eth-matcher <amount>        ETH for matcher wallet
  --eth-operator <amount>       ETH for operator wallet
  --usdc-operator <amount>      USDC for operator wallet (decimals=6)
  --allow-non-admin-signer      Skip strict signer == BASE_ADMIN check
  --dry-run                     Print planned transactions only
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
    --account)
      ACCOUNT="${2:-}"
      shift 2
      ;;
    --account=*)
      ACCOUNT="${1#*=}"
      shift
      ;;
    --password-file)
      PASSWORD_FILE="${2:-}"
      shift 2
      ;;
    --password-file=*)
      PASSWORD_FILE="${1#*=}"
      shift
      ;;
    --eth-deployer)
      ETH_DEPLOYER="${2:-}"
      shift 2
      ;;
    --eth-deployer=*)
      ETH_DEPLOYER="${1#*=}"
      shift
      ;;
    --eth-pauser)
      ETH_PAUSER="${2:-}"
      shift 2
      ;;
    --eth-pauser=*)
      ETH_PAUSER="${1#*=}"
      shift
      ;;
    --eth-resolver)
      ETH_RESOLVER="${2:-}"
      shift 2
      ;;
    --eth-resolver=*)
      ETH_RESOLVER="${1#*=}"
      shift
      ;;
    --eth-matcher)
      ETH_MATCHER="${2:-}"
      shift 2
      ;;
    --eth-matcher=*)
      ETH_MATCHER="${1#*=}"
      shift
      ;;
    --eth-operator)
      ETH_OPERATOR="${2:-}"
      shift 2
      ;;
    --eth-operator=*)
      ETH_OPERATOR="${1#*=}"
      shift
      ;;
    --usdc-operator)
      USDC_OPERATOR="${2:-}"
      shift 2
      ;;
    --usdc-operator=*)
      USDC_OPERATOR="${1#*=}"
      shift
      ;;
    --allow-non-admin-signer)
      ALLOW_NON_ADMIN_SIGNER=1
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

if [[ "$NETWORK" != "sepolia" && "$NETWORK" != "mainnet" ]]; then
  echo "--network must be sepolia or mainnet" >&2
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

if [[ -f "$ROOT_DIR/keys/base-role-wallets.local" ]]; then
  set -a
  # shellcheck disable=SC1091
  source "$ROOT_DIR/keys/base-role-wallets.local"
  set +a
fi

if [[ -z "${BASE_ADMIN:-}" ]]; then
  echo "BASE_ADMIN is not set" >&2
  exit 1
fi

if [[ ! -f "$PASSWORD_FILE" ]]; then
  echo "Password file not found: $PASSWORD_FILE" >&2
  exit 1
fi

BASE_DEPLOYER="${BASE_DEPLOYER:-}"
BASE_PAUSER="${BASE_PAUSER:-}"
BASE_RESOLVER="${BASE_RESOLVER:-}"
BASE_MATCHER="${BASE_MATCHER:-}"
BASE_OPERATOR="${BASE_OPERATOR:-}"

if [[ -z "$BASE_DEPLOYER" || -z "$BASE_PAUSER" || -z "$BASE_RESOLVER" || -z "$BASE_MATCHER" || -z "$BASE_OPERATOR" ]]; then
  echo "One or more role addresses are missing. Check keys/base-role-wallets.local or .env." >&2
  exit 1
fi

if [[ "$NETWORK" == "sepolia" ]]; then
  RPC_URL="${BASE_SEPOLIA_RPC_URL:-https://sepolia.base.org}"
  COLLATERAL_TOKEN="${COLLATERAL_TOKEN_BASE_SEPOLIA:-$ZERO_ADDR}"
  ETH_DEPLOYER="${ETH_DEPLOYER:-0.02}"
  ETH_PAUSER="${ETH_PAUSER:-0.005}"
  ETH_RESOLVER="${ETH_RESOLVER:-0.005}"
  ETH_MATCHER="${ETH_MATCHER:-0.005}"
  ETH_OPERATOR="${ETH_OPERATOR:-0}"
else
  RPC_URL="${BASE_RPC_URL:-https://mainnet.base.org}"
  COLLATERAL_TOKEN="${COLLATERAL_TOKEN_BASE_MAINNET:-$ZERO_ADDR}"
  ETH_DEPLOYER="${ETH_DEPLOYER:-0.003}"
  ETH_PAUSER="${ETH_PAUSER:-0.0015}"
  ETH_RESOLVER="${ETH_RESOLVER:-0.0015}"
  ETH_MATCHER="${ETH_MATCHER:-0.0015}"
  ETH_OPERATOR="${ETH_OPERATOR:-0}"
fi

if [[ "${COLLATERAL_TOKEN_ADDRESS:-$ZERO_ADDR}" != "$ZERO_ADDR" ]]; then
  COLLATERAL_TOKEN="$COLLATERAL_TOKEN_ADDRESS"
fi

lower() {
  echo "$1" | tr '[:upper:]' '[:lower:]'
}

if ! SIGNER_ADDRESS="$(cast wallet address --account "$ACCOUNT" --password-file "$PASSWORD_FILE" 2>/dev/null)"; then
  echo "Could not unlock account alias '$ACCOUNT'. Import the wallet into Foundry keystore first." >&2
  exit 1
fi

if [[ "$ALLOW_NON_ADMIN_SIGNER" -eq 0 && "$(lower "$SIGNER_ADDRESS")" != "$(lower "$BASE_ADMIN")" ]]; then
  cat <<MSG >&2
Signer/account mismatch:
- account alias '$ACCOUNT' resolves to $SIGNER_ADDRESS
- BASE_ADMIN is set to $BASE_ADMIN
Import BASE_ADMIN into Foundry keystore (recommended alias: base-admin), or pass --allow-non-admin-signer if intentional.
MSG
  exit 1
fi

to_wei() {
  cast --to-wei "$1" ether
}

queue_send_eth() {
  local label="$1"
  local to="$2"
  local amount_eth="$3"

  if [[ -z "$amount_eth" || "$amount_eth" == "0" || "$amount_eth" == "0.0" ]]; then
    return 0
  fi
  if [[ "$to" == "$ZERO_ADDR" || -z "$to" ]]; then
    echo "Skipping $label: zero or empty address"
    return 0
  fi

  local wei
  wei="$(to_wei "$amount_eth")"
  REQUIRED_WEI=$((REQUIRED_WEI + wei))
  TRANSFER_LABELS+=("$label")
  TRANSFER_TO+=("$to")
  TRANSFER_WEI+=("$wei")
  TRANSFER_ETH+=("$amount_eth")
}

REQUIRED_WEI=0
TRANSFER_LABELS=()
TRANSFER_TO=()
TRANSFER_WEI=()
TRANSFER_ETH=()

queue_send_eth "deployer" "$BASE_DEPLOYER" "$ETH_DEPLOYER"
queue_send_eth "pauser" "$BASE_PAUSER" "$ETH_PAUSER"
queue_send_eth "resolver" "$BASE_RESOLVER" "$ETH_RESOLVER"
queue_send_eth "matcher" "$BASE_MATCHER" "$ETH_MATCHER"
queue_send_eth "operator" "$BASE_OPERATOR" "$ETH_OPERATOR"

SIGNER_BALANCE_WEI="$(cast balance --rpc-url "$RPC_URL" "$SIGNER_ADDRESS")"

if (( SIGNER_BALANCE_WEI < REQUIRED_WEI )); then
  echo "Insufficient ETH for planned transfers." >&2
  echo "Signer balance: $(cast --to-unit "$SIGNER_BALANCE_WEI" ether) ETH" >&2
  echo "Required amount: $(cast --to-unit "$REQUIRED_WEI" ether) ETH (fees not included)" >&2
  exit 1
fi

for idx in "${!TRANSFER_TO[@]}"; do
  echo "ETH transfer [${TRANSFER_LABELS[$idx]}] -> ${TRANSFER_TO[$idx]} : ${TRANSFER_ETH[$idx]}"

  if [[ "$DRY_RUN" -eq 0 ]]; then
    cast send \
      --rpc-url "$RPC_URL" \
      --account "$ACCOUNT" \
      --password-file "$PASSWORD_FILE" \
      "${TRANSFER_TO[$idx]}" \
      --value "${TRANSFER_WEI[$idx]}"
  fi
done

if [[ "$USDC_OPERATOR" != "0" && "$USDC_OPERATOR" != "0.0" ]]; then
  if [[ "$COLLATERAL_TOKEN" == "$ZERO_ADDR" || -z "$COLLATERAL_TOKEN" ]]; then
    echo "USDC transfer requested, but collateral token is not configured." >&2
    exit 1
  fi

  USDC_RAW="$(cast --to-wei "$USDC_OPERATOR" mwei)"
  echo "USDC transfer [operator] -> $BASE_OPERATOR : $USDC_OPERATOR"

  if [[ "$DRY_RUN" -eq 0 ]]; then
    ADMIN_USDC_RAW="$(cast call --rpc-url "$RPC_URL" "$COLLATERAL_TOKEN" "balanceOf(address)(uint256)" "$SIGNER_ADDRESS")"
    if (( ADMIN_USDC_RAW < USDC_RAW )); then
      echo "Insufficient USDC for transfer." >&2
      echo "Signer USDC: $(cast --to-unit "$ADMIN_USDC_RAW" 6)" >&2
      echo "Required USDC: $USDC_OPERATOR" >&2
      exit 1
    fi

    cast send \
      --rpc-url "$RPC_URL" \
      --account "$ACCOUNT" \
      --password-file "$PASSWORD_FILE" \
      "$COLLATERAL_TOKEN" \
      "transfer(address,uint256)(bool)" \
      "$BASE_OPERATOR" \
      "$USDC_RAW"
  fi
fi

echo ""
echo "Funding plan complete for $NETWORK"
echo "Signer: $SIGNER_ADDRESS"
echo "Total planned ETH transfer: $(cast --to-unit "$REQUIRED_WEI" ether)"
if [[ "$DRY_RUN" -eq 1 ]]; then
  echo "Dry run only. No transactions were sent."
fi
