#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

NETWORK="mainnet"
RPC_URL=""
FIXTURE=""
COUNT="20"
ORDER_SIZE="100000"
YES_PRICE_BPS="5300"
NO_PRICE_BPS="4700"
ORDER_EXPIRY_SEC="2592000"
DEFAULT_CLOSE_DELAY_SEC="2592000"
AGENT_COUNT="3"
AGENT_SIZE="50000"
AGENT_PRICE_BPS="5100"
AGENT_CADENCE="3600"
AGENT_EXPIRY_WINDOW="7200"
REPORT_OUT=""
DRY_RUN=0

MARKET_CORE_ADDRESS="${MARKET_CORE_ADDRESS:-}"
ORDER_BOOK_ADDRESS="${ORDER_BOOK_ADDRESS:-}"
COLLATERAL_VAULT_ADDRESS="${COLLATERAL_VAULT_ADDRESS:-${NEXT_PUBLIC_COLLATERAL_VAULT_ADDRESS:-}}"
COLLATERAL_TOKEN_ADDRESS="${COLLATERAL_TOKEN_ADDRESS:-${NEXT_PUBLIC_COLLATERAL_TOKEN_ADDRESS:-}}"
AGENT_RUNTIME_ADDRESS="${AGENT_RUNTIME_ADDRESS:-${NEXT_PUBLIC_AGENT_RUNTIME_ADDRESS:-}}"

CREATOR_PRIVATE_KEY="${BASE_PILOT_MARKET_CREATOR_PRIVATE_KEY:-${BASE_SMOKE_ADMIN_PRIVATE_KEY:-${BASE_ADMIN_PRIVATE_KEY:-}}}"
CREATOR_ACCOUNT="${BASE_PILOT_MARKET_CREATOR_ACCOUNT:-${BASE_SMOKE_ADMIN_ACCOUNT:-${BASE_ADMIN_ACCOUNT:-}}}"

YES_TRADER_PRIVATE_KEY="${BASE_PILOT_YES_TRADER_PRIVATE_KEY:-${BASE_SMOKE_YES_TRADER_PRIVATE_KEY:-}}"
YES_TRADER_ACCOUNT="${BASE_PILOT_YES_TRADER_ACCOUNT:-${BASE_SMOKE_YES_TRADER_ACCOUNT:-}}"

NO_TRADER_PRIVATE_KEY="${BASE_PILOT_NO_TRADER_PRIVATE_KEY:-${BASE_SMOKE_NO_TRADER_PRIVATE_KEY:-}}"
NO_TRADER_ACCOUNT="${BASE_PILOT_NO_TRADER_ACCOUNT:-${BASE_SMOKE_NO_TRADER_ACCOUNT:-}}"

AGENT_OWNER_PRIVATE_KEY="${BASE_PILOT_AGENT_OWNER_PRIVATE_KEY:-}"
AGENT_OWNER_ACCOUNT="${BASE_PILOT_AGENT_OWNER_ACCOUNT:-}"

SIGNER_PASSWORD_FILE="${BASE_PILOT_PASSWORD_FILE:-${BASE_SMOKE_PASSWORD_FILE:-${BASE_KEYSTORE_PASSWORD_FILE:-$ROOT_DIR/keys/base-keystore-password.local}}}"
RESOLVER_ADDRESS="${BASE_MARKET_RESOLVER_ADDRESS:-}"

DRY_COUNTER=1

