#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

NETWORK="mainnet"
RPC_URL=""
API_URL="${API_URL:-http://127.0.0.1:8080/v1}"
POLL_INTERVAL_SEC="${AUTO_CLAIMER_POLL_INTERVAL_SEC:-30}"
CANDIDATE_SCAN_LIMIT="${AUTO_CLAIMER_CANDIDATE_SCAN_LIMIT:-1000}"
AGENT_SCAN_LIMIT="${AUTO_CLAIMER_AGENT_SCAN_LIMIT:-1000}"
ORDER_SCAN_WINDOW="${AUTO_CLAIMER_ORDER_SCAN_WINDOW:-2500}"
MAX_CLAIMS_PER_CYCLE="${AUTO_CLAIMER_MAX_CLAIMS_PER_CYCLE:-200}"
RECEIPT_POLL_SEC="2"
RECEIPT_MAX_POLLS="45"
AUTO_WITHDRAW=0
AUTO_WITHDRAW_SET=0
DRY_RUN=0
ONCE=0

ORDER_BOOK_ADDRESS="${ORDER_BOOK_ADDRESS:-${NEXT_PUBLIC_ORDER_BOOK_ADDRESS:-}}"
COLLATERAL_VAULT_ADDRESS="${COLLATERAL_VAULT_ADDRESS:-${NEXT_PUBLIC_COLLATERAL_VAULT_ADDRESS:-}}"
CLAIMER_PRIVATE_KEY="${AUTO_CLAIMER_PRIVATE_KEY:-${BASE_GLOBAL_CLAIMER_PRIVATE_KEY:-${BASE_AGENT_RUNTIME_OPERATOR_PRIVATE_KEY:-}}}"
OWNER_KEYS_FILE="${AUTO_CLAIMER_OWNER_KEYS_FILE:-}"
OWNER_KEYS_JSON="${AUTO_CLAIMER_OWNER_KEYS_JSON:-}"
OWNER_KEYS_JSON_RESOLVED="{}"
ADMIN_CONTROL_KEY="${ADMIN_CONTROL_KEY:-}"

ORDER_BOOK_CLAIM_FOR_SELECTOR="0x0de05659"
COLLATERAL_WITHDRAW_SELECTOR="0x2e1a7d4d"
ORDER_BOOK_ORDERS_SELECTOR="0xa85c38ef"
ORDER_BOOK_CLAIMED_TOPIC="0x93c1c30a0fa404e7a08a9f6a9d68323786a7e120f3adc0c16eb8855922e35dfa"

usage() {
  cat <<USAGE
Usage: scripts/base-auto-claimer.sh [options]

Continuously scans resolved markets and submits claimFor transactions for users/agent owners.
Supports optional auto-withdraw to owner wallets when owner private keys are provided.

Options:
  --network mainnet|sepolia         Target chain (default: mainnet)
  --rpc-url <url>                   RPC URL override
  --api-url <url>                   API base URL (default: http://127.0.0.1:8080/v1)
  --poll-interval-sec <seconds>     Poll interval (default: 30)
  --candidate-scan-limit <count>    Candidate API page size (default: 1000)
  --agent-scan-limit <count>        Agent scan page size (default: 1000)
  --order-scan-window <count>       Number of latest orders to scan for owner candidates (default: 2500)
  --max-claims-per-cycle <count>    Claim tx send cap per cycle, 0 = unlimited (default: 200)
  --auto-withdraw true|false        Auto-withdraw claimed vault balance for managed owners (default: false)
  --owner-keys-file <path>          JSON map of owner->private_key for managed auto-withdraw
  --owner-keys-json <json>          Inline JSON map of owner->private_key for managed auto-withdraw
  --dry-run                         Print actions only
  --once                            Run one cycle and exit
  -h|--help                         Show help

Required env for live claims:
  ORDER_BOOK_ADDRESS
  AUTO_CLAIMER_PRIVATE_KEY (or BASE_GLOBAL_CLAIMER_PRIVATE_KEY / BASE_AGENT_RUNTIME_OPERATOR_PRIVATE_KEY)

Optional env for managed withdraw:
  COLLATERAL_VAULT_ADDRESS
  AUTO_CLAIMER_OWNER_KEYS_FILE / AUTO_CLAIMER_OWNER_KEYS_JSON
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
    --poll-interval-sec)
      POLL_INTERVAL_SEC="${2:-}"
      shift 2
      ;;
    --poll-interval-sec=*)
      POLL_INTERVAL_SEC="${1#*=}"
      shift
      ;;
    --candidate-scan-limit)
      CANDIDATE_SCAN_LIMIT="${2:-}"
      shift 2
      ;;
    --candidate-scan-limit=*)
      CANDIDATE_SCAN_LIMIT="${1#*=}"
      shift
      ;;
    --agent-scan-limit)
      AGENT_SCAN_LIMIT="${2:-}"
      shift 2
      ;;
    --agent-scan-limit=*)
      AGENT_SCAN_LIMIT="${1#*=}"
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
      case "${2:-}" in
        true) AUTO_WITHDRAW=1 ;;
        false) AUTO_WITHDRAW=0 ;;
        *)
          echo "--auto-withdraw must be true or false" >&2
          exit 1
          ;;
      esac
      AUTO_WITHDRAW_SET=1
      shift 2
      ;;
    --auto-withdraw=*)
      case "${1#*=}" in
        true) AUTO_WITHDRAW=1 ;;
        false) AUTO_WITHDRAW=0 ;;
        *)
          echo "--auto-withdraw must be true or false" >&2
          exit 1
          ;;
      esac
      AUTO_WITHDRAW_SET=1
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
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    --once)
      ONCE=1
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

