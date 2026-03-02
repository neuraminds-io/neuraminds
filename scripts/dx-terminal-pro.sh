#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
API_URL="${DX_TERMINAL_API_URL:-https://api.terminal.markets}"
RPC_URL="${DX_TERMINAL_RPC_URL:-https://mainnet.base.org}"
DRY_RUN=0

load_env() {
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

  API_URL="${DX_TERMINAL_API_URL:-$API_URL}"
  RPC_URL="${DX_TERMINAL_RPC_URL:-$RPC_URL}"
}

usage() {
  cat <<USAGE
Usage: scripts/dx-terminal-pro.sh <command> [args] [--dry-run]

Read commands:
  vault
  positions
  deposits-withdrawals [limit]
  swaps [limit]
  logs [limit]
  strategies [activeOnly:true|false]
  leaderboard [limit]
  pnl-history
  tokens [includeMarketData:true|false]
  candles <token_address> <timeframe> [countback]
  holders <token_address> [limit] [offset]
  snapshot [output_json]

Write commands:
  update-settings <maxTradeBps> <slippageBps> <tradingActivity> <assetRiskPreference> <tradeSize> <holdingStyle> <diversification>
  add-strategy <priority_0_to_2> <expiry_unix_or_0> <text>
  disable-strategy <strategy_id>
  deposit <amount_eth>
  withdraw <amount_wei>

Environment:
  DX_TERMINAL_PRIVATE_KEY   Required for writes and owner derivation
  DX_TERMINAL_OWNER_ADDRESS Optional for reads
  DX_TERMINAL_VAULT_ADDRESS Optional override for reads/writes
  DX_TERMINAL_API_URL       Default: https://api.terminal.markets
  DX_TERMINAL_RPC_URL       Default: https://mainnet.base.org
USAGE
}