usage() {
  cat <<USAGE
Usage: scripts/base-seed-pilot.sh [options]

Seed Base pilot markets and reference agents for external OpenClaw E2E.

Options:
  --network mainnet|sepolia         Target chain (default: mainnet)
  --rpc-url <url>                   RPC URL override
  --fixture <path>                  Market fixture JSON (default: config/pilot/base-mainnet-markets.json)
  --count <n>                       Number of markets to seed (default: 20)
  --order-size <units>              Order size in collateral units (default: 100000 = 0.1 USDC, set 0 for markets-only seed)
  --yes-price-bps <bps>             YES order price bps (default: 5300)
  --no-price-bps <bps>              NO order price bps (default: 4700)
  --order-expiry-sec <seconds>      Order expiry delay (default: 2592000)
  --default-close-delay-sec <sec>   Market close delay when fixture omits override (default: 2592000)
  --agent-count <n>                 Number of reference agents (default: 3)
  --agent-size <units>              Agent order size (default: 50000)
  --agent-price-bps <bps>           Agent order price (default: 5100)
  --agent-cadence <seconds>         Agent cadence (default: 3600)
  --agent-expiry-window <seconds>   Agent expiry window (default: 7200)
  --report-out <path>               Output JSON report
  --dry-run                         Print tx calls without sending
  -h|--help                         Show this help

Env signer fallbacks:
  market creator:
    BASE_PILOT_MARKET_CREATOR_PRIVATE_KEY | BASE_PILOT_MARKET_CREATOR_ACCOUNT
    BASE_SMOKE_ADMIN_PRIVATE_KEY          | BASE_SMOKE_ADMIN_ACCOUNT
  yes trader:
    BASE_PILOT_YES_TRADER_PRIVATE_KEY     | BASE_PILOT_YES_TRADER_ACCOUNT
    BASE_SMOKE_YES_TRADER_PRIVATE_KEY     | BASE_SMOKE_YES_TRADER_ACCOUNT
  no trader:
    BASE_PILOT_NO_TRADER_PRIVATE_KEY      | BASE_PILOT_NO_TRADER_ACCOUNT
    BASE_SMOKE_NO_TRADER_PRIVATE_KEY      | BASE_SMOKE_NO_TRADER_ACCOUNT
  agent owner:
    BASE_PILOT_AGENT_OWNER_PRIVATE_KEY    | BASE_PILOT_AGENT_OWNER_ACCOUNT
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
    --fixture)
      FIXTURE="${2:-}"
      shift 2
      ;;
    --fixture=*)
      FIXTURE="${1#*=}"
      shift
      ;;
    --count)
      COUNT="${2:-}"
      shift 2
      ;;
    --count=*)
      COUNT="${1#*=}"
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
    --order-expiry-sec)
      ORDER_EXPIRY_SEC="${2:-}"
      shift 2
      ;;
    --order-expiry-sec=*)
      ORDER_EXPIRY_SEC="${1#*=}"
      shift
      ;;
    --default-close-delay-sec)
      DEFAULT_CLOSE_DELAY_SEC="${2:-}"
      shift 2
      ;;
    --default-close-delay-sec=*)
      DEFAULT_CLOSE_DELAY_SEC="${1#*=}"
      shift
      ;;
    --agent-count)
      AGENT_COUNT="${2:-}"
      shift 2
      ;;
    --agent-count=*)
      AGENT_COUNT="${1#*=}"
      shift
      ;;
    --agent-size)
      AGENT_SIZE="${2:-}"
      shift 2
      ;;
    --agent-size=*)
      AGENT_SIZE="${1#*=}"
      shift
      ;;
    --agent-price-bps)
      AGENT_PRICE_BPS="${2:-}"
      shift 2
      ;;
    --agent-price-bps=*)
      AGENT_PRICE_BPS="${1#*=}"
      shift
      ;;
    --agent-cadence)
      AGENT_CADENCE="${2:-}"
      shift 2
      ;;
    --agent-cadence=*)
      AGENT_CADENCE="${1#*=}"
      shift
      ;;
    --agent-expiry-window)
      AGENT_EXPIRY_WINDOW="${2:-}"
      shift 2
      ;;
    --agent-expiry-window=*)
      AGENT_EXPIRY_WINDOW="${1#*=}"
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

if [[ -z "$FIXTURE" ]]; then
  if [[ "$NETWORK" == "mainnet" ]]; then
    FIXTURE="$ROOT_DIR/config/pilot/base-mainnet-markets.json"
  else
    FIXTURE="$ROOT_DIR/config/pilot/base-mainnet-markets.json"
  fi
fi

