#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPORT_DIR="docs/reports"

PHASE="all"
CHAIN_MODE="base"
STRICT=1
WORKER_LIVE_TX=0
STAGING_NETWORK="sepolia"
PRODUCTION_NETWORK="mainnet"

STAGING_API_URL="${SYNTHETIC_STAGING_API_URL:-}"
STAGING_WEB_URL="${SYNTHETIC_STAGING_WEB_URL:-}"
PRODUCTION_API_URL="${SYNTHETIC_PROD_API_URL:-}"
PRODUCTION_WEB_URL="${SYNTHETIC_PROD_WEB_URL:-}"

STAGING_CHAIN_ID="84532"
PRODUCTION_CHAIN_ID="8453"

MONITOR_SAMPLES="6"
MONITOR_INTERVAL_SEC="15"
SOAK_SAMPLES="96"
SOAK_INTERVAL_SEC="900"

MAX_PERSISTENT_MATCHER_BACKLOG_SEC="60"
MAX_PAYOUT_OLDEST_PENDING_SEC="600"
MAX_INDEXER_LAG_BLOCKS="20"

SKIP_CHAOS=0

usage() {
  cat <<USAGE
Usage: scripts/launch-ops-execute.sh [options]

Options:
  --phase phase1|phase2|phase3|phase4|all
  --staging-api-url <url>
  --staging-web-url <url>
  --production-api-url <url>
  --production-web-url <url>
  --chain-mode base|solana|dual
  --staging-chain-id <id>
  --production-chain-id <id>
  --staging-network sepolia|mainnet
  --production-network sepolia|mainnet
  --monitor-samples <n>
  --monitor-interval-sec <n>
  --soak-samples <n>
  --soak-interval-sec <n>
  --strict / --no-strict
  --worker-live-tx (default: dry-run workers)
  --skip-chaos
  -h|--help
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --phase)
      PHASE="${2:-}"
      shift 2
      ;;
    --phase=*)
      PHASE="${1#*=}"
      shift
      ;;
    --staging-api-url)
      STAGING_API_URL="${2:-}"
      shift 2
      ;;
    --staging-api-url=*)
      STAGING_API_URL="${1#*=}"
      shift
      ;;
    --staging-web-url)
      STAGING_WEB_URL="${2:-}"
      shift 2
      ;;
    --staging-web-url=*)
      STAGING_WEB_URL="${1#*=}"
      shift
      ;;
    --production-api-url)
      PRODUCTION_API_URL="${2:-}"
      shift 2
      ;;
    --production-api-url=*)
      PRODUCTION_API_URL="${1#*=}"
      shift
      ;;
    --production-web-url)
      PRODUCTION_WEB_URL="${2:-}"
      shift 2
      ;;
    --production-web-url=*)
      PRODUCTION_WEB_URL="${1#*=}"
      shift
      ;;
    --chain-mode)
      CHAIN_MODE="${2:-}"
      shift 2
      ;;
    --chain-mode=*)
      CHAIN_MODE="${1#*=}"
      shift
      ;;
    --staging-chain-id)
      STAGING_CHAIN_ID="${2:-}"
      shift 2
      ;;
    --staging-chain-id=*)
      STAGING_CHAIN_ID="${1#*=}"
      shift
      ;;
    --production-chain-id)
      PRODUCTION_CHAIN_ID="${2:-}"
      shift 2
      ;;
    --production-chain-id=*)
      PRODUCTION_CHAIN_ID="${1#*=}"
      shift
      ;;
    --staging-network)
      STAGING_NETWORK="${2:-}"
      shift 2
      ;;
    --staging-network=*)
      STAGING_NETWORK="${1#*=}"
      shift
      ;;
    --production-network)
      PRODUCTION_NETWORK="${2:-}"
      shift 2
      ;;
    --production-network=*)
      PRODUCTION_NETWORK="${1#*=}"
      shift
      ;;
    --monitor-samples)
      MONITOR_SAMPLES="${2:-}"
      shift 2
      ;;
    --monitor-samples=*)
      MONITOR_SAMPLES="${1#*=}"
      shift
      ;;
    --monitor-interval-sec)
      MONITOR_INTERVAL_SEC="${2:-}"
      shift 2
      ;;
    --monitor-interval-sec=*)
      MONITOR_INTERVAL_SEC="${1#*=}"
      shift
      ;;
    --soak-samples)
      SOAK_SAMPLES="${2:-}"
      shift 2
      ;;
    --soak-samples=*)
      SOAK_SAMPLES="${1#*=}"
      shift
      ;;
    --soak-interval-sec)
      SOAK_INTERVAL_SEC="${2:-}"
      shift 2
      ;;
    --soak-interval-sec=*)
      SOAK_INTERVAL_SEC="${1#*=}"
      shift
      ;;
    --strict)
      STRICT=1
      shift
      ;;
    --no-strict)
      STRICT=0
      shift
      ;;
    --worker-live-tx)
      WORKER_LIVE_TX=1
      shift
      ;;
    --skip-chaos)
      SKIP_CHAOS=1
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