if [[ "$AUTO_WITHDRAW_SET" -eq 0 && -n "${AUTO_CLAIMER_AUTO_WITHDRAW:-}" ]]; then
  case "${AUTO_CLAIMER_AUTO_WITHDRAW}" in
    true) AUTO_WITHDRAW=1 ;;
    false) AUTO_WITHDRAW=0 ;;
    *)
      echo "AUTO_CLAIMER_AUTO_WITHDRAW must be true or false" >&2
      exit 1
      ;;
  esac
fi

if [[ "$NETWORK" != "mainnet" && "$NETWORK" != "sepolia" ]]; then
  echo "--network must be mainnet or sepolia" >&2
  exit 1
fi

require_positive_int() {
  local value="$1"
  local name="$2"
  if ! [[ "$value" =~ ^[0-9]+$ ]] || [[ "$value" -lt 1 ]]; then
    echo "$name must be a positive integer" >&2
    exit 1
  fi
}

require_non_negative_int() {
  local value="$1"
  local name="$2"
  if ! [[ "$value" =~ ^[0-9]+$ ]]; then
    echo "$name must be a non-negative integer" >&2
    exit 1
  fi
}

require_positive_int "$POLL_INTERVAL_SEC" "--poll-interval-sec"
require_positive_int "$CANDIDATE_SCAN_LIMIT" "--candidate-scan-limit"
require_positive_int "$AGENT_SCAN_LIMIT" "--agent-scan-limit"
require_positive_int "$ORDER_SCAN_WINDOW" "--order-scan-window"
require_non_negative_int "$MAX_CLAIMS_PER_CYCLE" "--max-claims-per-cycle"
require_positive_int "$RECEIPT_POLL_SEC" "RECEIPT_POLL_SEC"
require_positive_int "$RECEIPT_MAX_POLLS" "RECEIPT_MAX_POLLS"

if [[ -z "$RPC_URL" ]]; then
  if [[ "$NETWORK" == "sepolia" ]]; then
    RPC_URL="${BASE_SEPOLIA_RPC_URL:-https://sepolia.base.org}"
  else
    RPC_URL="${BASE_RPC_URL:-https://mainnet.base.org}"
  fi
fi

if [[ "$API_URL" == */v1 ]]; then
  API_URL="${API_URL%/}"
else
  API_URL="${API_URL%/}/v1"
fi

ORDER_BOOK_ADDRESS="${ORDER_BOOK_ADDRESS:-${NEXT_PUBLIC_ORDER_BOOK_ADDRESS:-}}"
COLLATERAL_VAULT_ADDRESS="${COLLATERAL_VAULT_ADDRESS:-${NEXT_PUBLIC_COLLATERAL_VAULT_ADDRESS:-}}"
CLAIMER_PRIVATE_KEY="${CLAIMER_PRIVATE_KEY:-${AUTO_CLAIMER_PRIVATE_KEY:-${BASE_GLOBAL_CLAIMER_PRIVATE_KEY:-${BASE_AGENT_RUNTIME_OPERATOR_PRIVATE_KEY:-}}}}"

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

