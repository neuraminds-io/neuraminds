#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

NETWORK="mainnet"
RPC_URL=""
API_URL="${API_URL:-http://127.0.0.1:8080/v1}"
INTERVAL_SEC="${MATCHER_INTERVAL_SEC:-15}"
MAX_FILL_SIZE="${MATCHER_MAX_FILL_SIZE:-1000000}"
MAX_MARKETS_PER_CYCLE="${MATCHER_MAX_MARKETS_PER_CYCLE:-100}"
MAX_MATCHES_PER_MARKET="${MATCHER_RATE_LIMIT_PER_MARKET:-1}"
DRY_RUN=0
ONCE=0

ORDER_BOOK_ADDRESS="${ORDER_BOOK_ADDRESS:-${NEXT_PUBLIC_ORDER_BOOK_ADDRESS:-}}"
MATCHER_PRIVATE_KEY="${BASE_MATCHER_PRIVATE_KEY:-${BASE_OPERATOR_PRIVATE_KEY:-${BASE_AGENT_RUNTIME_OPERATOR_PRIVATE_KEY:-}}}"
ADMIN_CONTROL_KEY="${ADMIN_CONTROL_KEY:-}"

usage() {
  cat <<USAGE
Usage: scripts/base-matcher-worker.sh [options]

Continuously scans OrderBook orders on Base and submits matchOrders transactions.

Options:
  --network mainnet|sepolia       Target chain (default: mainnet)
  --rpc-url <url>                 RPC URL override
  --api-url <url>                 API base URL (default: http://127.0.0.1:8080/v1)
  --interval-sec <seconds>        Polling interval (default: 15)
  --max-fill-size <amount>        Max fill size per match tx (default: 1000000)
  --max-markets-per-cycle <n>     Max markets scanned per cycle (default: 100)
  --max-matches-per-market <n>    Max match txs per market per cycle (default: 1)
  --once                          Run one cycle then exit
  --dry-run                       Print planned txs without sending
  -h|--help                       Show this help

Required for live execution:
  ORDER_BOOK_ADDRESS
  BASE_MATCHER_PRIVATE_KEY (or BASE_OPERATOR_PRIVATE_KEY / BASE_AGENT_RUNTIME_OPERATOR_PRIVATE_KEY)
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
    --max-fill-size)
      MAX_FILL_SIZE="${2:-}"
      shift 2
      ;;
    --max-fill-size=*)
      MAX_FILL_SIZE="${1#*=}"
      shift
      ;;
    --max-markets-per-cycle)
      MAX_MARKETS_PER_CYCLE="${2:-}"
      shift 2
      ;;
    --max-markets-per-cycle=*)
      MAX_MARKETS_PER_CYCLE="${1#*=}"
      shift
      ;;
    --max-matches-per-market)
      MAX_MATCHES_PER_MARKET="${2:-}"
      shift 2
      ;;
    --max-matches-per-market=*)
      MAX_MATCHES_PER_MARKET="${1#*=}"
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

if [[ "$API_URL" == */v1 ]]; then
  API_URL="${API_URL%/}"
else
  API_URL="${API_URL%/}/v1"
fi

require_bin() {
  local missing=0
  for bin in cast jq curl; do
    if ! command -v "$bin" >/dev/null 2>&1; then
      echo "missing required binary: $bin" >&2
      missing=1
    fi
  done
  if [[ "$missing" -eq 1 ]]; then
    exit 1
  fi
}

to_dec() {
  local value="$1"
  cast to-dec "$value" 2>/dev/null || echo "0"
}

is_paused() {
  local payload
  if ! payload="$(curl -fsS "$API_URL/evm/matcher/health" 2>/dev/null)"; then
    echo "false"
    return 0
  fi
  echo "$payload" | jq -r '.paused // false'
}

post_stats() {
  local attempted="$1"
  local matched="$2"
  local failed="$3"
  local backlog="$4"
  local tx_latency_ms="$5"
  local last_tx_hash="${6:-}"

  if [[ -z "$ADMIN_CONTROL_KEY" ]]; then
    return 0
  fi

  local body
  body="$(jq -n \
    --argjson attempted "$attempted" \
    --argjson matched "$matched" \
    --argjson failed "$failed" \
    --argjson backlog "$backlog" \
    --argjson txLatencyMs "$tx_latency_ms" \
    --arg lastTxHash "$last_tx_hash" \
    '{
      attempted: $attempted,
      matched: $matched,
      failed: $failed,
      backlog: $backlog,
      txLatencyMs: $txLatencyMs,
      lastTxHash: (if $lastTxHash == "" then null else $lastTxHash end)
    }')"

  curl -fsS \
    -X POST \
    -H "content-type: application/json" \
    -H "x-admin-key: $ADMIN_CONTROL_KEY" \
    -d "$body" \
    "$API_URL/evm/matcher/report" >/dev/null || true
}

