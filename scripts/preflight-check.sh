#!/bin/bash
set -euo pipefail

# Polyguard Pre-flight Check
# Run before mainnet deployment to verify everything is ready

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

PASS="${GREEN}PASS${NC}"
FAIL="${RED}FAIL${NC}"
WARN="${YELLOW}WARN${NC}"

passed=0
failed=0
warnings=0

check() {
    local name=$1
    local cmd=$2
    local required=${3:-true}

    if eval "$cmd" >/dev/null 2>&1; then
        echo -e "  [$PASS] $name"
        ((passed++))
    elif [[ "$required" == "true" ]]; then
        echo -e "  [$FAIL] $name"
        ((failed++))
    else
        echo -e "  [$WARN] $name"
        ((warnings++))
    fi
}

echo ""
echo "==========================================="
echo "  Polyguard Pre-flight Check"
echo "==========================================="
echo ""

echo "1. Prerequisites"
check "solana CLI installed" "command -v solana"
check "anchor CLI installed" "command -v anchor"
check "node installed" "command -v node"
check "docker installed" "command -v docker"
check "kubectl installed" "command -v kubectl" false

echo ""
echo "2. Wallet"
KEYPAIR="${KEYPAIR:-~/.config/solana/id.json}"
check "Keypair exists" "[[ -f $KEYPAIR ]]"

if [[ -f "$KEYPAIR" ]]; then
    PUBKEY=$(solana-keygen pubkey "$KEYPAIR" 2>/dev/null || echo "")
    if [[ -n "$PUBKEY" ]]; then
        echo -e "     Pubkey: $PUBKEY"
    fi
fi

echo ""
echo "3. Programs"
check "Market .so exists" "[[ -f target/deploy/polyguard_market.so ]]"
check "Orderbook .so exists" "[[ -f target/deploy/polyguard_orderbook.so ]]"

if [[ -f target/deploy/polyguard_market.so ]]; then
    size=$(ls -lh target/deploy/polyguard_market.so | awk '{print $5}')
    echo -e "     Market size: $size"
fi

if [[ -f target/deploy/polyguard_orderbook.so ]]; then
    size=$(ls -lh target/deploy/polyguard_orderbook.so | awk '{print $5}')
    echo -e "     Orderbook size: $size"
fi

echo ""
echo "4. IDL"
check "Market IDL exists" "[[ -f target/idl/polyguard_market.json ]]"
check "Orderbook IDL exists" "[[ -f target/idl/polyguard_orderbook.json ]]"

echo ""
echo "5. Tests"
echo "   Running Rust tests..."
if cargo test -p polyguard-orderbook --quiet 2>/dev/null; then
    echo -e "  [$PASS] Orderbook tests pass"
    ((passed++))
else
    echo -e "  [$FAIL] Orderbook tests failed"
    ((failed++))
fi

echo ""
echo "6. Balance Check"
RPC_URL="${RPC_URL:-https://api.mainnet-beta.solana.com}"

if [[ -f "$KEYPAIR" ]]; then
    balance=$(solana balance --keypair "$KEYPAIR" --url "$RPC_URL" 2>/dev/null | awk '{print $1}' || echo "0")
    echo -e "     RPC: $RPC_URL"
    echo -e "     Balance: $balance SOL"

    if (( $(echo "$balance >= 9" | bc -l 2>/dev/null || echo 0) )); then
        echo -e "  [$PASS] Sufficient balance (need ~9 SOL)"
        ((passed++))
    else
        echo -e "  [$FAIL] Insufficient balance (need ~9 SOL, have $balance)"
        ((failed++))
    fi
else
    echo -e "  [$FAIL] Cannot check balance - no keypair"
    ((failed++))
fi

echo ""
echo "7. Infrastructure"
check "Docker daemon running" "docker info" false
check "K8s manifests exist" "[[ -f infra/k8s/deployment.yaml ]]"
check "Secrets template exists" "[[ -f infra/k8s/secrets.yaml ]]"

echo ""
echo "8. Documentation"
check "Deployment plan exists" "[[ -f docs/DEPLOYMENT_PLAN.md ]]"
check "API docs exist" "[[ -f docs/openapi.yaml ]]"

echo ""
echo "==========================================="
echo "  Results"
echo "==========================================="
echo ""
echo -e "  Passed:   ${GREEN}$passed${NC}"
echo -e "  Failed:   ${RED}$failed${NC}"
echo -e "  Warnings: ${YELLOW}$warnings${NC}"
echo ""

if [[ $failed -eq 0 ]]; then
    echo -e "${GREEN}Ready for deployment!${NC}"
    echo ""
    echo "Run: ./scripts/deploy-mainnet.sh --generate-keys"
    exit 0
else
    echo -e "${RED}Fix $failed issue(s) before deploying${NC}"
    exit 1
fi