require_bin() {
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

require_positive_int() {
  local value="$1"
  local label="$2"
  if ! [[ "$value" =~ ^[0-9]+$ ]] || [[ "$value" -lt 1 ]]; then
    echo "$label must be a positive integer" >&2
    exit 1
  fi
}

require_non_negative_int() {
  local value="$1"
  local label="$2"
  if ! [[ "$value" =~ ^[0-9]+$ ]]; then
    echo "$label must be a non-negative integer" >&2
    exit 1
  fi
}

address_or_fail() {
  local value="$1"
  local label="$2"
  if [[ ! "$value" =~ ^0x[0-9a-fA-F]{40}$ ]]; then
    echo "$label missing or invalid: $value" >&2
    exit 1
  fi
}

require_password_file() {
  if [[ ! -f "$SIGNER_PASSWORD_FILE" ]]; then
    echo "password file not found: $SIGNER_PASSWORD_FILE" >&2
    exit 1
  fi
}

signer_address() {
  local private_key="$1"
  local account="$2"
  local fallback_label="${3:-generic}"
  if [[ -n "$private_key" ]]; then
    cast wallet address --private-key "$private_key"
    return
  fi
  if [[ -n "$account" ]]; then
    require_password_file
    cast wallet address --account "$account" --password-file "$SIGNER_PASSWORD_FILE"
    return
  fi
  if [[ "$DRY_RUN" -eq 1 ]]; then
    case "$fallback_label" in
      creator) echo "0x00000000000000000000000000000000000000a1" ;;
      yes) echo "0x00000000000000000000000000000000000000b1" ;;
      no) echo "0x00000000000000000000000000000000000000c1" ;;
      agent) echo "0x00000000000000000000000000000000000000d1" ;;
      *) echo "0x00000000000000000000000000000000000000f1" ;;
    esac
    return
  fi
  echo "missing signer" >&2
  exit 1
}

to_dec() {
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
  shift 2
  local raw
  raw="$(cast call --rpc-url "$RPC_URL" "$addr" "$sig" "$@" 2>/dev/null || echo "0")"
  to_dec "$raw"
}

fake_hash() {
  printf '0x%064x' "$DRY_COUNTER"
  DRY_COUNTER=$((DRY_COUNTER + 1))
}

tx_send() {
  local private_key="$1"
  local account="$2"
  shift 2

  local cmd=(cast send --rpc-url "$RPC_URL")
  local dry_cmd=(cast send --rpc-url "$RPC_URL")
  local has_signer=1

  if [[ -n "$private_key" ]]; then
    cmd+=(--private-key "$private_key")
    dry_cmd+=(--private-key "<redacted>")
  elif [[ -n "$account" ]]; then
    require_password_file
    cmd+=(--account "$account" --password-file "$SIGNER_PASSWORD_FILE")
    dry_cmd+=(--account "$account" --password-file "$SIGNER_PASSWORD_FILE")
  else
    has_signer=0
    dry_cmd+=(--from "<unset-signer>")
  fi

  cmd+=("$@")
  dry_cmd+=("$@")

  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "dry-run: ${dry_cmd[*]}" >&2
    fake_hash
    return 0
  fi

  if [[ "$has_signer" -eq 0 ]]; then
    echo "missing signer for transaction" >&2
    exit 1
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

resolve_contracts_from_manifest() {
  local manifest="$ROOT_DIR/config/deployments/base-addresses.json"
  if [[ ! -f "$manifest" ]]; then
    return 0
  fi

  local env_key="production"
  if [[ "$NETWORK" == "sepolia" ]]; then
    env_key="staging"
  fi

  if [[ -z "$MARKET_CORE_ADDRESS" ]]; then
    MARKET_CORE_ADDRESS="$(jq -r ".environments.${env_key}.contracts.marketCore // empty" "$manifest")"
  fi
  if [[ -z "$ORDER_BOOK_ADDRESS" ]]; then
    ORDER_BOOK_ADDRESS="$(jq -r ".environments.${env_key}.contracts.orderBook // empty" "$manifest")"
  fi
  if [[ -z "$COLLATERAL_VAULT_ADDRESS" ]]; then
    COLLATERAL_VAULT_ADDRESS="$(jq -r ".environments.${env_key}.contracts.collateralVault // empty" "$manifest")"
  fi
  if [[ -z "$AGENT_RUNTIME_ADDRESS" ]]; then
    AGENT_RUNTIME_ADDRESS="$(jq -r ".environments.${env_key}.contracts.agentRuntime // empty" "$manifest")"
  fi
  if [[ -z "$COLLATERAL_TOKEN_ADDRESS" ]]; then
    COLLATERAL_TOKEN_ADDRESS="$(jq -r ".environments.${env_key}.contracts.collateralToken // empty" "$manifest")"
  fi
}

seed_vault_for_signer() {
  local label="$1"
  local addr="$2"
  local private_key="$3"
  local account="$4"
  local required_amount="$5"
  local funding_file="$6"

  if (( required_amount <= 0 )); then
    return 0
  fi

  local available
  available="$(call_uint "$COLLATERAL_VAULT_ADDRESS" "availableBalance(address)(uint256)" "$addr")"
  if [[ -z "$available" ]]; then
    available="0"
  fi

  local to_deposit=0
  if (( available < required_amount )); then
    to_deposit=$((required_amount - available))
  fi

  local approve_tx=""
  local deposit_tx=""
  if (( to_deposit > 0 )); then
    approve_tx="$(tx_send "$private_key" "$account" "$COLLATERAL_TOKEN_ADDRESS" "approve(address,uint256)" "$COLLATERAL_VAULT_ADDRESS" "$to_deposit")"
    deposit_tx="$(tx_send "$private_key" "$account" "$COLLATERAL_VAULT_ADDRESS" "deposit(uint256)" "$to_deposit")"
  fi

  jq -nc \
    --arg label "$label" \
    --arg wallet "$addr" \
    --argjson requiredAmount "$required_amount" \
    --argjson availableBefore "$available" \
    --argjson depositAmount "$to_deposit" \
    --arg approveTx "$approve_tx" \
    --arg depositTx "$deposit_tx" \
    '{
      label: $label,
      wallet: $wallet,
      requiredAmount: $requiredAmount,
      availableBefore: $availableBefore,
      depositAmount: $depositAmount,
      txs: {
        approve: (if $approveTx == "" then null else $approveTx end),
        deposit: (if $depositTx == "" then null else $depositTx end)
      }
    }' >>"$funding_file"
}

