#!/usr/bin/env bash
# Security stress checks for Base/EVM + API stack

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

PASS=0
FAIL=0

check() {
  local name="$1"
  local code="$2"
  if [[ "${code}" == "0" ]]; then
    echo "[pass] ${name}"
    PASS=$((PASS + 1))
  else
    echo "[fail] ${name}"
    FAIL=$((FAIL + 1))
  fi
}

echo "=== neuraminds security stress test (base/evm) ==="
echo

echo "### 1. static analysis ###"
SECRETS=$(rg -n --hidden -S "(secret|password|api_key|private_key)\\s*=\\s*['\"][A-Za-z0-9]" . --glob '!**/node_modules/**' --glob '!**/.next/**' --glob '!**/target/**' --glob '!**/*.example' --glob '!**/tests/**' 2>/dev/null | wc -l | tr -d ' ')
check "No likely hardcoded secrets" "$([ "${SECRETS}" -eq 0 ] && echo 0 || echo 1)"

DANGEROUS_SOLIDITY=$(rg -n "\\b(tx\\.origin|delegatecall|selfdestruct)\\b" evm/src evm/script 2>/dev/null | wc -l | tr -d ' ')
check "No dangerous Solidity primitives in protocol contracts" "$([ "${DANGEROUS_SOLIDITY}" -eq 0 ] && echo 0 || echo 1)"

TODO_SECURITY=$(rg -n "TODO|FIXME" app/src evm/src web/src 2>/dev/null | rg -i "security|auth|key|secret|vuln|exploit" | wc -l | tr -d ' ')
check "No unresolved security TODO/FIXME markers" "$([ "${TODO_SECURITY}" -eq 0 ] && echo 0 || echo 1)"

echo
echo "### 2. backend/api checks ###"
JWT_VALIDATION=$(rg -n "verify|decode|refresh|jwt" app/src/api app/src/middleware 2>/dev/null | wc -l | tr -d ' ')
check "JWT validation logic present" "$([ "${JWT_VALIDATION}" -gt 0 ] && echo 0 || echo 1)"

INPUT_VALIDATION=$(rg -n "validate|validator|regex|schema" app/src/api 2>/dev/null | wc -l | tr -d ' ')
check "Input validation paths present" "$([ "${INPUT_VALIDATION}" -gt 10 ] && echo 0 || echo 1)"

RATE_LIMIT=$(rg -n "rate limit|rate_limit|RateLimit|Throttle" app/src 2>/dev/null | wc -l | tr -d ' ')
check "Rate limiting hooks present" "$([ "${RATE_LIMIT}" -gt 0 ] && echo 0 || echo 1)"

echo
echo "### 3. frontend checks ###"
XSS_DANGER=$(rg -n "dangerouslySetInnerHTML\\s*=" web/src 2>/dev/null | wc -l | tr -d ' ')
check "No dangerouslySetInnerHTML usage" "$([ "${XSS_DANGER}" -eq 0 ] && echo 0 || echo 1)"

EVAL_USAGE=$(rg -n "\\beval\\s*\\(" web/src 2>/dev/null | wc -l | tr -d ' ')
check "No eval() usage" "$([ "${EVAL_USAGE}" -eq 0 ] && echo 0 || echo 1)"

ENV_EXPOSURE=$(rg -n "NEXT_PUBLIC_" web/src 2>/dev/null | rg -v "NEXT_PUBLIC_API_URL|NEXT_PUBLIC_CHAIN_MODE|NEXT_PUBLIC_BASE_RPC_URL|NEXT_PUBLIC_BASE_CHAIN_ID|NEXT_PUBLIC_SIWE_DOMAIN|NEXT_PUBLIC_MARKET_CORE_ADDRESS|NEXT_PUBLIC_ORDER_BOOK_ADDRESS|NEXT_PUBLIC_COLLATERAL_TOKEN_ADDRESS|NEXT_PUBLIC_COLLATERAL_VAULT_ADDRESS" | wc -l | tr -d ' ')
check "Public env exposure limited" "$([ "${ENV_EXPOSURE}" -lt 10 ] && echo 0 || echo 1)"

echo
echo "### 4. dependency audit ###"
if command -v cargo-audit >/dev/null 2>&1; then
  set +e
  cargo audit >/tmp/neura_cargo_audit.log 2>&1
  CARGO_AUDIT_CODE=$?
  set -e
  check "cargo audit clean" "$([ "${CARGO_AUDIT_CODE}" -eq 0 ] && echo 0 || echo 1)"
else
  echo "[warn] cargo-audit not installed; skipping cargo audit"
fi

if command -v npm >/dev/null 2>&1; then
  set +e
  npm --prefix web audit --json >/tmp/neura_npm_audit.json 2>/dev/null
  NPM_AUDIT_CODE=$?
  set -e
  check "npm audit executed" "$([ "${NPM_AUDIT_CODE}" -eq 0 ] && echo 0 || echo 1)"
else
  echo "[warn] npm not installed; skipping npm audit"
fi

echo
echo "### 5. execution checks ###"
set +e
forge test --root evm >/tmp/neura_forge_test.log 2>&1
FORGE_TEST_CODE=$?
set -e
check "forge test passes" "$([ "${FORGE_TEST_CODE}" -eq 0 ] && echo 0 || echo 1)"

set +e
cargo test --manifest-path app/Cargo.toml >/tmp/neura_cargo_test.log 2>&1
CARGO_TEST_CODE=$?
set -e
check "cargo test passes" "$([ "${CARGO_TEST_CODE}" -eq 0 ] && echo 0 || echo 1)"

echo
echo "=== summary ==="
echo "passed: ${PASS}"
echo "failed: ${FAIL}"

if [[ "${FAIL}" -gt 0 ]]; then
  echo "security stress test failed"
  exit 1
fi

echo "security stress test passed"
