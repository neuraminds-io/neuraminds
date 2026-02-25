#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODE="production"
STRICT=0
ALLOW_MISSING_SECRETS=0

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
    *)
      echo "Unknown flag: $arg"
      echo "Usage: scripts/launch-readiness.sh [--strict] [--mode=production|staging|development] [--allow-missing-secrets]"
      exit 1
      ;;
  esac
done

echo "launch readiness starting"
echo "mode=${MODE}"
echo "strict=${STRICT}"
echo "allow_missing_secrets=${ALLOW_MISSING_SECRETS}"
echo ""

CONFIG_ARGS=(--mode="${MODE}" --write-report)
if [[ "${ALLOW_MISSING_SECRETS}" -eq 1 ]]; then
  CONFIG_ARGS+=(--allow-missing-secrets)
fi

(
  cd "${ROOT_DIR}"
  node scripts/validate-launch-config.mjs "${CONFIG_ARGS[@]}"
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

echo ""
echo "launch readiness complete"
echo "reports:"
echo "- docs/reports/launch-config-report.json"
echo "- docs/reports/production-loop-report.json"
echo "- docs/reports/launch-go-no-go.json"

(
  cd "${ROOT_DIR}"
  node scripts/generate-launch-summary.mjs || true
)
