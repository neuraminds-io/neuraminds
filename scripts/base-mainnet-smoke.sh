#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

NETWORK="mainnet"
RPC_URL="${BASE_RPC_URL:-https://mainnet.base.org}"
DRY_RUN=0
DEPOSIT_AMOUNT="300000"
ORDER_SIZE="100000"
YES_PRICE_BPS="5500"
NO_PRICE_BPS="4500"
OUTCOME="yes"
CLOSE_DELAY_SEC="120"
ORDER_EXPIRY_SEC="1800"
REPORT_OUT=""

MARKET_CORE_ADDRESS="${MARKET_CORE_ADDRESS:-}"
ORDER_BOOK_ADDRESS="${ORDER_BOOK_ADDRESS:-}"
COLLATERAL_VAULT_ADDRESS="${COLLATERAL_VAULT_ADDRESS:-}"
COLLATERAL_TOKEN_ADDRESS="${COLLATERAL_TOKEN_ADDRESS:-${NEXT_PUBLIC_COLLATERAL_TOKEN_ADDRESS:-}}"

SMOKE_ADMIN_PRIVATE_KEY="${BASE_SMOKE_ADMIN_PRIVATE_KEY:-}"
SMOKE_YES_TRADER_PRIVATE_KEY="${BASE_SMOKE_YES_TRADER_PRIVATE_KEY:-}"
SMOKE_NO_TRADER_PRIVATE_KEY="${BASE_SMOKE_NO_TRADER_PRIVATE_KEY:-}"

usage() {
  cat <<USAGE
Usage: scripts/base-mainnet-smoke.sh [options]

Options:
  --network mainnet|sepolia           Target chain (default: mainnet)
  --rpc-url <url>                     RPC URL override
  --deposit-amount <units>            Collateral deposit per trader (default: 300000 = 0.3 USDC)
  --order-size <units>                Matched size (default: 100000 = 0.1 USDC)
  --yes-price-bps <bps>               YES order price bps (default: 5500)
  --no-price-bps <bps>                NO order price bps (default: 4500)
  --outcome yes|no                    Resolution outcome (default: yes)
  --close-delay-sec <seconds>         Market close delay (default: 120)
  --order-expiry-sec <seconds>        Order expiry delay (default: 1800)
  --report-out <path>                 JSON report output path
  --dry-run                           Print actions without sending txs
  -h|--help                           Show this help

Environment (required for live execution):
  BASE_SMOKE_ADMIN_PRIVATE_KEY
  BASE_SMOKE_YES_TRADER_PRIVATE_KEY
  BASE_SMOKE_NO_TRADER_PRIVATE_KEY    Optional; defaults to YES trader key

Contract addresses are read from env first:
  MARKET_CORE_ADDRESS, ORDER_BOOK_ADDRESS, COLLATERAL_VAULT_ADDRESS, COLLATERAL_TOKEN_ADDRESS

Fallback:
  docs/reports/base-programs-deploy-<network>.json for contract addresses.
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
    --deposit-amount)
      DEPOSIT_AMOUNT="${2:-}"
      shift 2
      ;;
    --deposit-amount=*)
      DEPOSIT_AMOUNT="${1#*=}"
      shift
      ;;
    --order-size)
      ORDER_SIZE="${2:-}"
      shift 2
      ;;
    --order-size=*)
      ORDER_SIZE="${1#*=}"
      shift
      ;;
    --yes-price-bps)
      YES_PRICE_BPS="${2:-}"
      shift 2
      ;;
    --yes-price-bps=*)
      YES_PRICE_BPS="${1#*=}"
      shift
      ;;
    --no-price-bps)
      NO_PRICE_BPS="${2:-}"
      shift 2
      ;;
    --no-price-bps=*)
      NO_PRICE_BPS="${1#*=}"
      shift
      ;;
    --outcome)
      OUTCOME="${2:-}"
      shift 2
      ;;
    --outcome=*)
      OUTCOME="${1#*=}"
      shift
      ;;
    --close-delay-sec)
      CLOSE_DELAY_SEC="${2:-}"
      shift 2
      ;;
    --close-delay-sec=*)
      CLOSE_DELAY_SEC="${1#*=}"
      shift
      ;;
    --order-expiry-sec)
      ORDER_EXPIRY_SEC="${2:-}"
      shift 2
      ;;
    --order-expiry-sec=*)
      ORDER_EXPIRY_SEC="${1#*=}"
      shift
      ;;
    --report-out)
      REPORT_OUT="${2:-}"
      shift 2
      ;;
    --report-out=*)
      REPORT_OUT="${1#*=}"
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

