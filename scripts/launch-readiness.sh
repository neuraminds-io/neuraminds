#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODE="production"
STRICT=0
ALLOW_MISSING_SECRETS=0
API_URL=""
WEB_URL=""
CHAIN_MODE="${CHAIN_MODE:-base}"
RUN_WEB_E2E=0

for arg in "$@"; do
  case "$arg" in
    --strict)
      STRICT=1
      ;;
    --mode=*)
      MODE="${arg#*=}"
      ;;
    --allow-missing-secrets)
      ALLOW_MISSING_SECRETS=1
      ;;
    --api-url=*)
      API_URL="${arg#*=}"
      ;;
    --web-url=*)
      WEB_URL="${arg#*=}"
      ;;
    --chain-mode=*)
      CHAIN_MODE="${arg#*=}"
      ;;
    --run-web-e2e)
      RUN_WEB_E2E=1
      ;;
    *)
      echo "Unknown flag: $arg"
      echo "Usage: scripts/launch-readiness.sh [--strict] [--mode=production|staging|development] [--allow-missing-secrets] [--api-url=<url>] [--web-url=<url>] [--chain-mode=base|solana|dual] [--run-web-e2e]"
      exit 1
      ;;
  esac
done

echo "launch readiness starting"
echo "mode=${MODE}"
echo "strict=${STRICT}"
echo "allow_missing_secrets=${ALLOW_MISSING_SECRETS}"
echo "chain_mode=${CHAIN_MODE}"
echo "run_web_e2e=${RUN_WEB_E2E}"
echo "api_url=${API_URL:-<not set>}"
echo "web_url=${WEB_URL:-<not set>}"
echo ""

CONFIG_ARGS=(--mode="${MODE}" --write-report)
if [[ "${ALLOW_MISSING_SECRETS}" -eq 1 ]]; then
  CONFIG_ARGS+=(--allow-missing-secrets)
fi

(
  cd "${ROOT_DIR}"
  CHAIN_MODE="${CHAIN_MODE}" node scripts/validate-launch-config.mjs "${CONFIG_ARGS[@]}"
)

if [[ "${STRICT}" -eq 1 ]]; then
  (
    cd "${ROOT_DIR}"
    node scripts/production-loop-report.mjs --strict
  )
else
  (
    cd "${ROOT_DIR}"
    node scripts/production-loop-report.mjs
  )
fi

if [[ -n "${API_URL}" ]]; then
  SYNTH_ARGS=(--env "${MODE}" --api-url "${API_URL}" --chain-mode "${CHAIN_MODE}")
  if [[ -n "${WEB_URL}" ]]; then
    SYNTH_ARGS+=(--web-url "${WEB_URL}")
  fi

  (
    cd "${ROOT_DIR}"
    node scripts/synthetic-monitor.mjs "${SYNTH_ARGS[@]}"
  )
else
  echo "synthetic monitor skipped (set --api-url to enable live endpoint checks)"
fi

if [[ "${RUN_WEB_E2E}" -eq 1 ]]; then
  if [[ -z "${API_URL}" || -z "${WEB_URL}" ]]; then
    echo "web e2e smoke requires both --api-url and --web-url"
    exit 1
  fi

  if [[ "${CHAIN_MODE}" == "solana" ]]; then
    echo "web e2e smoke skipped: --chain-mode=solana"
  else
    (
      cd "${ROOT_DIR}"
      node scripts/base-sepolia-web-smoke.mjs --api-url "${API_URL}" --web-url "${WEB_URL}"
    )
  fi
fi

echo ""
echo "launch readiness complete"
echo "reports:"
echo "- docs/reports/launch-config-report.json"
echo "- docs/reports/production-loop-report.json"
echo "- docs/reports/launch-go-no-go.json"
if [[ -n "${API_URL}" ]]; then
  echo "- docs/reports/synthetic-monitor-${MODE}.json"
fi
if [[ "${RUN_WEB_E2E}" -eq 1 ]]; then
  echo "- web/playwright-report/index.html"
fi

(
  cd "${ROOT_DIR}"
  node scripts/generate-launch-summary.mjs || true
)
