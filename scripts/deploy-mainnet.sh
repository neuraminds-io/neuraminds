#!/bin/bash
set -euo pipefail

# Polyguard Mainnet Deployment Script
# Staged deployment with CI/CD support

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log() { echo -e "${GREEN}[$(date +'%H:%M:%S')]${NC} $1"; }
warn() { echo -e "${YELLOW}[$(date +'%H:%M:%S')] WARNING:${NC} $1"; }
error() { echo -e "${RED}[$(date +'%H:%M:%S')] ERROR:${NC} $1"; exit 1; }

# Configuration
CLUSTER="${CLUSTER:-mainnet}"
KEYPAIR="${KEYPAIR:-~/.config/solana/id.json}"
RPC_URL="${RPC_URL:-https://api.mainnet-beta.solana.com}"
PRIORITY_FEE="${PRIORITY_FEE:-10000}"
CI_MODE="${CI:-false}"
DRY_RUN="${DRY_RUN:-false}"

# Program paths
ORDERBOOK_SO="target/deploy/polyguard_orderbook.so"

usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --skip-build      Skip building programs"
    echo "  --generate-keys   Generate new program keypairs"
    echo "  --rpc URL         Solana RPC URL"
    echo "  --keypair PATH    Deployer keypair path"
    echo "  --priority-fee N  Priority fee in microlamports"
    echo "  --ci              CI mode (non-interactive)"
    echo "  --dry-run         Show what would be deployed without deploying"
    echo "  --help            Show this help"
    exit 0
}

check_prereqs() {
    log "Checking prerequisites..."

    command -v solana >/dev/null 2>&1 || error "solana CLI not found"
    command -v anchor >/dev/null 2>&1 || error "anchor CLI not found"

    [[ -f "$KEYPAIR" ]] || error "Keypair not found: $KEYPAIR"

    log "Prerequisites OK"
}

check_balance() {
    local required=$1
    local balance
    balance=$(solana balance --keypair "$KEYPAIR" --url "$RPC_URL" | awk '{print $1}')

    log "Wallet balance: $balance SOL (need ~$required SOL)"

    if (( $(echo "$balance < $required" | bc -l) )); then
        error "Insufficient balance. Need at least $required SOL"
    fi

    echo "$balance"
}

deploy_program() {
    local name=$1
    local so_path=$2
    local keypair_path=$3

    log "Deploying $name..."
    log "  Binary: $so_path"

    local size size_kb
    size=$(ls -l "$so_path" | awk '{print $5}')
    size_kb=$((size / 1024))
    log "  Size: ${size_kb} KB"

    if [[ "$DRY_RUN" == "true" ]]; then
        log "  [DRY RUN] Would deploy $name"
        return 0
    fi

    log "  Creating buffer account..."
    local buffer_output buffer_address
    buffer_output=$(solana program write-buffer "$so_path" \
        --keypair "$KEYPAIR" \
        --url "$RPC_URL" \
        --with-compute-unit-price "$PRIORITY_FEE" \
        2>&1)

    buffer_address=$(echo "$buffer_output" | grep -oE '[1-9A-HJ-NP-Za-km-z]{32,44}' | head -1)

    if [[ -z "$buffer_address" ]]; then
        error "Failed to create buffer: $buffer_output"
    fi

    log "  Buffer created: $buffer_address"

    log "  Deploying from buffer..."
    solana program deploy \
        --keypair "$KEYPAIR" \
        --url "$RPC_URL" \
        --program-id "$keypair_path" \
        --buffer "$buffer_address" \
        --with-compute-unit-price "$PRIORITY_FEE" \
        --upgrade-authority "$KEYPAIR"

    log "  $name deployed successfully"

    log "  Closing buffer to reclaim rent..."
    solana program close "$buffer_address" \
        --keypair "$KEYPAIR" \
        --url "$RPC_URL" \
        --recipient "$(solana-keygen pubkey "$KEYPAIR")" \
        2>/dev/null || warn "Buffer may already be closed"

    log "  Buffer closed, rent reclaimed"
}