if [[ "$OUTCOME" != "yes" && "$OUTCOME" != "no" ]]; then
  echo "--outcome must be yes or no" >&2
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

require_bins() {
  local missing=0
  for bin in cast jq; do
    if ! command -v "$bin" >/dev/null 2>&1; then
      echo "missing required binary: $bin" >&2
      missing=1
    fi
  done
  if [[ "$missing" -eq 1 ]]; then
    exit 1
  fi
}

is_uint() {
  [[ "$1" =~ ^[0-9]+$ ]]
}

as_dec() {
  local value="$1"
  if [[ "$value" =~ ^0x ]]; then
    cast --to-dec "$value"
  else
    echo "$value"
  fi
}

call_uint() {
  local addr="$1"
  local sig="$2"
  local raw
  raw="$(cast call --rpc-url "$RPC_URL" "$addr" "$sig")"
  as_dec "$raw"
}

call_bool() {
  local addr="$1"
  local sig="$2"
  shift 2
  local raw
  raw="$(cast call --rpc-url "$RPC_URL" "$addr" "$sig" "$@")"
  case "$raw" in
    true|1|0x1)
      echo "true"
      ;;
    *)
      echo "false"
      ;;
  esac
}

tx_send() {
  local private_key="$1"
  shift
  local cmd=(cast send --rpc-url "$RPC_URL" --private-key "$private_key" "$@")

  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "dry-run: ${cmd[*]}"
    echo "0x0000000000000000000000000000000000000000000000000000000000000000"
    return 0
  fi

  local output
  output="$("${cmd[@]}" 2>&1)"
  local tx_hash
  tx_hash="$(echo "$output" | grep -Eo '0x[0-9a-fA-F]{64}' | head -n 1 || true)"
  if [[ -z "$tx_hash" ]]; then
    echo "$output" >&2
    echo "failed to parse transaction hash" >&2
    exit 1
  fi

  echo "$tx_hash"
}

resolve_contracts_from_report() {
  local report="$ROOT_DIR/docs/reports/base-programs-deploy-${NETWORK}.json"
  if [[ ! -f "$report" ]]; then
    return 0
  fi

  if [[ -z "$MARKET_CORE_ADDRESS" ]]; then
    MARKET_CORE_ADDRESS="$(jq -r '.contracts.marketCore // empty' "$report")"
  fi
  if [[ -z "$ORDER_BOOK_ADDRESS" ]]; then
    ORDER_BOOK_ADDRESS="$(jq -r '.contracts.orderBook // empty' "$report")"
  fi
  if [[ -z "$COLLATERAL_VAULT_ADDRESS" ]]; then
    COLLATERAL_VAULT_ADDRESS="$(jq -r '.contracts.collateralVault // empty' "$report")"
  fi
}

address_or_fail() {
  local value="$1"
  local label="$2"
  if [[ ! "$value" =~ ^0x[0-9a-fA-F]{40}$ ]]; then
    echo "$label is missing or invalid: $value" >&2
    exit 1
  fi
}

require_bins

if [[ "$NETWORK" == "sepolia" && -n "${BASE_SEPOLIA_RPC_URL:-}" ]]; then
  RPC_URL="${BASE_SEPOLIA_RPC_URL}"
fi

if [[ ! -n "$RPC_URL" ]]; then
  echo "RPC URL is required" >&2
  exit 1
fi

if [[ "$DRY_RUN" -eq 0 ]]; then
  if [[ -z "$SMOKE_ADMIN_PRIVATE_KEY" || -z "$SMOKE_YES_TRADER_PRIVATE_KEY" ]]; then
    echo "BASE_SMOKE_ADMIN_PRIVATE_KEY and BASE_SMOKE_YES_TRADER_PRIVATE_KEY are required" >&2
    exit 1
  fi
fi

if [[ -z "$SMOKE_NO_TRADER_PRIVATE_KEY" ]]; then
  SMOKE_NO_TRADER_PRIVATE_KEY="$SMOKE_YES_TRADER_PRIVATE_KEY"
fi

resolve_contracts_from_report