require_bin
require_positive_int "$COUNT" "--count"
require_non_negative_int "$ORDER_SIZE" "--order-size"
require_positive_int "$YES_PRICE_BPS" "--yes-price-bps"
require_positive_int "$NO_PRICE_BPS" "--no-price-bps"
require_positive_int "$ORDER_EXPIRY_SEC" "--order-expiry-sec"
require_positive_int "$DEFAULT_CLOSE_DELAY_SEC" "--default-close-delay-sec"
require_non_negative_int "$AGENT_COUNT" "--agent-count"
require_positive_int "$AGENT_SIZE" "--agent-size"
require_positive_int "$AGENT_PRICE_BPS" "--agent-price-bps"
require_positive_int "$AGENT_CADENCE" "--agent-cadence"
require_positive_int "$AGENT_EXPIRY_WINDOW" "--agent-expiry-window"

if (( YES_PRICE_BPS < 1 || YES_PRICE_BPS > 9999 || NO_PRICE_BPS < 1 || NO_PRICE_BPS > 9999 )); then
  echo "order prices must be in range 1..9999" >&2
  exit 1
fi
if (( AGENT_PRICE_BPS < 1 || AGENT_PRICE_BPS > 9999 )); then
  echo "agent price must be in range 1..9999" >&2
  exit 1
fi
if (( YES_PRICE_BPS + NO_PRICE_BPS < 10000 )); then
  echo "yes/no prices must cross (sum >= 10000)" >&2
  exit 1
fi
if [[ ! -f "$FIXTURE" ]]; then
  echo "fixture file not found: $FIXTURE" >&2
  exit 1
fi

resolve_contracts_from_manifest

if [[ -z "$COLLATERAL_TOKEN_ADDRESS" && "$NETWORK" == "mainnet" ]]; then
  COLLATERAL_TOKEN_ADDRESS="0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
fi

address_or_fail "$MARKET_CORE_ADDRESS" "MARKET_CORE_ADDRESS"
address_or_fail "$ORDER_BOOK_ADDRESS" "ORDER_BOOK_ADDRESS"
address_or_fail "$COLLATERAL_VAULT_ADDRESS" "COLLATERAL_VAULT_ADDRESS"
address_or_fail "$COLLATERAL_TOKEN_ADDRESS" "COLLATERAL_TOKEN_ADDRESS"
address_or_fail "$AGENT_RUNTIME_ADDRESS" "AGENT_RUNTIME_ADDRESS"

if [[ -z "$NO_TRADER_PRIVATE_KEY" && -z "$NO_TRADER_ACCOUNT" ]]; then
  NO_TRADER_PRIVATE_KEY="$YES_TRADER_PRIVATE_KEY"
  NO_TRADER_ACCOUNT="$YES_TRADER_ACCOUNT"
fi
if [[ -z "$AGENT_OWNER_PRIVATE_KEY" && -z "$AGENT_OWNER_ACCOUNT" ]]; then
  AGENT_OWNER_PRIVATE_KEY="$YES_TRADER_PRIVATE_KEY"
  AGENT_OWNER_ACCOUNT="$YES_TRADER_ACCOUNT"