require_bins() {
  local missing=0
  for bin in cast curl jq; do
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

is_address() {
  [[ "$1" =~ ^0x[0-9a-fA-F]{40}$ ]]
}

require_private_key() {
  if [[ -z "${DX_TERMINAL_PRIVATE_KEY:-}" ]]; then
    echo "DX_TERMINAL_PRIVATE_KEY is required for this command" >&2
    exit 1
  fi
}

owner_address() {
  if [[ -n "${DX_TERMINAL_OWNER_ADDRESS:-}" ]]; then
    if ! is_address "$DX_TERMINAL_OWNER_ADDRESS"; then
      echo "DX_TERMINAL_OWNER_ADDRESS is not a valid address" >&2
      exit 1
    fi
    echo "$DX_TERMINAL_OWNER_ADDRESS"
    return
  fi

  require_private_key
  cast wallet address --private-key "$DX_TERMINAL_PRIVATE_KEY"
}

vault_address() {
  if [[ -n "${DX_TERMINAL_VAULT_ADDRESS:-}" ]]; then
    if ! is_address "$DX_TERMINAL_VAULT_ADDRESS"; then
      echo "DX_TERMINAL_VAULT_ADDRESS is not a valid address" >&2
      exit 1
    fi
    echo "$DX_TERMINAL_VAULT_ADDRESS"
    return
  fi

  local owner
  owner="$(owner_address)"
  local vault
  vault="$(curl -fsS "$API_URL/api/v1/vault?ownerAddress=$owner" | jq -r '.vaultAddress // empty')"

  if [[ -z "$vault" ]]; then
    echo "failed to resolve vault for owner: $owner" >&2
    exit 1
  fi

  if ! is_address "$vault"; then
    echo "resolved vault is invalid: $vault" >&2
    exit 1
  fi

  echo "$vault"
}

api_get() {
  local path="$1"
  curl -fsS "$API_URL$path"
}

print_json() {
  jq .
}

send_tx() {
  local vault="$1"
  local sig="$2"
  shift 2

  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "dry-run: cast send $vault \"$sig\" $* --rpc-url $RPC_URL"
    return
  fi

  require_private_key
  cast send "$vault" "$sig" "$@" --private-key "$DX_TERMINAL_PRIVATE_KEY" --rpc-url "$RPC_URL"
}

send_tx_with_value() {
  local vault="$1"
  local sig="$2"
  local value_eth="$3"

  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "dry-run: cast send $vault \"$sig\" --value ${value_eth}ether --rpc-url $RPC_URL"
    return
  fi

  require_private_key
  cast send "$vault" "$sig" --value "${value_eth}ether" --private-key "$DX_TERMINAL_PRIVATE_KEY" --rpc-url "$RPC_URL"
}

cmd_vault() {
  local vault
  vault="$(vault_address)"
  api_get "/api/v1/vault?vaultAddress=$vault" | print_json
}

cmd_positions() {
  local vault
  vault="$(vault_address)"
  api_get "/api/v1/positions/$vault" | print_json
}

cmd_deposits_withdrawals() {
  local limit="${1:-50}"
  if ! is_uint "$limit"; then
    echo "limit must be uint" >&2
    exit 1
  fi

  local vault
  vault="$(vault_address)"
  api_get "/api/v1/deposits-withdrawals/$vault?limit=$limit&order=desc" | print_json
}

cmd_swaps() {
  local limit="${1:-50}"
  if ! is_uint "$limit"; then
    echo "limit must be uint" >&2
    exit 1
  fi

  local vault
  vault="$(vault_address)"
  api_get "/api/v1/swaps?vaultAddress=$vault&limit=$limit&order=desc" | print_json
}

cmd_logs() {
  local limit="${1:-50}"
  if ! is_uint "$limit"; then
    echo "limit must be uint" >&2
    exit 1
  fi

  local vault
  vault="$(vault_address)"
  api_get "/api/v1/logs/$vault?limit=$limit&order=desc" | print_json
}

cmd_strategies() {
  local active="${1:-true}"
  if [[ "$active" != "true" && "$active" != "false" ]]; then
    echo "activeOnly must be true or false" >&2
    exit 1
  fi

  local vault
  vault="$(vault_address)"
  api_get "/api/v1/strategies/$vault?activeOnly=$active" | print_json
}

cmd_leaderboard() {
  local limit="${1:-25}"
  if ! is_uint "$limit"; then
    echo "limit must be uint" >&2
    exit 1
  fi

  api_get "/api/v1/leaderboard?limit=$limit&sortBy=total_pnl_usd" | print_json
}

cmd_pnl_history() {
  local vault
  vault="$(vault_address)"
  api_get "/api/v1/pnl-history/$vault" | print_json
}

cmd_tokens() {
  local include_market_data="${1:-true}"
  if [[ "$include_market_data" != "true" && "$include_market_data" != "false" ]]; then
    echo "includeMarketData must be true or false" >&2
    exit 1
  fi

  api_get "/api/v1/tokens?includeMarketData=$include_market_data" | print_json
}

cmd_candles() {
  local token="$1"
  local timeframe="$2"
  local countback="${3:-300}"

  if ! is_address "$token"; then
    echo "invalid token address" >&2
    exit 1
  fi

  if ! is_uint "$countback"; then
    echo "countback must be uint" >&2
    exit 1
  fi

  case "$timeframe" in
    1m|5m|15m|1h|4h|1d) ;;
    *)
      echo "timeframe must be one of: 1m, 5m, 15m, 1h, 4h, 1d" >&2
      exit 1
      ;;
  esac

  local to
  to="$(date +%s)"
  api_get "/api/v1/candles/$token?timeframe=$timeframe&to=$to&countback=$countback" | print_json
}

cmd_holders() {
  local token="$1"
  local limit="${2:-50}"
  local offset="${3:-0}"

  if ! is_address "$token"; then
    echo "invalid token address" >&2
    exit 1
  fi
  if ! is_uint "$limit" || ! is_uint "$offset"; then
    echo "limit and offset must be uint" >&2
    exit 1
  fi

  api_get "/api/v1/holders/$token?limit=$limit&offset=$offset&order=desc" | print_json
}