normalize_url() {
  local value="$1"
  value="${value%/}"
  printf '%s' "$value"
}

require_url() {
  local value="$1"
  local label="$2"
  if [[ -z "$value" ]]; then
    echo "missing required URL: $label" >&2
    exit 1
  fi
}

run_worker_once() {
  local label="$1"
  local log_file="$2"
  shift 2

  set +e
  "$@" >>"$log_file" 2>&1
  local status=$?
  set -e

  echo "$label exit_code=$status" >>"$log_file"
  if [[ "$status" -ne 0 ]]; then
    echo "warning: $label failed (exit $status)"
  fi

  return 0
}

probe_compliance() {
  local base_url="$1"
  local output_json="$2"

  local blocked_status
  local allowed_status
  local probe_mode
  local geo_override_key
  local -a blocked_headers
  local -a allowed_headers

  geo_override_key="${GEO_TEST_OVERRIDE_KEY:-}"
  probe_mode="x-country-fallback"
  blocked_headers=("-H" "content-type: application/json" "-H" "X-Country: US")
  allowed_headers=("-H" "content-type: application/json" "-H" "X-Country: JP")

  if [[ -n "$geo_override_key" ]]; then
    probe_mode="geo-test-override"
    blocked_headers=(
      "-H" "content-type: application/json"
      "-H" "X-Geo-Test-Key: $geo_override_key"
      "-H" "X-Geo-Test-Country: US"
    )
    allowed_headers=(
      "-H" "content-type: application/json"
      "-H" "X-Geo-Test-Key: $geo_override_key"
      "-H" "X-Geo-Test-Country: JP"
    )
  fi

  blocked_status="$(curl -sS -o /tmp/launch_ops_blocked_response.json -w '%{http_code}' \
    -X POST \
    "${blocked_headers[@]}" \
    -d '{"jsonrpc":"2.0","id":"blocked-check","method":"ping","params":{}}' \
    "$base_url/v1/web4/mcp" || true)"

  allowed_status="$(curl -sS -o /tmp/launch_ops_allowed_response.json -w '%{http_code}' \
    -X POST \
    "${allowed_headers[@]}" \
    -d '{"jsonrpc":"2.0","id":"allowed-check","method":"ping","params":{}}' \
    "$base_url/v1/web4/mcp" || true)"

  node -e '
    const fs = require("fs");
    const out = process.argv[1];
    const blockedStatus = process.argv[2];
    const allowedStatus = process.argv[3];
    const probeMode = process.argv[4];

    let blockedBody = "";
    let allowedBody = "";
    try { blockedBody = fs.readFileSync("/tmp/launch_ops_blocked_response.json", "utf8"); } catch {}
    try { allowedBody = fs.readFileSync("/tmp/launch_ops_allowed_response.json", "utf8"); } catch {}

    const report = {
      generatedAt: new Date().toISOString(),
      probeMode,
      blockedProbe: {
        country: "US",
        status: Number(blockedStatus || 0),
        pass: Number(blockedStatus || 0) === 403,
        body: blockedBody,
      },
      allowedProbe: {
        country: "JP",
        status: Number(allowedStatus || 0),
        pass: Number(allowedStatus || 0) === 200,
        body: allowedBody,
      },
    };

    report.ready = report.blockedProbe.pass && report.allowedProbe.pass;
    fs.writeFileSync(out, `${JSON.stringify(report, null, 2)}\n`, "utf8");
    if (!report.ready) process.exit(1);
  ' "$output_json" "$blocked_status" "$allowed_status" "$probe_mode"
}