generate_keypairs() {
    log "Generating mainnet program keypairs..."

    local keypair_dir="target/deploy"
    mkdir -p "$keypair_dir"

    if [[ ! -f "$keypair_dir/polyguard_orderbook-keypair.json" ]] || [[ "${FORCE_NEW_KEYS:-false}" == "true" ]]; then
        solana-keygen new --no-bip39-passphrase -o "$keypair_dir/polyguard_orderbook-keypair.json" --force
        log "  Generated orderbook keypair"
    fi

    local orderbook_id
    orderbook_id=$(solana-keygen pubkey "$keypair_dir/polyguard_orderbook-keypair.json")

    log "  Orderbook program ID: $orderbook_id"
    echo "$orderbook_id" > "$keypair_dir/orderbook-program-id.txt"
}

build_programs() {
    log "Building programs..."

    anchor build -p polyguard-orderbook

    log "Programs built successfully"
    ls -lh target/deploy/*.so
}

verify_deployment() {
    local program_id=$1
    local name=$2

    log "Verifying $name deployment..."

    local info
    info=$(solana program show "$program_id" --url "$RPC_URL" 2>&1) || {
        warn "Could not verify $name deployment"
        return 1
    }

    echo "$info"
    log "$name verified"
}

main() {
    echo ""
    echo "==========================================="
    echo "  Polyguard Mainnet Deployment"
    echo "==========================================="
    echo ""

    local skip_build=false
    local generate_keys=false

    while [[ $# -gt 0 ]]; do
        case $1 in
            --skip-build) skip_build=true; shift ;;
            --generate-keys) generate_keys=true; shift ;;
            --rpc) RPC_URL="$2"; shift 2 ;;
            --keypair) KEYPAIR="$2"; shift 2 ;;
            --priority-fee) PRIORITY_FEE="$2"; shift 2 ;;
            --ci) CI_MODE=true; shift ;;
            --dry-run) DRY_RUN=true; shift ;;
            --help) usage ;;
            *) error "Unknown option: $1" ;;
        esac
    done

    check_prereqs

    solana config set --url "$RPC_URL" --keypair "$KEYPAIR"

    log "Cluster: $CLUSTER"
    log "RPC: $RPC_URL"
    log "Keypair: $KEYPAIR"
    log "CI Mode: $CI_MODE"
    log "Dry Run: $DRY_RUN"

    if [[ "$generate_keys" == "true" ]]; then
        FORCE_NEW_KEYS=true generate_keypairs
    fi

    if [[ "$skip_build" != "true" ]]; then
        build_programs
    fi

    if [[ ! -f "$ORDERBOOK_SO" ]]; then
        error "Program binary not found: $ORDERBOOK_SO"
    fi

    check_balance 5

    local orderbook_id
    orderbook_id=$(solana-keygen pubkey target/deploy/polyguard_orderbook-keypair.json)

    echo ""
    log "Deployment Plan:"
    log "  1. Deploy orderbook program ($orderbook_id)"
    log "  2. Reclaim buffer rent"
    log "  3. Verify deployment"
    echo ""

    if [[ "$CI_MODE" != "true" && "$DRY_RUN" != "true" ]]; then
        read -p "Continue with deployment? [y/N] " -n 1 -r
        echo ""
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            log "Deployment cancelled"
            exit 0
        fi
    fi

    log "=== Deploying Orderbook Program ==="
    deploy_program "polyguard-orderbook" "$ORDERBOOK_SO" "target/deploy/polyguard_orderbook-keypair.json"

    if [[ "$DRY_RUN" != "true" ]]; then
        verify_deployment "$orderbook_id" "polyguard-orderbook"
    fi

    local final_balance
    final_balance=$(solana balance --keypair "$KEYPAIR" --url "$RPC_URL" | awk '{print $1}')

    echo ""
    echo "==========================================="
    echo "  Deployment Complete!"
    echo "==========================================="
    echo ""
    log "Orderbook Program: $orderbook_id"
    log "Final Balance:     $final_balance SOL"
    echo ""

    if [[ "$DRY_RUN" != "true" ]]; then
        log "Next steps:"
        log "  1. Update Anchor.toml with program ID"
        log "  2. Update app config"
        log "  3. Initialize program configs"
        log "  4. Deploy backend API"
    fi
    echo ""

    echo "$orderbook_id" > target/deploy/deployed-program-id.txt
}

main "$@"