cmd_snapshot() {
  local out="${1:-$ROOT_DIR/docs/reports/dx-terminal-snapshot.json}"
  local vault
  vault="$(vault_address)"

  local owner=""
  if [[ -n "${DX_TERMINAL_OWNER_ADDRESS:-}" ]]; then
    owner="$DX_TERMINAL_OWNER_ADDRESS"
  elif [[ -n "${DX_TERMINAL_PRIVATE_KEY:-}" ]]; then
    owner="$(owner_address)"
  fi

  local vault_json
  local positions_json
  local deposits_withdrawals_json
  local swaps_json
  local logs_json
  local strategies_json
  local leaderboard_json
  local pnl_history_json

  vault_json="$(api_get "/api/v1/vault?vaultAddress=$vault")"
  positions_json="$(api_get "/api/v1/positions/$vault")"
  deposits_withdrawals_json="$(api_get "/api/v1/deposits-withdrawals/$vault?limit=50&order=desc")"
  swaps_json="$(api_get "/api/v1/swaps?vaultAddress=$vault&limit=50&order=desc")"
  logs_json="$(api_get "/api/v1/logs/$vault?limit=50&order=desc")"
  strategies_json="$(api_get "/api/v1/strategies/$vault?activeOnly=true")"
  leaderboard_json="$(api_get "/api/v1/leaderboard?limit=25&sortBy=total_pnl_usd")"
  pnl_history_json="$(api_get "/api/v1/pnl-history/$vault")"

  mkdir -p "$(dirname "$out")"
  jq -n \
    --arg fetchedAt "$(date -u +"%Y-%m-%dT%H:%M:%SZ")" \
    --arg apiUrl "$API_URL" \
    --arg ownerAddress "$owner" \
    --arg vaultAddress "$vault" \
    --argjson vault "$vault_json" \
    --argjson positions "$positions_json" \
    --argjson depositsWithdrawals "$deposits_withdrawals_json" \
    --argjson swaps "$swaps_json" \
    --argjson logs "$logs_json" \
    --argjson strategies "$strategies_json" \
    --argjson leaderboard "$leaderboard_json" \
    --argjson pnlHistory "$pnl_history_json" \
    '{
      fetchedAt: $fetchedAt,
      source: "terminal.markets",
      apiUrl: $apiUrl,
      ownerAddress: $ownerAddress,
      vaultAddress: $vaultAddress,
      vault: $vault,
      positions: $positions,
      depositsWithdrawals: $depositsWithdrawals,
      swaps: $swaps,
      logs: $logs,
      strategies: $strategies,
      leaderboard: $leaderboard,
      pnlHistory: $pnlHistory
    }' > "$out"

  echo "snapshot written: $out"
}

cmd_update_settings() {
  if [[ "$#" -ne 7 ]]; then
    echo "update-settings requires 7 numeric args" >&2
    exit 1
  fi

  local max_trade_bps="$1"
  local slippage_bps="$2"
  local trading_activity="$3"
  local asset_risk="$4"
  local trade_size="$5"
  local holding_style="$6"
  local diversification="$7"

  for n in "$max_trade_bps" "$slippage_bps" "$trading_activity" "$asset_risk" "$trade_size" "$holding_style" "$diversification"; do
    if ! is_uint "$n"; then
      echo "all update-settings values must be uint" >&2
      exit 1
    fi
  done

  if (( max_trade_bps < 500 || max_trade_bps > 10000 )); then
    echo "maxTradeBps must be 500..10000" >&2
    exit 1
  fi
  if (( slippage_bps < 10 || slippage_bps > 5000 )); then
    echo "slippageBps must be 10..5000" >&2
    exit 1
  fi

  for slider in "$trading_activity" "$asset_risk" "$trade_size" "$holding_style" "$diversification"; do
    if (( slider < 1 || slider > 5 )); then
      echo "slider values must be 1..5" >&2
      exit 1
    fi
  done

  local vault
  vault="$(vault_address)"

  send_tx "$vault" \
    "updateSettings((uint256,uint256,uint8,uint8,uint8,uint8,uint8))" \
    "($max_trade_bps,$slippage_bps,$trading_activity,$asset_risk,$trade_size,$holding_style,$diversification)"
}