if [[ -z "$COLLATERAL_TOKEN_ADDRESS" && "$NETWORK" == "mainnet" ]]; then
  COLLATERAL_TOKEN_ADDRESS="0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
fi

address_or_fail "$MARKET_CORE_ADDRESS" "MARKET_CORE_ADDRESS"
address_or_fail "$ORDER_BOOK_ADDRESS" "ORDER_BOOK_ADDRESS"
address_or_fail "$COLLATERAL_VAULT_ADDRESS" "COLLATERAL_VAULT_ADDRESS"
address_or_fail "$COLLATERAL_TOKEN_ADDRESS" "COLLATERAL_TOKEN_ADDRESS"

for value in "$DEPOSIT_AMOUNT" "$ORDER_SIZE" "$YES_PRICE_BPS" "$NO_PRICE_BPS" "$CLOSE_DELAY_SEC" "$ORDER_EXPIRY_SEC"; do
  if ! is_uint "$value"; then
    echo "numeric argument validation failed: $value" >&2
    exit 1
  fi
done

if (( YES_PRICE_BPS < 1 || YES_PRICE_BPS > 9999 || NO_PRICE_BPS < 1 || NO_PRICE_BPS > 9999 )); then
  echo "price bps must be in range 1..9999" >&2
  exit 1
fi

if (( YES_PRICE_BPS + NO_PRICE_BPS < 10000 )); then
  echo "YES and NO prices must cross (sum >= 10000)" >&2
  exit 1
fi

if [[ -z "$REPORT_OUT" ]]; then
  REPORT_OUT="$ROOT_DIR/docs/reports/base-${NETWORK}-smoke-report.json"
fi
mkdir -p "$(dirname "$REPORT_OUT")"

if [[ "$DRY_RUN" -eq 1 ]]; then
  ADMIN_ADDR="0x0000000000000000000000000000000000000001"
  YES_TRADER_ADDR="0x0000000000000000000000000000000000000002"
  NO_TRADER_ADDR="0x0000000000000000000000000000000000000003"
else
  ADMIN_ADDR="$(cast wallet address --private-key "$SMOKE_ADMIN_PRIVATE_KEY")"
  YES_TRADER_ADDR="$(cast wallet address --private-key "$SMOKE_YES_TRADER_PRIVATE_KEY")"
  NO_TRADER_ADDR="$(cast wallet address --private-key "$SMOKE_NO_TRADER_PRIVATE_KEY")"
fi

echo "base smoke start"
echo "network=$NETWORK"
echo "rpc_url=$RPC_URL"
echo "dry_run=$DRY_RUN"
echo "market_core=$MARKET_CORE_ADDRESS"
echo "order_book=$ORDER_BOOK_ADDRESS"
echo "collateral_vault=$COLLATERAL_VAULT_ADDRESS"
echo "collateral_token=$COLLATERAL_TOKEN_ADDRESS"
echo "admin=$ADMIN_ADDR"
echo "yes_trader=$YES_TRADER_ADDR"
echo "no_trader=$NO_TRADER_ADDR"

if [[ "$DRY_RUN" -eq 0 ]]; then
  MARKET_CREATOR_ROLE="$(cast call --rpc-url "$RPC_URL" "$MARKET_CORE_ADDRESS" "MARKET_CREATOR_ROLE()(bytes32)")"
  RESOLVER_ROLE="$(cast call --rpc-url "$RPC_URL" "$MARKET_CORE_ADDRESS" "RESOLVER_ROLE()(bytes32)")"
  OPERATOR_ROLE="$(cast call --rpc-url "$RPC_URL" "$COLLATERAL_VAULT_ADDRESS" "OPERATOR_ROLE()(bytes32)")"

  if [[ "$(call_bool "$MARKET_CORE_ADDRESS" "hasRole(bytes32,address)(bool)" "$MARKET_CREATOR_ROLE" "$ADMIN_ADDR")" != "true" ]]; then
    echo "admin missing MARKET_CREATOR_ROLE" >&2
    exit 1
  fi
  if [[ "$(call_bool "$MARKET_CORE_ADDRESS" "hasRole(bytes32,address)(bool)" "$RESOLVER_ROLE" "$ADMIN_ADDR")" != "true" ]]; then
    echo "admin missing RESOLVER_ROLE" >&2
    exit 1
  fi
  if [[ "$(call_bool "$COLLATERAL_VAULT_ADDRESS" "hasRole(bytes32,address)(bool)" "$OPERATOR_ROLE" "$ORDER_BOOK_ADDRESS")" != "true" ]]; then
    echo "orderBook missing OPERATOR_ROLE in collateral vault" >&2
    exit 1
  fi