mkdir -p "$ROOT_DIR/$REPORT_DIR"

STAGING_API_URL="$(normalize_url "$STAGING_API_URL")"
STAGING_WEB_URL="$(normalize_url "$STAGING_WEB_URL")"
PRODUCTION_API_URL="$(normalize_url "$PRODUCTION_API_URL")"
PRODUCTION_WEB_URL="$(normalize_url "$PRODUCTION_WEB_URL")"

TIMESTAMP="$(date -u +%Y%m%dT%H%M%SZ)"

run_phase1() {
  require_url "$STAGING_API_URL" "staging-api-url"
  require_url "$STAGING_WEB_URL" "staging-web-url"

  echo "phase1: staging stabilize + freeze"

  node "$ROOT_DIR/scripts/synthetic-monitor.mjs" \
    --env staging-live \
    --api-url "$STAGING_API_URL" \
    --web-url "$STAGING_WEB_URL" \
    --chain-mode "$CHAIN_MODE" \
    --output "$REPORT_DIR/synthetic-monitor-staging-live-$TIMESTAMP.json" \
    --output-md "$REPORT_DIR/synthetic-monitor-staging-live-$TIMESTAMP.md"

  if [[ "$STRICT" -eq 1 ]]; then
    bash "$ROOT_DIR/scripts/launch-readiness.sh" \
      --strict \
      --mode=staging \
      --api-url="$STAGING_API_URL" \
      --web-url="$STAGING_WEB_URL" \
      --chain-mode="$CHAIN_MODE" \
      --dx-snapshot-out="$REPORT_DIR/dx-terminal-snapshot-staging-$TIMESTAMP.json"
  else
    bash "$ROOT_DIR/scripts/launch-readiness.sh" \
      --mode=staging \
      --api-url="$STAGING_API_URL" \
      --web-url="$STAGING_WEB_URL" \
      --chain-mode="$CHAIN_MODE"
  fi

  node "$ROOT_DIR/scripts/launch-ops-freeze.mjs" \
    --env staging \
    --api-url "$STAGING_API_URL" \
    --web-url "$STAGING_WEB_URL" \
    --chain-mode "$CHAIN_MODE" \
    --base-chain-id "$STAGING_CHAIN_ID" \
    --duration-hours 24 \
    --output "$REPORT_DIR/launch-freeze-staging-$TIMESTAMP.json" \
    --output-md "$REPORT_DIR/launch-freeze-staging-$TIMESTAMP.md"
}