normalize_addr() {
  echo "$1" | tr '[:upper:]' '[:lower:]'
}

is_evm_addr() {
  local value
  value="$(normalize_addr "$1")"
  [[ "$value" =~ ^0x[0-9a-f]{40}$ ]]
}

to_dec() {
  local value="$1"
  cast to-dec "$value" 2>/dev/null || echo "0"
}

extract_tx_hash() {
  local value="$1"
  echo "$value" | grep -Eo '0x[0-9a-fA-F]{64}' | tail -n 1
}

json_rpc() {
  local method="$1"
  local params="$2"
  cast rpc --rpc-url "$RPC_URL" "$method" "$params"
}

eth_call_data() {
  local to="$1"
  local data="$2"
  local params
  params="$(jq -nc --arg to "$to" --arg data "$data" '[{to:$to,data:$data},"latest"]')"
  json_rpc "eth_call" "$params" | jq -r '.result // "0x"'
}

load_owner_key_map() {
  local payload="{}"
  if [[ -n "$OWNER_KEYS_FILE" ]]; then
    if [[ ! -f "$OWNER_KEYS_FILE" ]]; then
      echo "owner keys file not found: $OWNER_KEYS_FILE" >&2
      exit 1
    fi
    payload="$(cat "$OWNER_KEYS_FILE")"
  elif [[ -n "$OWNER_KEYS_JSON" ]]; then
    payload="$OWNER_KEYS_JSON"
  fi

  OWNER_KEYS_JSON_RESOLVED="$(echo "$payload" | jq -c '
    if type != "object" then error("owner key map must be JSON object")
    else with_entries(.key |= ascii_downcase)
    end
  ' 2>/dev/null || true)"

  if [[ -z "$OWNER_KEYS_JSON_RESOLVED" ]]; then
    echo "invalid owner key map json" >&2
    exit 1
  fi
}

owner_private_key_for() {
  local owner
  owner="$(normalize_addr "$1")"
  echo "$OWNER_KEYS_JSON_RESOLVED" | jq -r --arg owner "$owner" '.[$owner] // ""'
}

wait_for_receipt() {
  local tx_hash="$1"
  local poll=1
  while [[ "$poll" -le "$RECEIPT_MAX_POLLS" ]]; do
    local params
    params="$(jq -nc --arg hash "$tx_hash" '[$hash]')"
    local response
    response="$(json_rpc "eth_getTransactionReceipt" "$params")"
    if [[ "$(echo "$response" | jq -r '.result != null')" == "true" ]]; then
      echo "$response"
      return 0
    fi
    sleep "$RECEIPT_POLL_SEC"
    poll=$((poll + 1))
  done
  return 1
}

