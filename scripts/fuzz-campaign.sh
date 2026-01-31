#!/usr/bin/env bash
#
# Fuzz Campaign Runner for Polyguard Smart Contracts
#
# Runs all fuzz targets with configurable duration and parallelism.
# Use for extended security testing before releases.
#
# Usage:
#   ./scripts/fuzz-campaign.sh                    # Default: 5 min per target
#   ./scripts/fuzz-campaign.sh --duration 3600    # 1 hour per target
#   ./scripts/fuzz-campaign.sh --target orderbook_operations  # Single target
#   ./scripts/fuzz-campaign.sh --ci               # CI mode: shorter runs, fail on crash
#   ./scripts/fuzz-campaign.sh --coverage         # Generate coverage report

set -euo pipefail

FUZZ_DIR="programs/polyguard-orderbook/fuzz"
ARTIFACTS_DIR="$FUZZ_DIR/artifacts"
CORPUS_DIR="$FUZZ_DIR/corpus"
COVERAGE_DIR="$FUZZ_DIR/coverage"

# Default settings
DURATION=300  # 5 minutes per target
JOBS=$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4)
CI_MODE=false
COVERAGE_MODE=false
SINGLE_TARGET=""

# All available fuzz targets
TARGETS=(
    "orderbook_operations"
    "price_matching"
    "arithmetic_safety"
    "settlement"
    "redemption"
    "market_resolution"
    "fee_calculations"
)

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[FAIL]${NC} $1"
}

print_usage() {
    cat << EOF
Polyguard Fuzz Campaign Runner

Usage: $0 [OPTIONS]

Options:
    --duration SECONDS   Time to run each fuzz target (default: 300)
    --target NAME        Run only the specified target
    --jobs N             Number of parallel jobs (default: auto-detect)
    --ci                 CI mode: 60s per target, exit 1 on any crash
    --coverage           Generate coverage report after fuzzing
    --list               List all available fuzz targets
    --help               Show this help message

Examples:
    # Run all targets for 5 minutes each (default)
    $0

    # Extended campaign: 1 hour per target
    $0 --duration 3600

    # Focus on specific target for deep testing
    $0 --target arithmetic_safety --duration 7200

    # CI integration
    $0 --ci

    # Generate coverage after fuzzing
    $0 --duration 60 --coverage

Available Targets:
    orderbook_operations  - Order placement and cancellation sequences
    price_matching        - Bid/ask matching logic
    arithmetic_safety     - Integer overflow/underflow detection
    settlement            - Trade settlement and refunds
    redemption            - Token redemption after market resolution
    market_resolution     - Oracle threshold evaluation
    fee_calculations      - Fee arithmetic and accumulation
EOF
}

parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --duration)
                DURATION="$2"
                shift 2
                ;;
            --target)
                SINGLE_TARGET="$2"
                shift 2
                ;;
            --jobs)
                JOBS="$2"
                shift 2
                ;;
            --ci)
                CI_MODE=true
                DURATION=60
                shift
                ;;
            --coverage)
                COVERAGE_MODE=true
                shift
                ;;
            --list)
                echo "Available fuzz targets:"
                for t in "${TARGETS[@]}"; do
                    echo "  - $t"
                done
                exit 0
                ;;
            --help)
                print_usage
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                print_usage
                exit 1
                ;;
        esac
    done
}

check_prerequisites() {
    log_info "Checking prerequisites..."

    if ! command -v cargo &> /dev/null; then
        log_error "cargo not found. Install Rust: https://rustup.rs"
        exit 1
    fi

    if ! rustup show | grep -q "nightly"; then
        log_warn "Nightly toolchain not found. Installing..."
        rustup install nightly
    fi

    if ! cargo +nightly fuzz --version &> /dev/null; then
        log_warn "cargo-fuzz not found. Installing..."
        cargo install cargo-fuzz
    fi

    if [[ ! -d "$FUZZ_DIR" ]]; then
        log_error "Fuzz directory not found: $FUZZ_DIR"
        exit 1
    fi

    log_success "Prerequisites OK"
}