fi

APPROVE_YES_TX="$(tx_send "$SMOKE_YES_TRADER_PRIVATE_KEY" "$COLLATERAL_TOKEN_ADDRESS" "approve(address,uint256)" "$COLLATERAL_VAULT_ADDRESS" "$DEPOSIT_AMOUNT")"
DEPOSIT_YES_TX="$(tx_send "$SMOKE_YES_TRADER_PRIVATE_KEY" "$COLLATERAL_VAULT_ADDRESS" "deposit(uint256)" "$DEPOSIT_AMOUNT")"

APPROVE_NO_TX=""
DEPOSIT_NO_TX=""
if [[ "$SMOKE_NO_TRADER_PRIVATE_KEY" != "$SMOKE_YES_TRADER_PRIVATE_KEY" ]]; then
  APPROVE_NO_TX="$(tx_send "$SMOKE_NO_TRADER_PRIVATE_KEY" "$COLLATERAL_TOKEN_ADDRESS" "approve(address,uint256)" "$COLLATERAL_VAULT_ADDRESS" "$DEPOSIT_AMOUNT")"
  DEPOSIT_NO_TX="$(tx_send "$SMOKE_NO_TRADER_PRIVATE_KEY" "$COLLATERAL_VAULT_ADDRESS" "deposit(uint256)" "$DEPOSIT_AMOUNT")"
fi

NOW_TS="$(date +%s)"
CLOSE_TIME="$((NOW_TS + CLOSE_DELAY_SEC))"
EXPIRY_TIME="$((NOW_TS + ORDER_EXPIRY_SEC))"
RUN_ID="$(date -u +"%Y%m%dT%H%M%SZ")"
QUESTION="NeuraMinds Base smoke $RUN_ID?"
DESCRIPTION="Production smoke test for create/trade/match/resolve/claim."

CREATE_TX="$(tx_send "$SMOKE_ADMIN_PRIVATE_KEY" "$MARKET_CORE_ADDRESS" \
  "createMarketRich(string,string,string,string,uint64,address)" \
  "$QUESTION" "$DESCRIPTION" "smoke" "ops" "$CLOSE_TIME" "$ADMIN_ADDR")"

if [[ "$DRY_RUN" -eq 1 ]]; then
  CURRENT_MARKET_COUNT="$(call_uint "$MARKET_CORE_ADDRESS" "marketCount()(uint256)")"
  MARKET_ID="$((CURRENT_MARKET_COUNT + 1))"
else
  MARKET_ID="$(call_uint "$MARKET_CORE_ADDRESS" "marketCount()(uint256)")"
fi

PLACE_YES_TX="$(tx_send "$SMOKE_YES_TRADER_PRIVATE_KEY" "$ORDER_BOOK_ADDRESS" \
  "placeOrder(uint256,bool,uint128,uint128,uint64)" \
  "$MARKET_ID" "true" "$YES_PRICE_BPS" "$ORDER_SIZE" "$EXPIRY_TIME")"
if [[ "$DRY_RUN" -eq 1 ]]; then
  CURRENT_ORDER_COUNT="$(call_uint "$ORDER_BOOK_ADDRESS" "orderCount()(uint256)")"
  YES_ORDER_ID="$((CURRENT_ORDER_COUNT + 1))"
else
  YES_ORDER_ID="$(call_uint "$ORDER_BOOK_ADDRESS" "orderCount()(uint256)")"
fi

PLACE_NO_TX="$(tx_send "$SMOKE_NO_TRADER_PRIVATE_KEY" "$ORDER_BOOK_ADDRESS" \
  "placeOrder(uint256,bool,uint128,uint128,uint64)" \
  "$MARKET_ID" "false" "$NO_PRICE_BPS" "$ORDER_SIZE" "$EXPIRY_TIME")"
if [[ "$DRY_RUN" -eq 1 ]]; then
  NO_ORDER_ID="$((YES_ORDER_ID + 1))"