run_phase2() {
  require_url "$STAGING_API_URL" "staging-api-url"
  require_url "$STAGING_WEB_URL" "staging-web-url"

  echo "phase2: worker reliability closure"

  local worker_log
  worker_log="$ROOT_DIR/$REPORT_DIR/launch-ops-phase2-workers-$TIMESTAMP.log"
  : >"$worker_log"

  local matcher_cmd=(bash "$ROOT_DIR/scripts/base-matcher-worker.sh" --network "$STAGING_NETWORK" --api-url "$STAGING_API_URL" --once)
  local payout_cmd=(bash "$ROOT_DIR/scripts/base-global-payout-worker.sh" --network "$STAGING_NETWORK" --api-url "$STAGING_API_URL" --once)

  if [[ "$WORKER_LIVE_TX" -eq 0 ]]; then
    matcher_cmd+=(--dry-run)
    payout_cmd+=(--dry-run)
  fi

  run_worker_once "matcher_worker" "$worker_log" "${matcher_cmd[@]}"
  run_worker_once "payout_worker" "$worker_log" "${payout_cmd[@]}"

  node "$ROOT_DIR/scripts/launch-ops-monitor.mjs" \
    --env staging-phase2 \
    --api-url "$STAGING_API_URL" \
    --web-url "$STAGING_WEB_URL" \
    --samples "$MONITOR_SAMPLES" \
    --interval-sec "$MONITOR_INTERVAL_SEC" \
    --max-persistent-matcher-backlog-sec "$MAX_PERSISTENT_MATCHER_BACKLOG_SEC" \
    --max-payout-oldest-pending-seconds "$MAX_PAYOUT_OLDEST_PENDING_SEC" \
    --max-indexer-lag-blocks "$MAX_INDEXER_LAG_BLOCKS" \
    --output "$REPORT_DIR/launch-ops-phase2-monitor-$TIMESTAMP.json" \
    --output-md "$REPORT_DIR/launch-ops-phase2-monitor-$TIMESTAMP.md"
}

run_phase3() {
  require_url "$STAGING_API_URL" "staging-api-url"
  require_url "$STAGING_WEB_URL" "staging-web-url"

  echo "phase3: soak + chaos"

  node "$ROOT_DIR/scripts/launch-ops-monitor.mjs" \
    --env staging-soak \
    --api-url "$STAGING_API_URL" \
    --web-url "$STAGING_WEB_URL" \
    --samples "$SOAK_SAMPLES" \
    --interval-sec "$SOAK_INTERVAL_SEC" \
    --max-persistent-matcher-backlog-sec "$MAX_PERSISTENT_MATCHER_BACKLOG_SEC" \
    --max-payout-oldest-pending-seconds "$MAX_PAYOUT_OLDEST_PENDING_SEC" \
    --max-indexer-lag-blocks "$MAX_INDEXER_LAG_BLOCKS" \
    --output "$REPORT_DIR/launch-ops-phase3-soak-$TIMESTAMP.json" \
    --output-md "$REPORT_DIR/launch-ops-phase3-soak-$TIMESTAMP.md"

  probe_compliance "$STAGING_API_URL" "$ROOT_DIR/$REPORT_DIR/launch-ops-phase3-compliance-$TIMESTAMP.json"

  if [[ "$SKIP_CHAOS" -eq 1 ]]; then
    echo "phase3 chaos drill skipped (--skip-chaos)"
    return 0
  fi

  if [[ -n "${ADMIN_CONTROL_KEY:-}" ]]; then
    curl -fsS \
      -X POST \
      -H 'content-type: application/json' \
      -H "x-admin-key: $ADMIN_CONTROL_KEY" \
      -d '{"reason":"launch_ops_chaos_drill"}' \
      "$STAGING_API_URL/v1/evm/matcher/pause" >/dev/null

    sleep 3

    curl -fsS \
      -X POST \
      -H 'content-type: application/json' \
      -H "x-admin-key: $ADMIN_CONTROL_KEY" \
      "$STAGING_API_URL/v1/evm/matcher/resume" >/dev/null

    node "$ROOT_DIR/scripts/launch-ops-monitor.mjs" \
      --env staging-post-chaos \
      --api-url "$STAGING_API_URL" \
      --web-url "$STAGING_WEB_URL" \
      --samples "$MONITOR_SAMPLES" \
      --interval-sec "$MONITOR_INTERVAL_SEC" \
      --max-persistent-matcher-backlog-sec "$MAX_PERSISTENT_MATCHER_BACKLOG_SEC" \
      --max-payout-oldest-pending-seconds "$MAX_PAYOUT_OLDEST_PENDING_SEC" \
      --max-indexer-lag-blocks "$MAX_INDEXER_LAG_BLOCKS" \
      --output "$REPORT_DIR/launch-ops-phase3-post-chaos-$TIMESTAMP.json" \
      --output-md "$REPORT_DIR/launch-ops-phase3-post-chaos-$TIMESTAMP.md"
  else
    echo "phase3 chaos drill skipped (ADMIN_CONTROL_KEY not set)"
  fi
}