validate_claim_tx() {
  local tx_hash="$1"
  local owner="$2"
  local market_id="$3"

  local owner_norm
  owner_norm="$(normalize_addr "$owner")"

  local tx_params tx_json receipt_json
  tx_params="$(jq -nc --arg hash "$tx_hash" '[$hash]')"
  tx_json="$(json_rpc "eth_getTransactionByHash" "$tx_params")"
  receipt_json="$(wait_for_receipt "$tx_hash")"

  local tx_to tx_input tx_value
  tx_to="$(echo "$tx_json" | jq -r '.result.to // ""' | tr '[:upper:]' '[:lower:]')"
  tx_input="$(echo "$tx_json" | jq -r '.result.input // ""' | tr '[:upper:]' '[:lower:]')"
  tx_value="$(echo "$tx_json" | jq -r '.result.value // "0x0"' | tr '[:upper:]' '[:lower:]')"

  if [[ "$tx_to" != "$(normalize_addr "$ORDER_BOOK_ADDRESS")" ]]; then
    echo "claim validation failed: tx target mismatch tx=$tx_hash to=$tx_to" >&2
    return 1
  fi
  if [[ "${tx_input:0:10}" != "$ORDER_BOOK_CLAIM_FOR_SELECTOR" ]]; then
    echo "claim validation failed: method selector mismatch tx=$tx_hash" >&2
    return 1
  fi
  if [[ "$(to_dec "$tx_value")" != "0" ]]; then
    echo "claim validation failed: non-zero native value tx=$tx_hash value=$tx_value" >&2
    return 1
  fi

  local input_no_prefix arg_owner_hex arg_market_hex arg_owner arg_market
  input_no_prefix="${tx_input#0x}"
  arg_owner_hex="${input_no_prefix:32:40}"
  arg_market_hex="${input_no_prefix:72:64}"
  arg_owner="0x${arg_owner_hex}"
  arg_market="$(to_dec "0x${arg_market_hex}")"

  if [[ "$(normalize_addr "$arg_owner")" != "$owner_norm" ]]; then
    echo "claim validation failed: owner arg mismatch tx=$tx_hash owner=$arg_owner expected=$owner" >&2
    return 1
  fi
  if [[ "$arg_market" != "$market_id" ]]; then
    echo "claim validation failed: market arg mismatch tx=$tx_hash market=$arg_market expected=$market_id" >&2
    return 1
  fi

  local market_topic owner_topic claim_log_found
  market_topic="$(printf '0x%064x' "$market_id")"
  owner_topic="0x000000000000000000000000${owner_norm#0x}"
  claim_log_found="$(echo "$receipt_json" | jq -r \
    --arg orderbook "$(normalize_addr "$ORDER_BOOK_ADDRESS")" \
    --arg topic0 "${ORDER_BOOK_CLAIMED_TOPIC,,}" \
    --arg topic1 "${market_topic,,}" \
    --arg topic2 "${owner_topic,,}" '
      any(.result.logs[]?;
        ((.address // "") | ascii_downcase) == $orderbook
        and ((.topics[0] // "") | ascii_downcase) == $topic0
        and ((.topics[1] // "") | ascii_downcase) == $topic1
        and ((.topics[2] // "") | ascii_downcase) == $topic2
      )
    ')"

  if [[ "$claim_log_found" != "true" ]]; then
    echo "claim validation failed: Claimed event not found tx=$tx_hash owner=$owner market=$market_id" >&2
    return 1
  fi

  local receipt_status
  receipt_status="$(echo "$receipt_json" | jq -r '.result.status // "0x0"')"
  if [[ "$(to_dec "$receipt_status")" != "1" ]]; then
    echo "claim validation failed: receipt status != 1 tx=$tx_hash status=$receipt_status" >&2
    return 1
  fi

  return 0
}

validate_withdraw_tx() {
  local tx_hash="$1"
  local owner="$2"
  local amount="$3"

  local owner_norm
  owner_norm="$(normalize_addr "$owner")"

  local tx_params tx_json receipt_json
  tx_params="$(jq -nc --arg hash "$tx_hash" '[$hash]')"
  tx_json="$(json_rpc "eth_getTransactionByHash" "$tx_params")"
  receipt_json="$(wait_for_receipt "$tx_hash")"

  local tx_to tx_from tx_input tx_value
  tx_to="$(echo "$tx_json" | jq -r '.result.to // ""' | tr '[:upper:]' '[:lower:]')"
  tx_from="$(echo "$tx_json" | jq -r '.result.from // ""' | tr '[:upper:]' '[:lower:]')"
  tx_input="$(echo "$tx_json" | jq -r '.result.input // ""' | tr '[:upper:]' '[:lower:]')"
  tx_value="$(echo "$tx_json" | jq -r '.result.value // "0x0"' | tr '[:upper:]' '[:lower:]')"

  if [[ "$tx_to" != "$(normalize_addr "$COLLATERAL_VAULT_ADDRESS")" ]]; then
    echo "withdraw validation failed: vault target mismatch tx=$tx_hash to=$tx_to" >&2
    return 1
  fi
  if [[ "$tx_from" != "$owner_norm" ]]; then
    echo "withdraw validation failed: sender mismatch tx=$tx_hash from=$tx_from expected=$owner_norm" >&2
    return 1
  fi
  if [[ "${tx_input:0:10}" != "$COLLATERAL_WITHDRAW_SELECTOR" ]]; then
    echo "withdraw validation failed: method selector mismatch tx=$tx_hash" >&2
    return 1
  fi
  if [[ "$(to_dec "$tx_value")" != "0" ]]; then
    echo "withdraw validation failed: non-zero native value tx=$tx_hash value=$tx_value" >&2
    return 1
  fi

  local input_no_prefix arg_amount_hex arg_amount
  input_no_prefix="${tx_input#0x}"
  arg_amount_hex="${input_no_prefix:8:64}"
  arg_amount="$(to_dec "0x${arg_amount_hex}")"
  if [[ "$arg_amount" != "$amount" ]]; then
    echo "withdraw validation failed: amount mismatch tx=$tx_hash amount=$arg_amount expected=$amount" >&2
    return 1
  fi

  local receipt_status
  receipt_status="$(echo "$receipt_json" | jq -r '.result.status // "0x0"')"
  if [[ "$(to_dec "$receipt_status")" != "1" ]]; then
    echo "withdraw validation failed: receipt status != 1 tx=$tx_hash status=$receipt_status" >&2
    return 1
  fi

  return 0
}

scan_order_candidates() {
  local candidate_file="$1"
  local count_raw count_dec start_id

  count_raw="$(cast call --rpc-url "$RPC_URL" "$ORDER_BOOK_ADDRESS" "orderCount()(uint256)" 2>/dev/null || echo "0")"
  count_raw="${count_raw//$'\n'/}"
  count_raw="${count_raw// /}"
  count_dec="$(to_dec "$count_raw")"

  if [[ "$count_dec" -le 0 ]]; then
    return 0
  fi

  if [[ "$count_dec" -gt "$ORDER_SCAN_WINDOW" ]]; then
    start_id=$((count_dec - ORDER_SCAN_WINDOW + 1))
  else
    start_id=1
  fi

  local id
  for ((id = count_dec; id >= start_id; id--)); do
    local data raw payload maker_slot market_slot maker market
    data="${ORDER_BOOK_ORDERS_SELECTOR}$(printf '%064x' "$id")"
    raw="$(eth_call_data "$ORDER_BOOK_ADDRESS" "$data")"
    payload="${raw#0x}"

    if [[ ${#payload} -lt 128 ]]; then
      continue
    fi

    maker_slot="${payload:0:64}"
    market_slot="${payload:64:64}"
    maker="0x${maker_slot:24:40}"
    market="$(to_dec "0x${market_slot}")"

    if [[ "$maker" == "0x0000000000000000000000000000000000000000" || "$market" -le 0 ]]; then
      continue
    fi

    if is_evm_addr "$maker"; then
      echo "$(normalize_addr "$maker") $market" >> "$candidate_file"
    fi
  done
}

scan_payout_candidates() {
  local candidate_file="$1"
  local payload
  if ! payload="$(curl -fsS "$API_URL/evm/payouts/candidates?limit=$CANDIDATE_SCAN_LIMIT")"; then
    echo "warning: failed to query payout candidates from $API_URL" >&2
    return 0
  fi

  echo "$payload" |
    jq -r '.candidates[]? | select(.owner != null and .market_id != null) | "\(.owner) \(.market_id)"' |
    while read -r owner market_id; do
      if [[ -z "$owner" || -z "$market_id" ]]; then
        continue
      fi
      if is_evm_addr "$owner" && [[ "$market_id" =~ ^[0-9]+$ ]]; then
        echo "$(normalize_addr "$owner") $market_id" >> "$candidate_file"
      fi
    done
}

seed_payout_jobs() {
  curl -fsS "$API_URL/evm/payouts/health" >/dev/null 2>&1 || true
}

report_payout_job() {
  local owner="$1"
  local market_id="$2"
  local status="$3"
  local last_tx="${4:-}"
  local last_error="${5:-}"
  local retry_after="${6:-}"

  if [[ -z "$ADMIN_CONTROL_KEY" ]]; then
    return 0
  fi

  local body
  body="$(jq -n \
    --arg owner "$owner" \
    --argjson marketId "$market_id" \
    --arg status "$status" \
    --arg lastTx "$last_tx" \
    --arg lastError "$last_error" \
    --arg retryAfter "$retry_after" \
    '{
      marketId: $marketId,
      wallet: $owner,
      status: $status,
      lastTx: (if $lastTx == "" then null else $lastTx end),
      lastError: (if $lastError == "" then null else $lastError end),
      retryAfterSeconds: (if $retryAfter == "" then null else ($retryAfter | tonumber) end)
    }')"

  curl -fsS \
    -X POST \
    -H "content-type: application/json" \
    -H "x-admin-key: $ADMIN_CONTROL_KEY" \
    -d "$body" \
    "$API_URL/evm/payouts/report" >/dev/null 2>&1 || true
}

scan_agent_candidates() {
  local candidate_file="$1"
  local payload
  if ! payload="$(curl -fsS "$API_URL/evm/agents?limit=$AGENT_SCAN_LIMIT&offset=0")"; then
    echo "warning: failed to query agents from $API_URL" >&2
    return 0
  fi

  echo "$payload" |
    jq -r '.agents[]? | select(.owner != null and .market_id != null) | "\(.owner) \(.market_id)"' |
    while read -r owner market_id; do
      if [[ -z "$owner" || -z "$market_id" ]]; then
        continue
      fi
      if is_evm_addr "$owner"; then
        echo "$(normalize_addr "$owner") $market_id" >> "$candidate_file"
      fi
    done
}

auto_withdraw_if_configured() {
  local owner="$1"
  local amount="$2"

  if [[ "$AUTO_WITHDRAW" -ne 1 ]]; then
    return 0
  fi
  if [[ -z "$COLLATERAL_VAULT_ADDRESS" ]]; then
    echo "skip auto-withdraw owner=$owner amount=$amount reason=missing_collateral_vault" >&2
    return 0
  fi

  local owner_key
  owner_key="$(owner_private_key_for "$owner")"
  if [[ -z "$owner_key" || "$owner_key" == "null" ]]; then
    echo "skip auto-withdraw owner=$owner amount=$amount reason=missing_owner_key" >&2
    return 0
  fi

  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "dry-run withdraw owner=$owner amount=$amount"
    return 0
  fi

  local withdraw_out tx_hash
  if ! withdraw_out="$(cast send --async --rpc-url "$RPC_URL" --private-key "$owner_key" "$COLLATERAL_VAULT_ADDRESS" "withdraw(uint256)" "$amount" 2>&1)"; then
    echo "failed auto-withdraw owner=$owner amount=$amount error=$withdraw_out" >&2
    return 1
  fi
  tx_hash="$(extract_tx_hash "$withdraw_out")"
  if [[ -z "$tx_hash" ]]; then
    echo "failed auto-withdraw owner=$owner amount=$amount error=missing_tx_hash output=$withdraw_out" >&2
    return 1
  fi

  if validate_withdraw_tx "$tx_hash" "$owner" "$amount"; then
    echo "withdrawn owner=$owner amount=$amount tx=$tx_hash"
    return 0
  fi

  echo "failed auto-withdraw owner=$owner amount=$amount tx=$tx_hash reason=validation_failed" >&2
  return 1
}

poll_and_claim() {
  local candidate_file unique_file
  candidate_file="$(mktemp)"
  unique_file="$(mktemp)"

  trap 'rm -f "$candidate_file" "$unique_file"' RETURN

  seed_payout_jobs
  scan_payout_candidates "$candidate_file"
  scan_agent_candidates "$candidate_file"
  scan_order_candidates "$candidate_file"

  if [[ ! -s "$candidate_file" ]]; then
    echo "no claim candidates ($(date -u +"%Y-%m-%dT%H:%M:%SZ"))"
    return 0
  fi

  sort -u "$candidate_file" > "$unique_file"

  local total_candidates claim_attempts claim_success claim_fail withdraw_success withdraw_fail
  total_candidates="$(wc -l < "$unique_file" | tr -d ' ')"
  claim_attempts=0
  claim_success=0
  claim_fail=0
  withdraw_success=0
  withdraw_fail=0

  echo "claimer cycle start candidates=$total_candidates ($(date -u +"%Y-%m-%dT%H:%M:%SZ"))"

  while read -r owner market_id; do
    if [[ -z "$owner" || -z "$market_id" ]]; then
      continue
    fi

    local claimable_raw claimable_dec
    claimable_raw="$(cast call --rpc-url "$RPC_URL" "$ORDER_BOOK_ADDRESS" "claimable(uint256,address)(uint256)" "$market_id" "$owner" 2>/dev/null || echo "0")"
    claimable_raw="${claimable_raw//$'\n'/}"
    claimable_raw="${claimable_raw// /}"
    claimable_dec="$(to_dec "$claimable_raw")"

    if [[ "$claimable_dec" -le 0 ]]; then
      continue
    fi

    if [[ "$MAX_CLAIMS_PER_CYCLE" -gt 0 && "$claim_attempts" -ge "$MAX_CLAIMS_PER_CYCLE" ]]; then
      echo "claim cap reached max_claims_per_cycle=$MAX_CLAIMS_PER_CYCLE"
      break
    fi

    claim_attempts=$((claim_attempts + 1))
    report_payout_job "$owner" "$market_id" "processing" "" "" ""

    if [[ "$DRY_RUN" -eq 1 ]]; then
      echo "dry-run claim owner=$owner market=$market_id claimable=$claimable_dec"
      auto_withdraw_if_configured "$owner" "$claimable_dec" || true
      report_payout_job "$owner" "$market_id" "pending" "" "" ""
      continue
    fi

    local claim_out tx_hash
    if ! claim_out="$(cast send --async --rpc-url "$RPC_URL" --private-key "$CLAIMER_PRIVATE_KEY" "$ORDER_BOOK_ADDRESS" "claimFor(address,uint256)" "$owner" "$market_id" 2>&1)"; then
      claim_fail=$((claim_fail + 1))
      echo "failed claim owner=$owner market=$market_id claimable=$claimable_dec error=$claim_out" >&2
      report_payout_job "$owner" "$market_id" "retry" "" "$claim_out" "30"
      continue
    fi

    tx_hash="$(extract_tx_hash "$claim_out")"
    if [[ -z "$tx_hash" ]]; then
      claim_fail=$((claim_fail + 1))
      echo "failed claim owner=$owner market=$market_id claimable=$claimable_dec error=missing_tx_hash output=$claim_out" >&2
      report_payout_job "$owner" "$market_id" "retry" "" "missing_tx_hash" "30"
      continue
    fi

    if validate_claim_tx "$tx_hash" "$owner" "$market_id"; then
      claim_success=$((claim_success + 1))
      echo "claimed owner=$owner market=$market_id claimable=$claimable_dec tx=$tx_hash"
      report_payout_job "$owner" "$market_id" "paid" "$tx_hash" "" ""
      if auto_withdraw_if_configured "$owner" "$claimable_dec"; then
        withdraw_success=$((withdraw_success + 1))
      else
        withdraw_fail=$((withdraw_fail + 1))
      fi
    else
      claim_fail=$((claim_fail + 1))
      echo "failed claim owner=$owner market=$market_id claimable=$claimable_dec tx=$tx_hash reason=validation_failed" >&2
      report_payout_job "$owner" "$market_id" "retry" "$tx_hash" "validation_failed" "30"
    fi
  done < "$unique_file"

  echo "claimer cycle result claims_attempted=$claim_attempts claims_ok=$claim_success claims_failed=$claim_fail withdraw_ok=$withdraw_success withdraw_failed=$withdraw_fail"
}

require_bin
load_owner_key_map

if [[ "$DRY_RUN" -eq 0 ]]; then
  if [[ -z "$ORDER_BOOK_ADDRESS" ]] || ! is_evm_addr "$ORDER_BOOK_ADDRESS"; then
    echo "ORDER_BOOK_ADDRESS must be set to a valid 0x address" >&2
    exit 1
  fi
  if [[ -z "$CLAIMER_PRIVATE_KEY" ]]; then
    echo "AUTO_CLAIMER_PRIVATE_KEY or BASE_GLOBAL_CLAIMER_PRIVATE_KEY or BASE_AGENT_RUNTIME_OPERATOR_PRIVATE_KEY is required" >&2
    exit 1
  fi
fi

echo "auto-claimer started network=$NETWORK api=$API_URL interval=${POLL_INTERVAL_SEC}s candidate_scan_limit=$CANDIDATE_SCAN_LIMIT agent_scan_limit=$AGENT_SCAN_LIMIT order_scan_window=$ORDER_SCAN_WINDOW max_claims_per_cycle=$MAX_CLAIMS_PER_CYCLE auto_withdraw=$AUTO_WITHDRAW dry_run=$DRY_RUN"

while true; do
  if ! poll_and_claim; then
    if [[ "$ONCE" -eq 1 ]]; then
      exit 1
    fi
  fi

  if [[ "$ONCE" -eq 1 ]]; then
    break
  fi

  sleep "$POLL_INTERVAL_SEC"
done