else
  NO_ORDER_ID="$(call_uint "$ORDER_BOOK_ADDRESS" "orderCount()(uint256)")"
fi

MATCH_TX="$(tx_send "$SMOKE_ADMIN_PRIVATE_KEY" "$ORDER_BOOK_ADDRESS" \
  "matchOrders(uint256,uint256,uint128)" \
  "$YES_ORDER_ID" "$NO_ORDER_ID" "$ORDER_SIZE")"

if [[ "$DRY_RUN" -eq 0 ]]; then
  echo "waiting for close time ($CLOSE_TIME) ..."
  while [[ "$(date +%s)" -lt "$CLOSE_TIME" ]]; do
    sleep 2
  done
fi

if [[ "$OUTCOME" == "yes" ]]; then
  RESOLVE_BOOL="true"
  WINNER_ADDR="$YES_TRADER_ADDR"
  WINNER_KEY="$SMOKE_YES_TRADER_PRIVATE_KEY"
else
  RESOLVE_BOOL="false"
  WINNER_ADDR="$NO_TRADER_ADDR"
  WINNER_KEY="$SMOKE_NO_TRADER_PRIVATE_KEY"
fi

RESOLVE_TX="$(tx_send "$SMOKE_ADMIN_PRIVATE_KEY" "$MARKET_CORE_ADDRESS" "resolveMarket(uint256,bool)" "$MARKET_ID" "$RESOLVE_BOOL")"
CLAIM_TX="$(tx_send "$WINNER_KEY" "$ORDER_BOOK_ADDRESS" "claim(uint256)" "$MARKET_ID")"

jq -n \
  --arg generatedAt "$(date -u +"%Y-%m-%dT%H:%M:%SZ")" \
  --arg network "$NETWORK" \
  --arg rpcUrl "$RPC_URL" \
  --argjson dryRun "$([[ "$DRY_RUN" -eq 1 ]] && echo true || echo false)" \
  --arg marketCore "$MARKET_CORE_ADDRESS" \
  --arg orderBook "$ORDER_BOOK_ADDRESS" \
  --arg collateralVault "$COLLATERAL_VAULT_ADDRESS" \
  --arg collateralToken "$COLLATERAL_TOKEN_ADDRESS" \
  --arg admin "$ADMIN_ADDR" \
  --arg yesTrader "$YES_TRADER_ADDR" \
  --arg noTrader "$NO_TRADER_ADDR" \
  --arg winner "$WINNER_ADDR" \
  --arg outcome "$OUTCOME" \
  --argjson marketId "$MARKET_ID" \
  --argjson yesOrderId "$YES_ORDER_ID" \
  --argjson noOrderId "$NO_ORDER_ID" \
  --arg approveYesTx "$APPROVE_YES_TX" \
  --arg depositYesTx "$DEPOSIT_YES_TX" \
  --arg approveNoTx "$APPROVE_NO_TX" \
  --arg depositNoTx "$DEPOSIT_NO_TX" \
  --arg createTx "$CREATE_TX" \
  --arg placeYesTx "$PLACE_YES_TX" \
  --arg placeNoTx "$PLACE_NO_TX" \
  --arg matchTx "$MATCH_TX" \
  --arg resolveTx "$RESOLVE_TX" \
  --arg claimTx "$CLAIM_TX" \
  '{
    generatedAt: $generatedAt,
    network: $network,
    rpcUrl: $rpcUrl,
    dryRun: $dryRun,
    contracts: {
      marketCore: $marketCore,
      orderBook: $orderBook,
      collateralVault: $collateralVault,
      collateralToken: $collateralToken
    },
    actors: {
      admin: $admin,
      yesTrader: $yesTrader,
      noTrader: $noTrader,
      winner: $winner
    },
    market: {
      marketId: $marketId,
      outcome: $outcome
    },
    orders: {
      yesOrderId: $yesOrderId,
      noOrderId: $noOrderId
    },
    txs: {
      approveYes: $approveYesTx,
      depositYes: $depositYesTx,
      approveNo: $approveNoTx,
      depositNo: $depositNoTx,
      createMarket: $createTx,
      placeYes: $placeYesTx,
      placeNo: $placeNoTx,
      match: $matchTx,
      resolve: $resolveTx,
      claim: $claimTx
    }
  }' > "$REPORT_OUT"

echo "base smoke complete"
echo "report: $REPORT_OUT"
