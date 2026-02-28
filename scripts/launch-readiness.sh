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
SKIP_DX_SNAPSHOT=0
REQUIRE_DX_SNAPSHOT=0
DX_SNAPSHOT_OUT="docs/reports/dx-terminal-snapshot.json"
DX_SNAPSHOT_CAPTURED=0
REQUIRE_DX_SNAPSHOT_EXPLICIT=0

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
    --skip-dx-snapshot)
      SKIP_DX_SNAPSHOT=1
      ;;
    --require-dx-snapshot)
      REQUIRE_DX_SNAPSHOT=1
      REQUIRE_DX_SNAPSHOT_EXPLICIT=1
      ;;
    --dx-snapshot-out=*)
      DX_SNAPSHOT_OUT="${arg#*=}"
      ;;
    *)
      echo "Unknown flag: $arg"
      echo "Usage: scripts/launch-readiness.sh [--strict] [--mode=production|staging|development] [--allow-missing-secrets] [--api-url=<url>] [--web-url=<url>] [--chain-mode=base|solana|dual] [--run-web-e2e] [--skip-dx-snapshot] [--require-dx-snapshot] [--dx-snapshot-out=<path>]"
      exit 1
      ;;
  esac
done

if [[ "${STRICT}" -eq 1 && "${SKIP_DX_SNAPSHOT}" -eq 0 && "${REQUIRE_DX_SNAPSHOT_EXPLICIT}" -eq 0 ]]; then
  REQUIRE_DX_SNAPSHOT=1
fi

if [[ "${STRICT}" -eq 1 && "${ALLOW_MISSING_SECRETS}" -eq 1 ]]; then
  echo "strict mode cannot run with --allow-missing-secrets"
  exit 1
fi

if [[ -z "${API_URL}" ]]; then
  if [[ "${MODE}" == "production" ]]; then
    API_URL="${SYNTHETIC_PROD_API_URL:-${NEXT_PUBLIC_API_URL:-}}"
  elif [[ "${MODE}" == "staging" ]]; then
    API_URL="${SYNTHETIC_STAGING_API_URL:-${NEXT_PUBLIC_API_URL:-}}"
  fi
fi

if [[ -z "${WEB_URL}" ]]; then
  if [[ "${MODE}" == "production" ]]; then
    WEB_URL="${SYNTHETIC_PROD_WEB_URL:-}"
  elif [[ "${MODE}" == "staging" ]]; then
    WEB_URL="${SYNTHETIC_STAGING_WEB_URL:-}"
  fi
fi

echo "launch readiness starting"
echo "mode=${MODE}"
echo "strict=${STRICT}"
echo "allow_missing_secrets=${ALLOW_MISSING_SECRETS}"
echo "chain_mode=${CHAIN_MODE}"
echo "run_web_e2e=${RUN_WEB_E2E}"
echo "skip_dx_snapshot=${SKIP_DX_SNAPSHOT}"
echo "require_dx_snapshot=${REQUIRE_DX_SNAPSHOT}"
echo "dx_snapshot_out=${DX_SNAPSHOT_OUT}"
echo "api_url=${API_URL:-<not set>}"
echo "web_url=${WEB_URL:-<not set>}"
echo ""

CONFIG_ARGS=(--mode="${MODE}" --write-report)
if [[ "${ALLOW_MISSING_SECRETS}" -eq 1 ]]; then
  CONFIG_ARGS+=(--allow-missing-secrets)
fi

(
  cd "${ROOT_DIR}"
  CHAIN_MODE="${CHAIN_MODE}" node scripts/validate-launch-config.mjs --chain-mode="${CHAIN_MODE}" "${CONFIG_ARGS[@]}"
)

ADDRESS_ENV="production"
if [[ "${MODE}" == "staging" ]]; then
  ADDRESS_ENV="staging"
fi
(
  cd "${ROOT_DIR}"
  node scripts/validate-address-manifest.mjs --environment="${ADDRESS_ENV}" --write-report
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

  (
    cd "${ROOT_DIR}"
    node scripts/base-sepolia-web-smoke.mjs --api-url "${API_URL}" --web-url "${WEB_URL}"
  )
fi

if [[ "${SKIP_DX_SNAPSHOT}" -eq 0 ]]; then
  if (
    cd "${ROOT_DIR}"
    bash scripts/dx-terminal-pro.sh snapshot "${DX_SNAPSHOT_OUT}"
  ); then
    DX_SNAPSHOT_CAPTURED=1
  else
    echo "dx snapshot capture failed"
    if [[ "${REQUIRE_DX_SNAPSHOT}" -eq 1 ]]; then
      exit 1
    fi
  fi
else
  echo "dx snapshot skipped (--skip-dx-snapshot)"
fi

echo ""
echo "launch readiness complete"
echo "reports:"
echo "- docs/reports/launch-config-report.json"
echo "- docs/reports/address-manifest-report.json"
echo "- docs/reports/production-loop-report.json"
echo "- docs/reports/launch-go-no-go.json"
if [[ -n "${API_URL}" ]]; then
  echo "- docs/reports/synthetic-monitor-${MODE}.json"
fi
if [[ "${RUN_WEB_E2E}" -eq 1 ]]; then
  echo "- web/playwright-report/index.html"
fi
if [[ "${DX_SNAPSHOT_CAPTURED}" -eq 1 ]]; then
  echo "- ${DX_SNAPSHOT_OUT}"
fi

(
  cd "${ROOT_DIR}"
  node scripts/generate-launch-summary.mjs || true
)