run_phase4() {
  require_url "$PRODUCTION_API_URL" "production-api-url"
  require_url "$PRODUCTION_WEB_URL" "production-web-url"

  echo "phase4: production cutover readiness"

  bash "$ROOT_DIR/scripts/launch-readiness.sh" \
    --strict \
    --mode=production \
    --api-url="$PRODUCTION_API_URL" \
    --web-url="$PRODUCTION_WEB_URL" \
    --chain-mode="$CHAIN_MODE" \
    --dx-snapshot-out="$REPORT_DIR/dx-terminal-snapshot-production-$TIMESTAMP.json"

  node "$ROOT_DIR/scripts/launch-ops-monitor.mjs" \
    --env production-preflight \
    --api-url "$PRODUCTION_API_URL" \
    --web-url "$PRODUCTION_WEB_URL" \
    --samples "$MONITOR_SAMPLES" \
    --interval-sec "$MONITOR_INTERVAL_SEC" \
    --max-persistent-matcher-backlog-sec "$MAX_PERSISTENT_MATCHER_BACKLOG_SEC" \
    --max-payout-oldest-pending-seconds "$MAX_PAYOUT_OLDEST_PENDING_SEC" \
    --max-indexer-lag-blocks "$MAX_INDEXER_LAG_BLOCKS" \
    --output "$REPORT_DIR/launch-ops-phase4-preflight-$TIMESTAMP.json" \
    --output-md "$REPORT_DIR/launch-ops-phase4-preflight-$TIMESTAMP.md"

  bash "$ROOT_DIR/scripts/base-wallet-status.sh" --network "$PRODUCTION_NETWORK" >"$ROOT_DIR/$REPORT_DIR/base-wallet-status-$PRODUCTION_NETWORK-$TIMESTAMP.txt"

  node -e '
    const fs = require("fs");
    const output = process.argv[1];
    const chainMode = process.argv[2];
    const chainId = Number(process.argv[3]);
    const api = process.argv[4];
    const web = process.argv[5];
    const timestamp = new Date().toISOString();

    const report = {
      generatedAt: timestamp,
      chainMode,
      chainId,
      endpoints: { api, web },
      workerStartOrder: ["indexer", "matcher", "payout", "xmtp-bridge", "mcp-server"],
      syntheticCadence: {
        everyMinutes: 15,
        durationMinutes: 120,
      },
      rollbackThresholds: {
        api5xxPercentOver5m: 2,
        matcherBacklogSeconds: 60,
        payoutStuckSeconds: 600,
        indexerLagBlocks: 20,
      },
      notes: [
        "No missing-secret tolerance allowed in production strict mode.",
        "Address manifest must match backend, frontend, and workflows.",
      ],
    };

    fs.writeFileSync(output, `${JSON.stringify(report, null, 2)}\n`, "utf8");
  ' "$ROOT_DIR/$REPORT_DIR/launch-ops-phase4-cutover-$TIMESTAMP.json" "$CHAIN_MODE" "$PRODUCTION_CHAIN_ID" "$PRODUCTION_API_URL" "$PRODUCTION_WEB_URL"
}

case "$PHASE" in
  phase1)
    run_phase1
    ;;
  phase2)
    run_phase2
    ;;
  phase3)
    run_phase3
    ;;
  phase4)
    run_phase4
    ;;
  all)
    run_phase1
    run_phase2
    run_phase3
    run_phase4
    ;;
  *)
    echo "invalid phase: $PHASE" >&2
    usage
    exit 1
    ;;
esac

echo "launch ops execution complete"
echo "timestamp=$TIMESTAMP"
echo "reports_dir=$REPORT_DIR"