fetch_order_count() {
  local raw
  raw="$(cast call --rpc-url "$RPC_URL" "$ORDER_BOOK_ADDRESS" "orderCount()(uint256)" 2>/dev/null || echo "0")"
  to_dec "$raw"
}

read_order_row() {
  local order_id="$1"
  local raw
  raw="$(cast call \
    --rpc-url "$RPC_URL" \
    "$ORDER_BOOK_ADDRESS" \
    "orders(uint256)(address,uint256,bool,uint128,uint128,uint128,uint64,bool)" \
    "$order_id" 2>/dev/null || true)"
  if [[ -z "$raw" ]]; then
    return 1
  fi

  local maker market_id is_yes price_bps remaining expiry canceled
  maker="$(echo "$raw" | sed -n '1p' | tr -d '[:space:]')"
  market_id="$(to_dec "$(echo "$raw" | sed -n '2p' | tr -d '[:space:]')")"
  is_yes="$(echo "$raw" | sed -n '3p' | tr -d '[:space:]' | tr '[:upper:]' '[:lower:]')"
  price_bps="$(to_dec "$(echo "$raw" | sed -n '4p' | tr -d '[:space:]')")"
  remaining="$(to_dec "$(echo "$raw" | sed -n '6p' | tr -d '[:space:]')")"
  expiry="$(to_dec "$(echo "$raw" | sed -n '7p' | tr -d '[:space:]')")"
  canceled="$(echo "$raw" | sed -n '8p' | tr -d '[:space:]' | tr '[:upper:]' '[:lower:]')"

  if [[ -z "$maker" || "$maker" == "0x0000000000000000000000000000000000000000" ]]; then
    return 1
  fi

  echo "$order_id $market_id $is_yes $price_bps $remaining $expiry $canceled"
}