fi
if (( ORDER_SIZE == 0 )) && [[ -z "$YES_TRADER_PRIVATE_KEY" && -z "$YES_TRADER_ACCOUNT" ]]; then
  YES_TRADER_PRIVATE_KEY="$CREATOR_PRIVATE_KEY"
  YES_TRADER_ACCOUNT="$CREATOR_ACCOUNT"
fi
if (( ORDER_SIZE == 0 )) && [[ -z "$NO_TRADER_PRIVATE_KEY" && -z "$NO_TRADER_ACCOUNT" ]]; then
  NO_TRADER_PRIVATE_KEY="$CREATOR_PRIVATE_KEY"
  NO_TRADER_ACCOUNT="$CREATOR_ACCOUNT"
fi
if (( AGENT_COUNT == 0 )) && [[ -z "$AGENT_OWNER_PRIVATE_KEY" && -z "$AGENT_OWNER_ACCOUNT" ]]; then
  AGENT_OWNER_PRIVATE_KEY="$CREATOR_PRIVATE_KEY"
  AGENT_OWNER_ACCOUNT="$CREATOR_ACCOUNT"
fi

if [[ "$DRY_RUN" -eq 0 ]]; then
  if [[ -z "$CREATOR_PRIVATE_KEY" && -z "$CREATOR_ACCOUNT" ]]; then
    echo "missing market creator signer" >&2
    exit 1
  fi
  if (( ORDER_SIZE > 0 )) && [[ -z "$YES_TRADER_PRIVATE_KEY" && -z "$YES_TRADER_ACCOUNT" ]]; then
    echo "missing YES trader signer" >&2
    exit 1
  fi
  if (( ORDER_SIZE > 0 )) && [[ -z "$NO_TRADER_PRIVATE_KEY" && -z "$NO_TRADER_ACCOUNT" ]]; then
    echo "missing NO trader signer" >&2
    exit 1
  fi
  if (( AGENT_COUNT > 0 )) && [[ -z "$AGENT_OWNER_PRIVATE_KEY" && -z "$AGENT_OWNER_ACCOUNT" ]]; then
    echo "missing agent owner signer" >&2
    exit 1
  fi
fi

CREATOR_ADDR="$(signer_address "$CREATOR_PRIVATE_KEY" "$CREATOR_ACCOUNT" creator | tr '[:upper:]' '[:lower:]')"
YES_ADDR="$(signer_address "$YES_TRADER_PRIVATE_KEY" "$YES_TRADER_ACCOUNT" yes | tr '[:upper:]' '[:lower:]')"
NO_ADDR="$(signer_address "$NO_TRADER_PRIVATE_KEY" "$NO_TRADER_ACCOUNT" no | tr '[:upper:]' '[:lower:]')"
AGENT_OWNER_ADDR="$(signer_address "$AGENT_OWNER_PRIVATE_KEY" "$AGENT_OWNER_ACCOUNT" agent | tr '[:upper:]' '[:lower:]')"

if [[ -z "$RESOLVER_ADDRESS" ]]; then
  RESOLVER_ADDRESS="$CREATOR_ADDR"
fi
RESOLVER_ADDRESS="$(echo "$RESOLVER_ADDRESS" | tr '[:upper:]' '[:lower:]')"
address_or_fail "$RESOLVER_ADDRESS" "BASE_MARKET_RESOLVER_ADDRESS"

if [[ -z "$REPORT_OUT" ]]; then
  REPORT_OUT="$ROOT_DIR/docs/reports/base-${NETWORK}-pilot-seed.json"
fi
mkdir -p "$(dirname "$REPORT_OUT")"

MARKET_TOTAL="$(jq '.markets | length' "$FIXTURE")"
if (( MARKET_TOTAL <= 0 )); then
  echo "fixture has no markets" >&2
  exit 1
fi

MARKET_COUNT="$COUNT"
if (( MARKET_COUNT > MARKET_TOTAL )); then
  MARKET_COUNT="$MARKET_TOTAL"
fi

AGENTS_TO_CREATE="$AGENT_COUNT"
if (( AGENTS_TO_CREATE > MARKET_COUNT )); then
  AGENTS_TO_CREATE="$MARKET_COUNT"
fi

