#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EVM_DIR="${ROOT_DIR}/evm"
ARTIFACTS_DIR="${ROOT_DIR}/docs/reports/fuzz"

RUNS=2000
ITERATIONS=3
MATCH_CONTRACT=""
MATCH_TEST=""
CI_MODE=0

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
  echo -e "${BLUE}[info]${NC} $1"
}

log_success() {
  echo -e "${GREEN}[pass]${NC} $1"
}

log_warn() {
  echo -e "${YELLOW}[warn]${NC} $1"
}

log_error() {
  echo -e "${RED}[fail]${NC} $1"
}

usage() {
  cat <<USAGE
Base/EVM fuzz stress campaign

Usage: $0 [options]

Options:
  --runs <n>              Fuzz/invariant runs per forge invocation (default: ${RUNS})
  --iterations <n>        Number of randomized forge invocations (default: ${ITERATIONS})
  --match-contract <name> Limit to matching contract tests
  --match-test <regex>    Limit to matching test names
  --ci                    CI mode: single iteration, fail fast
  --help                  Show this help

Examples:
  $0
  $0 --runs 5000 --iterations 5
  $0 --match-contract OrderBookTest --runs 10000
USAGE
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --runs)
        RUNS="$2"
        shift 2
        ;;
      --iterations)
        ITERATIONS="$2"
        shift 2
        ;;
      --match-contract)
        MATCH_CONTRACT="$2"
        shift 2
        ;;
      --match-test)
        MATCH_TEST="$2"
        shift 2
        ;;
      --ci)
        CI_MODE=1
        ITERATIONS=1
        shift
        ;;
      --help)
        usage
        exit 0
        ;;
      *)
        log_error "Unknown option: $1"
        usage
        exit 1
        ;;
    esac
  done
}

check_prerequisites() {
  if ! command -v forge >/dev/null 2>&1; then
    log_error "forge not found (install Foundry first)"
    exit 1
  fi
  if [[ ! -d "${EVM_DIR}" ]]; then
    log_error "missing EVM directory: ${EVM_DIR}"
    exit 1
  fi
}

build_forge_cmd() {
  local -a cmd
  cmd=(forge test --root "${EVM_DIR}")

  if [[ -n "${MATCH_CONTRACT}" ]]; then
    cmd+=(--match-contract "${MATCH_CONTRACT}")
  fi
  if [[ -n "${MATCH_TEST}" ]]; then
    cmd+=(--match-test "${MATCH_TEST}")
  fi
  if [[ "${CI_MODE}" -eq 1 ]]; then
    cmd+=(--fail-fast)
  fi

  printf '%q ' "${cmd[@]}"
}

run_iteration() {
  local index="$1"
  local seed
  seed="$(od -An -N4 -tu4 /dev/urandom | tr -d '[:space:]')"
  local log_file="${ARTIFACTS_DIR}/run-${index}.log"
  local cmd
  cmd="$(build_forge_cmd)"

  log_info "iteration ${index}/${ITERATIONS} seed=${seed} runs=${RUNS}"
  log_info "command: ${cmd}"

  set +e
  FOUNDRY_FUZZ_RUNS="${RUNS}" \
  FOUNDRY_INVARIANT_RUNS="${RUNS}" \
    eval "${cmd} --fuzz-seed ${seed}" | tee "${log_file}"
  local exit_code=$?
  set -e

  if [[ ${exit_code} -ne 0 ]]; then
    log_error "iteration ${index} failed (see ${log_file})"
    return "${exit_code}"
  fi

  log_success "iteration ${index} passed"
  return 0
}

main() {
  parse_args "$@"
  check_prerequisites

  mkdir -p "${ARTIFACTS_DIR}"
  log_info "artifact directory: ${ARTIFACTS_DIR}"

  local failed=0
  local i
  for ((i = 1; i <= ITERATIONS; i++)); do
    if ! run_iteration "${i}"; then
      failed=$((failed + 1))
      if [[ "${CI_MODE}" -eq 1 ]]; then
        break
      fi
    fi
  done

  if [[ ${failed} -gt 0 ]]; then
    log_error "fuzz stress campaign failed (${failed}/${ITERATIONS} failed)"
    exit 1
  fi

  log_success "fuzz stress campaign completed (${ITERATIONS}/${ITERATIONS} passed)"
}

main "$@"