run_fuzz_target() {
    local target=$1
    local duration=$2
    local artifact_dir="$ARTIFACTS_DIR/$target"
    local corpus_dir="$CORPUS_DIR/$target"

    mkdir -p "$artifact_dir" "$corpus_dir"

    log_info "Running: $target (${duration}s)"

    local start_time=$(date +%s)

    # Run fuzzer
    set +e
    cd "$FUZZ_DIR"
    cargo +nightly fuzz run "$target" \
        --jobs "$JOBS" \
        -- \
        -max_total_time="$duration" \
        -artifact_prefix="$artifact_dir/" \
        2>&1 | tee "$artifact_dir/fuzz.log"
    local exit_code=$?
    cd - > /dev/null
    set -e

    local end_time=$(date +%s)
    local elapsed=$((end_time - start_time))

    # Check for crashes
    local crashes=$(find "$artifact_dir" -name "crash-*" 2>/dev/null | wc -l | tr -d ' ')
    local ooms=$(find "$artifact_dir" -name "oom-*" 2>/dev/null | wc -l | tr -d ' ')
    local timeouts=$(find "$artifact_dir" -name "timeout-*" 2>/dev/null | wc -l | tr -d ' ')

    if [[ $crashes -gt 0 ]] || [[ $ooms -gt 0 ]]; then
        log_error "$target: $crashes crashes, $ooms OOMs, $timeouts timeouts (${elapsed}s)"
        return 1
    elif [[ $exit_code -ne 0 ]]; then
        log_warn "$target: exited with code $exit_code (${elapsed}s)"
        return $exit_code
    else
        log_success "$target: no issues found (${elapsed}s)"
        return 0
    fi
}

generate_coverage() {
    log_info "Generating coverage report..."

    mkdir -p "$COVERAGE_DIR"

    cd "$FUZZ_DIR"
    for target in "${TARGETS[@]}"; do
        if [[ -d "$CORPUS_DIR/$target" ]] && [[ -n "$(ls -A "$CORPUS_DIR/$target" 2>/dev/null)" ]]; then
            log_info "Coverage for: $target"
            cargo +nightly fuzz coverage "$target" "$CORPUS_DIR/$target" 2>&1 || true
        fi
    done
    cd - > /dev/null

    log_success "Coverage report generated in $COVERAGE_DIR"
}

print_summary() {
    local total=$1
    local passed=$2
    local failed=$3

    echo ""
    echo "======================================"
    echo "           FUZZ CAMPAIGN SUMMARY"
    echo "======================================"
    echo ""
    echo "Duration per target: ${DURATION}s"
    echo "Parallel jobs:       $JOBS"
    echo ""
    echo "Targets run:         $total"
    echo "Passed:              $passed"
    echo "Failed:              $failed"
    echo ""

    if [[ $failed -gt 0 ]]; then
        log_error "Some fuzz targets found issues!"
        echo ""
        echo "Check artifacts in: $ARTIFACTS_DIR"
        echo ""
        echo "To reproduce a crash:"
        echo "  cargo +nightly fuzz run <target> <artifact_file>"
        return 1
    else
        log_success "All fuzz targets passed!"
        return 0
    fi
}

main() {
    parse_args "$@"
    check_prerequisites

    echo ""
    echo "======================================"
    echo "     POLYGUARD FUZZ CAMPAIGN"
    echo "======================================"
    echo ""
    echo "Duration:  ${DURATION}s per target"
    echo "Jobs:      $JOBS"
    echo "CI mode:   $CI_MODE"
    echo "Coverage:  $COVERAGE_MODE"
    echo ""

    mkdir -p "$ARTIFACTS_DIR" "$CORPUS_DIR"

    local total=0
    local passed=0
    local failed=0
    local targets_to_run=()

    if [[ -n "$SINGLE_TARGET" ]]; then
        targets_to_run=("$SINGLE_TARGET")
    else
        targets_to_run=("${TARGETS[@]}")
    fi

    for target in "${targets_to_run[@]}"; do
        total=$((total + 1))

        if run_fuzz_target "$target" "$DURATION"; then
            passed=$((passed + 1))
        else
            failed=$((failed + 1))
            if $CI_MODE; then
                log_error "CI mode: failing fast on first crash"
                print_summary "$total" "$passed" "$failed"
                exit 1
            fi
        fi
    done

    if $COVERAGE_MODE; then
        generate_coverage
    fi

    print_summary "$total" "$passed" "$failed"
}

main "$@"