echo "pilot seed start"
echo "network=$NETWORK"
echo "rpc_url=$RPC_URL"
echo "dry_run=$DRY_RUN"
echo "fixture=$FIXTURE"
echo "markets_to_seed=$MARKET_COUNT"
echo "agents_to_seed=$AGENTS_TO_CREATE"
echo "market_core=$MARKET_CORE_ADDRESS"
echo "order_book=$ORDER_BOOK_ADDRESS"
echo "collateral_vault=$COLLATERAL_VAULT_ADDRESS"
echo "collateral_token=$COLLATERAL_TOKEN_ADDRESS"
echo "agent_runtime=$AGENT_RUNTIME_ADDRESS"
echo "creator=$CREATOR_ADDR"
echo "resolver=$RESOLVER_ADDRESS"
echo "yes_trader=$YES_ADDR"
echo "no_trader=$NO_ADDR"
echo "agent_owner=$AGENT_OWNER_ADDR"

if [[ "$DRY_RUN" -eq 0 ]]; then
  MARKET_CREATOR_ROLE="$(cast call --rpc-url "$RPC_URL" "$MARKET_CORE_ADDRESS" "MARKET_CREATOR_ROLE()(bytes32)")"
  if [[ "$(cast call --rpc-url "$RPC_URL" "$MARKET_CORE_ADDRESS" "hasRole(bytes32,address)(bool)" "$MARKET_CREATOR_ROLE" "$CREATOR_ADDR" | tr '[:upper:]' '[:lower:]')" != "true" ]]; then
    echo "creator wallet missing MARKET_CREATOR_ROLE" >&2
    exit 1
  fi
fi

yes_required=$((ORDER_SIZE * MARKET_COUNT))
no_required=$((ORDER_SIZE * MARKET_COUNT))
agent_required=$((AGENT_SIZE * AGENTS_TO_CREATE))

yes_total_required=0
no_total_required=0
agent_total_required=0

if [[ "$YES_ADDR" == "$NO_ADDR" ]]; then
  yes_total_required=$((yes_total_required + yes_required + no_required))
else
  yes_total_required=$((yes_total_required + yes_required))
  no_total_required=$((no_total_required + no_required))
fi

if [[ "$AGENT_OWNER_ADDR" == "$YES_ADDR" ]]; then
  yes_total_required=$((yes_total_required + agent_required))
elif [[ "$AGENT_OWNER_ADDR" == "$NO_ADDR" ]]; then
  no_total_required=$((no_total_required + agent_required))
else
  agent_total_required=$((agent_total_required + agent_required))
fi

funding_file="$(mktemp)"
market_file="$(mktemp)"
agent_file="$(mktemp)"
trap 'rm -f "$funding_file" "$market_file" "$agent_file"' EXIT

seed_vault_for_signer "yes_trader" "$YES_ADDR" "$YES_TRADER_PRIVATE_KEY" "$YES_TRADER_ACCOUNT" "$yes_total_required" "$funding_file"
if [[ "$NO_ADDR" != "$YES_ADDR" ]]; then
  seed_vault_for_signer "no_trader" "$NO_ADDR" "$NO_TRADER_PRIVATE_KEY" "$NO_TRADER_ACCOUNT" "$no_total_required" "$funding_file"
fi
if [[ "$AGENT_OWNER_ADDR" != "$YES_ADDR" && "$AGENT_OWNER_ADDR" != "$NO_ADDR" ]]; then
  seed_vault_for_signer "agent_owner" "$AGENT_OWNER_ADDR" "$AGENT_OWNER_PRIVATE_KEY" "$AGENT_OWNER_ACCOUNT" "$agent_total_required" "$funding_file"
fi

current_market_count="$(call_uint "$MARKET_CORE_ADDRESS" "marketCount()(uint256)")"
if [[ -z "$current_market_count" ]]; then
  current_market_count="0"
fi
current_order_count="$(call_uint "$ORDER_BOOK_ADDRESS" "orderCount()(uint256)")"
if [[ -z "$current_order_count" ]]; then
  current_order_count="0"
fi
current_agent_count="$(call_uint "$AGENT_RUNTIME_ADDRESS" "agentCount()(uint256)")"
if [[ -z "$current_agent_count" ]]; then
  current_agent_count="0"
fi

now_ts="$(date +%s)"