cmd_add_strategy() {
  if [[ "$#" -lt 3 ]]; then
    echo "add-strategy requires: <priority_0_to_2> <expiry_unix_or_0> <text>" >&2
    exit 1
  fi

  local priority="$1"
  local expiry="$2"
  shift 2
  local text="$*"

  if ! is_uint "$priority" || ! is_uint "$expiry"; then
    echo "priority and expiry must be uint" >&2
    exit 1
  fi

  if (( priority > 2 )); then
    echo "priority must be 0..2" >&2
    exit 1
  fi

  if [[ -z "$text" ]]; then
    echo "strategy text cannot be empty" >&2
    exit 1
  fi

  if (( ${#text} > 1024 )); then
    echo "strategy text must be <= 1024 characters" >&2
    exit 1
  fi

  local now
  now="$(date +%s)"
  if (( expiry != 0 && expiry <= now )); then
    echo "expiry must be 0 or a future unix timestamp" >&2
    exit 1
  fi

  local vault
  vault="$(vault_address)"

  send_tx "$vault" "addStrategy(string,uint64,uint8)" "$text" "$expiry" "$priority"
}

cmd_disable_strategy() {
  local strategy_id="$1"
  if ! is_uint "$strategy_id"; then
    echo "strategy id must be uint" >&2
    exit 1
  fi

  local vault
  vault="$(vault_address)"

  send_tx "$vault" "disableStrategy(uint256)" "$strategy_id"
}

cmd_deposit() {
  local amount_eth="$1"
  if ! [[ "$amount_eth" =~ ^[0-9]+([.][0-9]+)?$ ]]; then
    echo "amount_eth must be numeric" >&2
    exit 1
  fi

  local vault
  vault="$(vault_address)"

  send_tx_with_value "$vault" "depositETH()" "$amount_eth"
}

cmd_withdraw() {
  local amount_wei="$1"
  if ! is_uint "$amount_wei"; then
    echo "amount_wei must be uint" >&2
    exit 1
  fi

  local vault
  vault="$(vault_address)"

  send_tx "$vault" "withdrawETH(uint256)" "$amount_wei"
}

main() {
  load_env
  require_bins

  if [[ "$#" -eq 0 ]]; then
    usage
    exit 1
  fi

  local args=()
  while [[ "$#" -gt 0 ]]; do
    case "$1" in
      --dry-run)
        DRY_RUN=1
        shift
        ;;
      *)
        args+=("$1")
        shift
        ;;
    esac
  done

  if [[ "${#args[@]}" -eq 0 ]]; then
    usage
    exit 1
  fi

  local command="${args[0]}"
  local params=("${args[@]:1}")

  case "$command" in
    help|-h|--help)
      usage
      ;;
    vault)
      cmd_vault
      ;;
    positions)
      cmd_positions
      ;;
    deposits-withdrawals)
      cmd_deposits_withdrawals "${params[0]:-50}"
      ;;
    swaps)
      cmd_swaps "${params[0]:-50}"
      ;;
    logs)
      cmd_logs "${params[0]:-50}"
      ;;
    strategies)
      cmd_strategies "${params[0]:-true}"
      ;;
    leaderboard)
      cmd_leaderboard "${params[0]:-25}"
      ;;
    pnl-history)
      cmd_pnl_history
      ;;
    tokens)
      cmd_tokens "${params[0]:-true}"
      ;;
    candles)
      if [[ "${#params[@]}" -lt 2 ]]; then
        echo "candles requires: <token_address> <timeframe> [countback]" >&2
        exit 1
      fi
      cmd_candles "${params[0]}" "${params[1]}" "${params[2]:-300}"
      ;;
    holders)
      if [[ "${#params[@]}" -lt 1 ]]; then
        echo "holders requires: <token_address> [limit] [offset]" >&2
        exit 1
      fi
      cmd_holders "${params[0]}" "${params[1]:-50}" "${params[2]:-0}"
      ;;
    snapshot)
      cmd_snapshot "${params[0]:-$ROOT_DIR/docs/reports/dx-terminal-snapshot.json}"
      ;;
    update-settings)
      cmd_update_settings "${params[@]}"
      ;;
    add-strategy)
      cmd_add_strategy "${params[@]}"
      ;;
    disable-strategy)
      if [[ "${#params[@]}" -ne 1 ]]; then
        echo "disable-strategy requires: <strategy_id>" >&2
        exit 1
      fi
      cmd_disable_strategy "${params[0]}"
      ;;
    deposit)
      if [[ "${#params[@]}" -ne 1 ]]; then
        echo "deposit requires: <amount_eth>" >&2
        exit 1
      fi
      cmd_deposit "${params[0]}"
      ;;
    withdraw)
      if [[ "${#params[@]}" -ne 1 ]]; then
        echo "withdraw requires: <amount_wei>" >&2
        exit 1
      fi
      cmd_withdraw "${params[0]}"
      ;;
    *)
      echo "unknown command: $command" >&2
      usage
      exit 1
      ;;
  esac
}

main "$@"