execute_cycle() {
  local started_ms ended_ms
  started_ms="$(date +%s%3N)"
  local now
  now="$(date +%s)"

  local paused
  paused="$(is_paused)"
  if [[ "$paused" == "true" ]]; then
    echo "matcher paused by admin"
    post_stats 0 0 0 0 0 ""
    return 0
  fi

  local count
  count="$(fetch_order_count)"
  if [[ -z "$count" || "$count" == "0" ]]; then
    post_stats 0 0 0 0 0 ""
    echo "no orders"
    return 0
  fi

  local tmp_orders tmp_candidates
  tmp_orders="$(mktemp)"
  tmp_candidates="$(mktemp)"
  trap 'rm -f "$tmp_orders" "$tmp_candidates"' RETURN

  local i
  for ((i=1; i<=count; i++)); do
    local row
    if ! row="$(read_order_row "$i")"; then
      continue
    fi
    local order_id market_id is_yes price_bps remaining expiry canceled
    read -r order_id market_id is_yes price_bps remaining expiry canceled <<<"$row"

    if [[ "$canceled" == "true" ]]; then
      continue
    fi
    if (( remaining <= 0 )); then
      continue
    fi
    if (( expiry > 0 && expiry < now )); then
      continue
    fi
    printf "%s\t%s\t%s\t%s\t%s\t%s\n" "$market_id" "$is_yes" "$price_bps" "$remaining" "$order_id" "$expiry" >>"$tmp_orders"
  done

  if [[ ! -s "$tmp_orders" ]]; then
    post_stats 0 0 0 0 0 ""
    echo "no open orders"
    return 0
  fi

  local processed_markets=0
  local attempted=0
  local matched=0
  local failed=0
  local backlog=0
  local last_tx=""

  mapfile -t markets < <(cut -f1 "$tmp_orders" | sort -n | uniq | head -n "$MAX_MARKETS_PER_CYCLE")
  for market_id in "${markets[@]}"; do
    (( processed_markets += 1 ))
    local matches_for_market=0

    while (( matches_for_market < MAX_MATCHES_PER_MARKET )); do
      local best_yes best_no
      best_yes="$(awk -F '\t' -v m="$market_id" '$1==m && $2=="true" {print}' "$tmp_orders" | sort -t $'\t' -k3,3nr -k5,5n | head -n1)"
      best_no="$(awk -F '\t' -v m="$market_id" '$1==m && $2=="false" {print}' "$tmp_orders" | sort -t $'\t' -k3,3nr -k5,5n | head -n1)"
      if [[ -z "$best_yes" || -z "$best_no" ]]; then
        break
      fi

      local yes_price no_price
      yes_price="$(echo "$best_yes" | cut -f3)"
      no_price="$(echo "$best_no" | cut -f3)"
      if (( yes_price + no_price < 10000 )); then
        break
      fi

      (( backlog += 1 ))
      (( attempted += 1 ))

      local yes_remaining no_remaining yes_order_id no_order_id
      yes_remaining="$(echo "$best_yes" | cut -f4)"
      no_remaining="$(echo "$best_no" | cut -f4)"
      yes_order_id="$(echo "$best_yes" | cut -f5)"
      no_order_id="$(echo "$best_no" | cut -f5)"

      local fill="$yes_remaining"
      if (( no_remaining < fill )); then fill="$no_remaining"; fi
      if (( MAX_FILL_SIZE < fill )); then fill="$MAX_FILL_SIZE"; fi
      if (( fill <= 0 )); then
        break
      fi

      if [[ "$DRY_RUN" -eq 1 ]]; then
        echo "dry-run match market=$market_id yes_order=$yes_order_id no_order=$no_order_id fill=$fill"
        (( matched += 1 ))
      else
        local out
        if out="$(cast send \
          --async \
          --rpc-url "$RPC_URL" \
          --private-key "$MATCHER_PRIVATE_KEY" \
          "$ORDER_BOOK_ADDRESS" \
          "matchOrders(uint256,uint256,uint128)" \
          "$yes_order_id" "$no_order_id" "$fill" 2>&1)"; then
          (( matched += 1 ))
          last_tx="$(echo "$out" | tr -d '\n' | tr -d '\r')"
          echo "matched market=$market_id yes_order=$yes_order_id no_order=$no_order_id fill=$fill tx=$last_tx"
        else
          (( failed += 1 ))
          echo "match failed market=$market_id yes_order=$yes_order_id no_order=$no_order_id fill=$fill error=$out" >&2
        fi
      fi

      local yes_new=$(( yes_remaining - fill ))
      local no_new=$(( no_remaining - fill ))
      awk -F '\t' -v OFS='\t' -v id="$yes_order_id" -v rem="$yes_new" '
        $5 == id { $4 = rem; print; next } { print }
      ' "$tmp_orders" >"$tmp_candidates" && mv "$tmp_candidates" "$tmp_orders"
      awk -F '\t' -v OFS='\t' -v id="$no_order_id" -v rem="$no_new" '
        $5 == id { $4 = rem; print; next } { print }
      ' "$tmp_orders" >"$tmp_candidates" && mv "$tmp_candidates" "$tmp_orders"

      awk -F '\t' '$4 > 0 { print }' "$tmp_orders" >"$tmp_candidates" && mv "$tmp_candidates" "$tmp_orders"

      (( matches_for_market += 1 ))
    done
  done

  ended_ms="$(date +%s%3N)"
  local latency_ms=$(( ended_ms - started_ms ))
  post_stats "$attempted" "$matched" "$failed" "$backlog" "$latency_ms" "$last_tx"
  echo "cycle complete markets=$processed_markets attempted=$attempted matched=$matched failed=$failed backlog=$backlog latency_ms=$latency_ms"
}

require_bin

if [[ "$NETWORK" != "mainnet" && "$NETWORK" != "sepolia" ]]; then
  echo "--network must be mainnet or sepolia" >&2
  exit 1
fi

if [[ "$DRY_RUN" -eq 0 ]]; then
  if [[ -z "$ORDER_BOOK_ADDRESS" ]]; then
    echo "ORDER_BOOK_ADDRESS is required" >&2
    exit 1
  fi
  if [[ -z "$MATCHER_PRIVATE_KEY" ]]; then
    echo "BASE_MATCHER_PRIVATE_KEY (or BASE_OPERATOR_PRIVATE_KEY / BASE_AGENT_RUNTIME_OPERATOR_PRIVATE_KEY) is required" >&2
    exit 1
  fi
fi

echo "matcher worker started network=$NETWORK api=$API_URL interval=${INTERVAL_SEC}s dry_run=$DRY_RUN"

while true; do
  if ! execute_cycle; then
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