for ((i=0; i<MARKET_COUNT; i++)); do
  question="$(jq -r ".markets[$i].question" "$FIXTURE")"
  description="$(jq -r ".markets[$i].description // \"\"" "$FIXTURE")"
  category="$(jq -r ".markets[$i].category // \"agentic\"" "$FIXTURE")"
  resolution_source="$(jq -r ".markets[$i].resolutionSource // \"public reports\"" "$FIXTURE")"
  close_delay="$(jq -r ".markets[$i].closeDelaySec // .defaultCloseDelaySec // $DEFAULT_CLOSE_DELAY_SEC" "$FIXTURE")"

  if ! [[ "$close_delay" =~ ^[0-9]+$ ]]; then
    close_delay="$DEFAULT_CLOSE_DELAY_SEC"
  fi

  close_time=$((now_ts + close_delay))
  expiry_time=$((now_ts + ORDER_EXPIRY_SEC))

  create_tx="$(tx_send "$CREATOR_PRIVATE_KEY" "$CREATOR_ACCOUNT" "$MARKET_CORE_ADDRESS" \
    "createMarketRich(string,string,string,string,uint64,address)" \
    "$question" "$description" "$category" "$resolution_source" "$close_time" "$RESOLVER_ADDRESS")"

  if [[ "$DRY_RUN" -eq 1 ]]; then
    current_market_count=$((current_market_count + 1))
  else
    current_market_count="$(call_uint "$MARKET_CORE_ADDRESS" "marketCount()(uint256)")"
  fi
  market_id="$current_market_count"

  place_yes_tx=""
  place_no_tx=""
  yes_order_id="null"
  no_order_id="null"
  if (( ORDER_SIZE > 0 )); then
    place_yes_tx="$(tx_send "$YES_TRADER_PRIVATE_KEY" "$YES_TRADER_ACCOUNT" "$ORDER_BOOK_ADDRESS" \
      "placeOrder(uint256,bool,uint128,uint128,uint64)" \
      "$market_id" "true" "$YES_PRICE_BPS" "$ORDER_SIZE" "$expiry_time")"

    if [[ "$DRY_RUN" -eq 1 ]]; then
      current_order_count=$((current_order_count + 1))
    else
      current_order_count="$(call_uint "$ORDER_BOOK_ADDRESS" "orderCount()(uint256)")"
    fi
    yes_order_id="$current_order_count"

    place_no_tx="$(tx_send "$NO_TRADER_PRIVATE_KEY" "$NO_TRADER_ACCOUNT" "$ORDER_BOOK_ADDRESS" \
      "placeOrder(uint256,bool,uint128,uint128,uint64)" \
      "$market_id" "false" "$NO_PRICE_BPS" "$ORDER_SIZE" "$expiry_time")"

    if [[ "$DRY_RUN" -eq 1 ]]; then
      current_order_count=$((current_order_count + 1))
    else
      current_order_count="$(call_uint "$ORDER_BOOK_ADDRESS" "orderCount()(uint256)")"
    fi
    no_order_id="$current_order_count"
  fi

  jq -nc \
    --argjson index "$((i + 1))" \
    --argjson marketId "$market_id" \
    --arg question "$question" \
    --arg category "$category" \
    --argjson closeTime "$close_time" \
    --argjson expiryTime "$expiry_time" \
    --arg createTx "$create_tx" \
    --arg placeYesTx "$place_yes_tx" \
    --argjson yesOrderId "$yes_order_id" \
    --arg placeNoTx "$place_no_tx" \
    --argjson noOrderId "$no_order_id" \
    '{
      index: $index,
      marketId: $marketId,
      question: $question,
      category: $category,
      closeTime: $closeTime,
      expiryTime: $expiryTime,
      txs: {
        createMarket: $createTx,
        placeYes: $placeYesTx,
        placeNo: $placeNoTx
      },
      orders: {
        yesOrderId: $yesOrderId,
        noOrderId: $noOrderId
      }
    }' >>"$market_file"

  if (( i < AGENTS_TO_CREATE )); then
    strategy="pilot_ref_agent_market_${market_id}"
    agent_is_yes="true"
    if (( i % 2 == 1 )); then
      agent_is_yes="false"
    fi

    create_agent_tx="$(tx_send "$AGENT_OWNER_PRIVATE_KEY" "$AGENT_OWNER_ACCOUNT" "$AGENT_RUNTIME_ADDRESS" \
      "createAgent(uint256,bool,uint128,uint128,uint64,uint64,string)" \
      "$market_id" "$agent_is_yes" "$AGENT_PRICE_BPS" "$AGENT_SIZE" "$AGENT_CADENCE" "$AGENT_EXPIRY_WINDOW" "$strategy")"

    if [[ "$DRY_RUN" -eq 1 ]]; then
      current_agent_count=$((current_agent_count + 1))
    else
      current_agent_count="$(call_uint "$AGENT_RUNTIME_ADDRESS" "agentCount()(uint256)")"
    fi

    jq -nc \
      --argjson index "$((i + 1))" \
      --argjson marketId "$market_id" \
      --argjson agentId "$current_agent_count" \
      --arg owner "$AGENT_OWNER_ADDR" \
      --arg strategy "$strategy" \
      --arg isYes "$agent_is_yes" \
      --arg txHash "$create_agent_tx" \
      '{
        index: $index,
        agentId: $agentId,
        marketId: $marketId,
        owner: $owner,
        isYes: ($isYes == "true"),
        strategy: $strategy,
        txHash: $txHash
      }' >>"$agent_file"
  fi

done

if [[ ! -s "$funding_file" ]]; then
  echo "[]" >"$funding_file"
fi
if [[ ! -s "$market_file" ]]; then
  echo "[]" >"$market_file"
fi
if [[ ! -s "$agent_file" ]]; then
  echo "[]" >"$agent_file"
fi

jq -s '.' "$funding_file" >"$funding_file.array"
jq -s '.' "$market_file" >"$market_file.array"
jq -s '.' "$agent_file" >"$agent_file.array"

jq -n \
  --arg generatedAt "$(date -u +"%Y-%m-%dT%H:%M:%SZ")" \
  --arg network "$NETWORK" \
  --arg rpcUrl "$RPC_URL" \
  --arg fixture "$FIXTURE" \
  --argjson dryRun "$([[ "$DRY_RUN" -eq 1 ]] && echo true || echo false)" \
  --arg marketCore "$MARKET_CORE_ADDRESS" \
  --arg orderBook "$ORDER_BOOK_ADDRESS" \
  --arg collateralVault "$COLLATERAL_VAULT_ADDRESS" \
  --arg collateralToken "$COLLATERAL_TOKEN_ADDRESS" \
  --arg agentRuntime "$AGENT_RUNTIME_ADDRESS" \
  --arg creator "$CREATOR_ADDR" \
  --arg resolver "$RESOLVER_ADDRESS" \
  --arg yesTrader "$YES_ADDR" \
  --arg noTrader "$NO_ADDR" \
  --arg agentOwner "$AGENT_OWNER_ADDR" \
  --argjson marketCount "$MARKET_COUNT" \
  --argjson agentCount "$AGENTS_TO_CREATE" \
  --argjson orderSize "$ORDER_SIZE" \
  --argjson yesPriceBps "$YES_PRICE_BPS" \
  --argjson noPriceBps "$NO_PRICE_BPS" \
  --argjson agentSize "$AGENT_SIZE" \
  --argjson agentPriceBps "$AGENT_PRICE_BPS" \
  --argjson agentCadence "$AGENT_CADENCE" \
  --argjson agentExpiryWindow "$AGENT_EXPIRY_WINDOW" \
  --slurpfile funding "$funding_file.array" \
  --slurpfile markets "$market_file.array" \
  --slurpfile agents "$agent_file.array" \
  '{
    generatedAt: $generatedAt,
    network: $network,
    rpcUrl: $rpcUrl,
    fixture: $fixture,
    dryRun: $dryRun,
    contracts: {
      marketCore: $marketCore,
      orderBook: $orderBook,
      collateralVault: $collateralVault,
      collateralToken: $collateralToken,
      agentRuntime: $agentRuntime
    },
    actors: {
      creator: $creator,
      resolver: $resolver,
      yesTrader: $yesTrader,
      noTrader: $noTrader,
      agentOwner: $agentOwner
    },
    config: {
      marketCount: $marketCount,
      agentCount: $agentCount,
      orderSize: $orderSize,
      yesPriceBps: $yesPriceBps,
      noPriceBps: $noPriceBps,
      agentSize: $agentSize,
      agentPriceBps: $agentPriceBps,
      agentCadence: $agentCadence,
      agentExpiryWindow: $agentExpiryWindow
    },
    funding: $funding[0],
    markets: $markets[0],
    agents: $agents[0]
  }' >"$REPORT_OUT"

rm -f "$funding_file.array" "$market_file.array" "$agent_file.array"

echo "pilot seed complete"
echo "report: $REPORT_OUT"
