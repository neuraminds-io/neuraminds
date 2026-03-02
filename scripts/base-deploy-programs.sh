#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NETWORK=""
ACCOUNT=""
DRY_RUN=0
VERIFY_MODE="auto"
PASSWORD_FILE=""
TMP_PASSWORD_FILE=""

usage() {
  cat <<USAGE
Usage: scripts/base-deploy-programs.sh --network sepolia|mainnet [options]

Options:
  --account <alias>     Foundry account alias (default: FOUNDRY_ACCOUNT or base-deployer)
  --password-file <path>  Keystore password file (default: keys/base-keystore-password.local)
  --dry-run             Run simulation only (no broadcast)
  --verify              Force BaseScan verification
  --no-verify           Disable BaseScan verification
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --network)
      NETWORK="${2:-}"
      shift 2
      ;;
    --network=*)
      NETWORK="${1#*=}"
      shift
      ;;
    --account)
      ACCOUNT="${2:-}"
      shift 2
      ;;
    --account=*)
      ACCOUNT="${1#*=}"
      shift
      ;;
    --password-file)
      PASSWORD_FILE="${2:-}"
      shift 2
      ;;
    --password-file=*)
      PASSWORD_FILE="${1#*=}"
      shift
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    --verify)
      VERIFY_MODE="force"
      shift
      ;;
    --no-verify)
      VERIFY_MODE="off"
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

if [[ "$NETWORK" != "sepolia" && "$NETWORK" != "mainnet" ]]; then
  echo "--network must be sepolia or mainnet" >&2
  exit 1
fi

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

ACCOUNT="${ACCOUNT:-${FOUNDRY_ACCOUNT:-base-deployer}}"
PASSWORD_FILE="${PASSWORD_FILE:-${BASE_KEYSTORE_PASSWORD_FILE:-$ROOT_DIR/keys/base-keystore-password.local}}"

if [[ "$NETWORK" == "sepolia" ]]; then
  CHAIN_ID="84532"
  RPC_URL="${BASE_SEPOLIA_RPC_URL:-https://sepolia.base.org}"
else
  CHAIN_ID="8453"
  RPC_URL="${BASE_RPC_URL:-https://mainnet.base.org}"
fi

if [[ -z "$RPC_URL" ]]; then
  echo "RPC URL is missing for $NETWORK" >&2
  exit 1
fi

if [[ ! -f "$PASSWORD_FILE" ]]; then
  if [[ -n "${BASE_KEYSTORE_PASSWORD:-}" ]]; then
    TMP_PASSWORD_FILE="$(mktemp)"
    trap '[[ -n "$TMP_PASSWORD_FILE" ]] && rm -f "$TMP_PASSWORD_FILE"' EXIT
    printf '%s' "$BASE_KEYSTORE_PASSWORD" > "$TMP_PASSWORD_FILE"
    PASSWORD_FILE="$TMP_PASSWORD_FILE"
  else
    echo "Password file not found and BASE_KEYSTORE_PASSWORD is empty." >&2
    exit 1
  fi
fi

KEYSTORE_DIR="${FOUNDRY_KEYSTORE_DIR:-$HOME/.foundry/keystores}"
KEYSTORE_PATH="${KEYSTORE_DIR}/${ACCOUNT}"

if [[ ! -f "$KEYSTORE_PATH" ]]; then
  echo "Keystore not found for account '$ACCOUNT': $KEYSTORE_PATH" >&2
  exit 1
fi

if ! cast wallet address --keystore "$KEYSTORE_PATH" --password-file "$PASSWORD_FILE" >/dev/null 2>&1; then
  echo "Unable to unlock keystore '$KEYSTORE_PATH' with password file '$PASSWORD_FILE'." >&2
  exit 1
fi

FORGE_CMD=(
  forge script script/DeployPrograms.s.sol:DeployProgramsScript
  --rpc-url "$RPC_URL"
  --keystore "$KEYSTORE_PATH"
  --password-file "$PASSWORD_FILE"
)

if [[ "$DRY_RUN" -eq 0 ]]; then
  FORGE_CMD+=(--broadcast)
fi

if [[ "$VERIFY_MODE" == "force" ]]; then
  FORGE_CMD+=(--verify)
elif [[ "$VERIFY_MODE" == "auto" && "$DRY_RUN" -eq 0 && -n "${BASESCAN_API_KEY:-}" ]]; then
  FORGE_CMD+=(--verify)
fi

(
  cd "$ROOT_DIR/evm"
  "${FORGE_CMD[@]}"
)

if [[ "$DRY_RUN" -eq 1 ]]; then
  RUN_FILE_REL="evm/broadcast/DeployPrograms.s.sol/${CHAIN_ID}/dry-run/run-latest.json"
else
  RUN_FILE_REL="evm/broadcast/DeployPrograms.s.sol/${CHAIN_ID}/run-latest.json"
fi
RUN_FILE="$ROOT_DIR/$RUN_FILE_REL"

if [[ ! -f "$RUN_FILE" ]]; then
  echo "Expected broadcast file not found: $RUN_FILE" >&2
  exit 1
fi

MARKET_CORE_ADDRESS="$(jq -r '.transactions[] | select(.transactionType=="CREATE" and .contractName=="MarketCore") | .contractAddress' "$RUN_FILE" | tail -n 1)"
ORDER_BOOK_ADDRESS="$(jq -r '.transactions[] | select(.transactionType=="CREATE" and .contractName=="OrderBook") | .contractAddress' "$RUN_FILE" | tail -n 1)"
COLLATERAL_VAULT_ADDRESS="$(jq -r '.transactions[] | select(.transactionType=="CREATE" and .contractName=="CollateralVault") | .contractAddress' "$RUN_FILE" | tail -n 1)"
AGENT_RUNTIME_ADDRESS="$(jq -r '.transactions[] | select(.transactionType=="CREATE" and .contractName=="AgentRuntime") | .contractAddress' "$RUN_FILE" | tail -n 1)"
AGENT_IDENTITY_REGISTRY_ADDRESS="$(jq -r '.transactions[] | select(.transactionType=="CREATE" and .contractName=="AgentIdentityRegistry") | .contractAddress' "$RUN_FILE" | tail -n 1)"
AGENT_REPUTATION_REGISTRY_ADDRESS="$(jq -r '.transactions[] | select(.transactionType=="CREATE" and .contractName=="AgentReputationRegistry") | .contractAddress' "$RUN_FILE" | tail -n 1)"
ERC8004_IDENTITY_REGISTRY_ADDRESS="$(jq -r '.transactions[] | select(.transactionType=="CREATE" and .contractName=="ERC8004IdentityRegistry") | .contractAddress' "$RUN_FILE" | tail -n 1)"
ERC8004_REPUTATION_REGISTRY_ADDRESS="$(jq -r '.transactions[] | select(.transactionType=="CREATE" and .contractName=="ERC8004ReputationRegistry") | .contractAddress' "$RUN_FILE" | tail -n 1)"

if [[ -z "$MARKET_CORE_ADDRESS" || "$MARKET_CORE_ADDRESS" == "null" || -z "$ORDER_BOOK_ADDRESS" || "$ORDER_BOOK_ADDRESS" == "null" || -z "$COLLATERAL_VAULT_ADDRESS" || "$COLLATERAL_VAULT_ADDRESS" == "null" || -z "$AGENT_RUNTIME_ADDRESS" || "$AGENT_RUNTIME_ADDRESS" == "null" || -z "$AGENT_IDENTITY_REGISTRY_ADDRESS" || "$AGENT_IDENTITY_REGISTRY_ADDRESS" == "null" || -z "$AGENT_REPUTATION_REGISTRY_ADDRESS" || "$AGENT_REPUTATION_REGISTRY_ADDRESS" == "null" || -z "$ERC8004_IDENTITY_REGISTRY_ADDRESS" || "$ERC8004_IDENTITY_REGISTRY_ADDRESS" == "null" || -z "$ERC8004_REPUTATION_REGISTRY_ADDRESS" || "$ERC8004_REPUTATION_REGISTRY_ADDRESS" == "null" ]]; then
  echo "Could not extract deployed contract addresses from $RUN_FILE" >&2
  exit 1
fi

REPORT_DIR="$ROOT_DIR/docs/reports"
mkdir -p "$REPORT_DIR"
TIMESTAMP="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
if [[ "$DRY_RUN" -eq 1 ]]; then
  REPORT_JSON="$REPORT_DIR/base-programs-deploy-${NETWORK}-dry-run.json"
  REPORT_ENV="$REPORT_DIR/base-programs-deploy-${NETWORK}-dry-run.env"
else
  REPORT_JSON="$REPORT_DIR/base-programs-deploy-${NETWORK}.json"
  REPORT_ENV="$REPORT_DIR/base-programs-deploy-${NETWORK}.env"
fi

jq -n \
  --arg timestamp "$TIMESTAMP" \
  --arg network "$NETWORK" \
  --arg chainId "$CHAIN_ID" \
  --arg rpcUrl "$RPC_URL" \
  --arg account "$ACCOUNT" \
  --arg dryRun "$DRY_RUN" \
  --arg marketCore "$MARKET_CORE_ADDRESS" \
  --arg orderBook "$ORDER_BOOK_ADDRESS" \
  --arg collateralVault "$COLLATERAL_VAULT_ADDRESS" \
  --arg agentRuntime "$AGENT_RUNTIME_ADDRESS" \
  --arg identityRegistry "$AGENT_IDENTITY_REGISTRY_ADDRESS" \
  --arg reputationRegistry "$AGENT_REPUTATION_REGISTRY_ADDRESS" \
  --arg erc8004IdentityRegistry "$ERC8004_IDENTITY_REGISTRY_ADDRESS" \
  --arg erc8004ReputationRegistry "$ERC8004_REPUTATION_REGISTRY_ADDRESS" \
  --arg runFile "$RUN_FILE_REL" \
  '{
    timestamp: $timestamp,
    network: $network,
    chainId: ($chainId | tonumber),
    rpcUrl: $rpcUrl,
    account: $account,
    dryRun: ($dryRun == "1"),
    contracts: {
      marketCore: $marketCore,
      orderBook: $orderBook,
      collateralVault: $collateralVault,
      agentRuntime: $agentRuntime,
      agentIdentityRegistry: $identityRegistry,
      agentReputationRegistry: $reputationRegistry,
      erc8004IdentityRegistry: $erc8004IdentityRegistry,
      erc8004ReputationRegistry: $erc8004ReputationRegistry
    },
    runFile: $runFile
  }' > "$REPORT_JSON"

cat > "$REPORT_ENV" <<ENV
# Generated $TIMESTAMP
MARKET_CORE_ADDRESS=$MARKET_CORE_ADDRESS
ORDER_BOOK_ADDRESS=$ORDER_BOOK_ADDRESS
COLLATERAL_VAULT_ADDRESS=$COLLATERAL_VAULT_ADDRESS
AGENT_RUNTIME_ADDRESS=$AGENT_RUNTIME_ADDRESS
AGENT_IDENTITY_REGISTRY_ADDRESS=$AGENT_IDENTITY_REGISTRY_ADDRESS
AGENT_REPUTATION_REGISTRY_ADDRESS=$AGENT_REPUTATION_REGISTRY_ADDRESS
ERC8004_IDENTITY_REGISTRY_ADDRESS=$ERC8004_IDENTITY_REGISTRY_ADDRESS
ERC8004_REPUTATION_REGISTRY_ADDRESS=$ERC8004_REPUTATION_REGISTRY_ADDRESS
ENV

echo ""
echo "Deploy programs complete ($NETWORK)"
echo "MarketCore: $MARKET_CORE_ADDRESS"
echo "OrderBook: $ORDER_BOOK_ADDRESS"
echo "CollateralVault: $COLLATERAL_VAULT_ADDRESS"
echo "AgentRuntime: $AGENT_RUNTIME_ADDRESS"
echo "AgentIdentityRegistry: $AGENT_IDENTITY_REGISTRY_ADDRESS"
echo "AgentReputationRegistry: $AGENT_REPUTATION_REGISTRY_ADDRESS"
echo "ERC8004IdentityRegistry: $ERC8004_IDENTITY_REGISTRY_ADDRESS"
echo "ERC8004ReputationRegistry: $ERC8004_REPUTATION_REGISTRY_ADDRESS"
echo "Report: $REPORT_JSON"
echo "Env snippet: $REPORT_ENV"

if [[ "$DRY_RUN" -eq 0 ]]; then
  for required_var in BASE_ADMIN BASE_MARKET_CREATOR BASE_RESOLVER BASE_PAUSER BASE_OPERATOR; do
    if [[ -z "${!required_var:-}" ]]; then
      echo "Missing required variable for role verification: $required_var" >&2
      exit 1
    fi
  done

  wait_for_code() {
    local contract="$1"
    local label="$2"
    local attempts=25
    local sleep_seconds=2

    for ((i=1; i<=attempts; i++)); do
      local code
      code="$(cast code --rpc-url "$RPC_URL" "$contract" 2>/dev/null || true)"
      if [[ -n "$code" && "$code" != "0x" ]]; then
        return 0
      fi

      if [[ "$i" -lt "$attempts" ]]; then
        sleep "$sleep_seconds"
      fi
    done

    echo "Contract code not available yet for $label at $contract" >&2
    return 1
  }

  wait_for_code "$MARKET_CORE_ADDRESS" "MarketCore"
  wait_for_code "$ORDER_BOOK_ADDRESS" "OrderBook"
  wait_for_code "$COLLATERAL_VAULT_ADDRESS" "CollateralVault"
  wait_for_code "$AGENT_RUNTIME_ADDRESS" "AgentRuntime"
  wait_for_code "$AGENT_IDENTITY_REGISTRY_ADDRESS" "AgentIdentityRegistry"
  wait_for_code "$AGENT_REPUTATION_REGISTRY_ADDRESS" "AgentReputationRegistry"
  wait_for_code "$ERC8004_IDENTITY_REGISTRY_ADDRESS" "ERC8004IdentityRegistry"
  wait_for_code "$ERC8004_REPUTATION_REGISTRY_ADDRESS" "ERC8004ReputationRegistry"

  ROLE_REPORT="$REPORT_DIR/base-programs-roles-${NETWORK}.json"
  REPUTATION_ORACLE_ADDRESS="${BASE_REPUTATION_ORACLE:-${BASE_OPERATOR:-}}"
  if [[ -z "$REPUTATION_ORACLE_ADDRESS" ]]; then
    echo "Missing required variable for role verification: BASE_REPUTATION_ORACLE or BASE_OPERATOR" >&2
    exit 1
  fi
  ERC8004_ISSUER_ADDRESS="${BASE_IDENTITY_ISSUER:-${BASE_ADMIN:-}}"
  ERC8004_ATTESTER_ADDRESS="${BASE_REPUTATION_ATTESTER:-$REPUTATION_ORACLE_ADDRESS}"

  DEFAULT_ADMIN_ROLE="0x0000000000000000000000000000000000000000000000000000000000000000"
  MARKET_CREATOR_ROLE="$(cast call --rpc-url "$RPC_URL" "$MARKET_CORE_ADDRESS" "MARKET_CREATOR_ROLE()(bytes32)")"
  RESOLVER_ROLE="$(cast call --rpc-url "$RPC_URL" "$MARKET_CORE_ADDRESS" "RESOLVER_ROLE()(bytes32)")"
  MARKET_PAUSER_ROLE="$(cast call --rpc-url "$RPC_URL" "$MARKET_CORE_ADDRESS" "PAUSER_ROLE()(bytes32)")"
  ORDERBOOK_PAUSER_ROLE="$(cast call --rpc-url "$RPC_URL" "$ORDER_BOOK_ADDRESS" "PAUSER_ROLE()(bytes32)")"
  AGENT_RUNTIME_ROLE="$(cast call --rpc-url "$RPC_URL" "$ORDER_BOOK_ADDRESS" "AGENT_RUNTIME_ROLE()(bytes32)")"
  OPERATOR_ROLE="$(cast call --rpc-url "$RPC_URL" "$COLLATERAL_VAULT_ADDRESS" "OPERATOR_ROLE()(bytes32)")"
  VAULT_PAUSER_ROLE="$(cast call --rpc-url "$RPC_URL" "$COLLATERAL_VAULT_ADDRESS" "PAUSER_ROLE()(bytes32)")"
  RUNTIME_PAUSER_ROLE="$(cast call --rpc-url "$RPC_URL" "$AGENT_RUNTIME_ADDRESS" "PAUSER_ROLE()(bytes32)")"
  IDENTITY_PAUSER_ROLE="$(cast call --rpc-url "$RPC_URL" "$AGENT_IDENTITY_REGISTRY_ADDRESS" "PAUSER_ROLE()(bytes32)")"
  IDENTITY_REGISTRAR_ROLE="$(cast call --rpc-url "$RPC_URL" "$AGENT_IDENTITY_REGISTRY_ADDRESS" "REGISTRAR_ROLE()(bytes32)")"
  REPUTATION_PAUSER_ROLE="$(cast call --rpc-url "$RPC_URL" "$AGENT_REPUTATION_REGISTRY_ADDRESS" "PAUSER_ROLE()(bytes32)")"
  REPUTATION_ORACLE_ROLE="$(cast call --rpc-url "$RPC_URL" "$AGENT_REPUTATION_REGISTRY_ADDRESS" "ORACLE_ROLE()(bytes32)")"
  ERC8004_IDENTITY_PAUSER_ROLE="$(cast call --rpc-url "$RPC_URL" "$ERC8004_IDENTITY_REGISTRY_ADDRESS" "PAUSER_ROLE()(bytes32)")"
  ERC8004_IDENTITY_ISSUER_ROLE="$(cast call --rpc-url "$RPC_URL" "$ERC8004_IDENTITY_REGISTRY_ADDRESS" "ISSUER_ROLE()(bytes32)")"
  ERC8004_REPUTATION_PAUSER_ROLE="$(cast call --rpc-url "$RPC_URL" "$ERC8004_REPUTATION_REGISTRY_ADDRESS" "PAUSER_ROLE()(bytes32)")"
  ERC8004_REPUTATION_ATTESTER_ROLE="$(cast call --rpc-url "$RPC_URL" "$ERC8004_REPUTATION_REGISTRY_ADDRESS" "ATTESTER_ROLE()(bytes32)")"

  has_role() {
    local contract="$1"
    local role="$2"
    local actor="$3"
    cast call --rpc-url "$RPC_URL" "$contract" "hasRole(bytes32,address)(bool)" "$role" "$actor"
  }

  ADMIN_HAS_DEFAULT="$(has_role "$MARKET_CORE_ADDRESS" "$DEFAULT_ADMIN_ROLE" "$BASE_ADMIN")"
  CREATOR_HAS_ROLE="$(has_role "$MARKET_CORE_ADDRESS" "$MARKET_CREATOR_ROLE" "$BASE_MARKET_CREATOR")"
  RESOLVER_HAS_ROLE="$(has_role "$MARKET_CORE_ADDRESS" "$RESOLVER_ROLE" "$BASE_RESOLVER")"
  PAUSER_MARKET_HAS_ROLE="$(has_role "$MARKET_CORE_ADDRESS" "$MARKET_PAUSER_ROLE" "$BASE_PAUSER")"

  ADMIN_ORDERBOOK="$(has_role "$ORDER_BOOK_ADDRESS" "$DEFAULT_ADMIN_ROLE" "$BASE_ADMIN")"
  PAUSER_ORDERBOOK_HAS_ROLE="$(has_role "$ORDER_BOOK_ADDRESS" "$ORDERBOOK_PAUSER_ROLE" "$BASE_PAUSER")"
  RUNTIME_CAN_PLACE_FOR="$(has_role "$ORDER_BOOK_ADDRESS" "$AGENT_RUNTIME_ROLE" "$AGENT_RUNTIME_ADDRESS")"
  OPTIONAL_RUNTIME_OPERATOR_HAS_ROLE="n/a"
  if [[ -n "${BASE_AGENT_RUNTIME_OPERATOR:-}" ]]; then
    OPTIONAL_RUNTIME_OPERATOR_HAS_ROLE="$(has_role "$ORDER_BOOK_ADDRESS" "$AGENT_RUNTIME_ROLE" "$BASE_AGENT_RUNTIME_OPERATOR")"
  fi

  ADMIN_VAULT="$(has_role "$COLLATERAL_VAULT_ADDRESS" "$DEFAULT_ADMIN_ROLE" "$BASE_ADMIN")"
  OPERATOR_HAS_ROLE="$(has_role "$COLLATERAL_VAULT_ADDRESS" "$OPERATOR_ROLE" "$BASE_OPERATOR")"
  ORDERBOOK_OPERATOR_HAS_ROLE="$(has_role "$COLLATERAL_VAULT_ADDRESS" "$OPERATOR_ROLE" "$ORDER_BOOK_ADDRESS")"
  PAUSER_VAULT_HAS_ROLE="$(has_role "$COLLATERAL_VAULT_ADDRESS" "$VAULT_PAUSER_ROLE" "$BASE_PAUSER")"

  ADMIN_RUNTIME="$(has_role "$AGENT_RUNTIME_ADDRESS" "$DEFAULT_ADMIN_ROLE" "$BASE_ADMIN")"
  PAUSER_RUNTIME_HAS_ROLE="$(has_role "$AGENT_RUNTIME_ADDRESS" "$RUNTIME_PAUSER_ROLE" "$BASE_PAUSER")"
  RUNTIME_IDENTITY_REGISTRY="$(cast call --rpc-url "$RPC_URL" "$AGENT_RUNTIME_ADDRESS" "identityRegistry()(address)")"

  ADMIN_IDENTITY="$(has_role "$AGENT_IDENTITY_REGISTRY_ADDRESS" "$DEFAULT_ADMIN_ROLE" "$BASE_ADMIN")"
  PAUSER_IDENTITY_HAS_ROLE="$(has_role "$AGENT_IDENTITY_REGISTRY_ADDRESS" "$IDENTITY_PAUSER_ROLE" "$BASE_PAUSER")"
  REGISTRAR_IDENTITY_HAS_ROLE="$(has_role "$AGENT_IDENTITY_REGISTRY_ADDRESS" "$IDENTITY_REGISTRAR_ROLE" "$AGENT_RUNTIME_ADDRESS")"

  ADMIN_REPUTATION="$(has_role "$AGENT_REPUTATION_REGISTRY_ADDRESS" "$DEFAULT_ADMIN_ROLE" "$BASE_ADMIN")"
  PAUSER_REPUTATION_HAS_ROLE="$(has_role "$AGENT_REPUTATION_REGISTRY_ADDRESS" "$REPUTATION_PAUSER_ROLE" "$BASE_PAUSER")"
  ORACLE_REPUTATION_HAS_ROLE="$(has_role "$AGENT_REPUTATION_REGISTRY_ADDRESS" "$REPUTATION_ORACLE_ROLE" "$REPUTATION_ORACLE_ADDRESS")"

  ADMIN_ERC8004_IDENTITY="$(has_role "$ERC8004_IDENTITY_REGISTRY_ADDRESS" "$DEFAULT_ADMIN_ROLE" "$BASE_ADMIN")"
  PAUSER_ERC8004_IDENTITY_HAS_ROLE="$(has_role "$ERC8004_IDENTITY_REGISTRY_ADDRESS" "$ERC8004_IDENTITY_PAUSER_ROLE" "$BASE_PAUSER")"
  ISSUER_ERC8004_IDENTITY_HAS_ROLE="$(has_role "$ERC8004_IDENTITY_REGISTRY_ADDRESS" "$ERC8004_IDENTITY_ISSUER_ROLE" "$ERC8004_ISSUER_ADDRESS")"

  ADMIN_ERC8004_REPUTATION="$(has_role "$ERC8004_REPUTATION_REGISTRY_ADDRESS" "$DEFAULT_ADMIN_ROLE" "$BASE_ADMIN")"
  PAUSER_ERC8004_REPUTATION_HAS_ROLE="$(has_role "$ERC8004_REPUTATION_REGISTRY_ADDRESS" "$ERC8004_REPUTATION_PAUSER_ROLE" "$BASE_PAUSER")"
  ATTESTER_ERC8004_REPUTATION_HAS_ROLE="$(has_role "$ERC8004_REPUTATION_REGISTRY_ADDRESS" "$ERC8004_REPUTATION_ATTESTER_ROLE" "$ERC8004_ATTESTER_ADDRESS")"

  jq -n \
    --arg timestamp "$TIMESTAMP" \
    --arg network "$NETWORK" \
    --arg marketCore "$MARKET_CORE_ADDRESS" \
    --arg orderBook "$ORDER_BOOK_ADDRESS" \
    --arg collateralVault "$COLLATERAL_VAULT_ADDRESS" \
    --arg agentRuntime "$AGENT_RUNTIME_ADDRESS" \
    --arg identityRegistry "$AGENT_IDENTITY_REGISTRY_ADDRESS" \
    --arg reputationRegistry "$AGENT_REPUTATION_REGISTRY_ADDRESS" \
    --arg erc8004IdentityRegistry "$ERC8004_IDENTITY_REGISTRY_ADDRESS" \
    --arg erc8004ReputationRegistry "$ERC8004_REPUTATION_REGISTRY_ADDRESS" \
    --arg adminHasDefault "$ADMIN_HAS_DEFAULT" \
    --arg creatorHasRole "$CREATOR_HAS_ROLE" \
    --arg resolverHasRole "$RESOLVER_HAS_ROLE" \
    --arg pauserMarketHasRole "$PAUSER_MARKET_HAS_ROLE" \
    --arg adminOrderbook "$ADMIN_ORDERBOOK" \
    --arg pauserOrderbookHasRole "$PAUSER_ORDERBOOK_HAS_ROLE" \
    --arg runtimeCanPlaceFor "$RUNTIME_CAN_PLACE_FOR" \
    --arg optionalRuntimeOperatorHasRole "$OPTIONAL_RUNTIME_OPERATOR_HAS_ROLE" \
    --arg adminVault "$ADMIN_VAULT" \
    --arg operatorHasRole "$OPERATOR_HAS_ROLE" \
    --arg orderbookOperatorHasRole "$ORDERBOOK_OPERATOR_HAS_ROLE" \
    --arg pauserVaultHasRole "$PAUSER_VAULT_HAS_ROLE" \
    --arg adminRuntime "$ADMIN_RUNTIME" \
    --arg pauserRuntimeHasRole "$PAUSER_RUNTIME_HAS_ROLE" \
    --arg runtimeIdentityRegistry "$RUNTIME_IDENTITY_REGISTRY" \
    --arg adminIdentity "$ADMIN_IDENTITY" \
    --arg pauserIdentityHasRole "$PAUSER_IDENTITY_HAS_ROLE" \
    --arg registrarIdentityHasRole "$REGISTRAR_IDENTITY_HAS_ROLE" \
    --arg adminReputation "$ADMIN_REPUTATION" \
    --arg pauserReputationHasRole "$PAUSER_REPUTATION_HAS_ROLE" \
    --arg oracleReputationHasRole "$ORACLE_REPUTATION_HAS_ROLE" \
    --arg reputationOracle "$REPUTATION_ORACLE_ADDRESS" \
    --arg adminErc8004Identity "$ADMIN_ERC8004_IDENTITY" \
    --arg pauserErc8004IdentityHasRole "$PAUSER_ERC8004_IDENTITY_HAS_ROLE" \
    --arg issuerErc8004IdentityHasRole "$ISSUER_ERC8004_IDENTITY_HAS_ROLE" \
    --arg erc8004Issuer "$ERC8004_ISSUER_ADDRESS" \
    --arg adminErc8004Reputation "$ADMIN_ERC8004_REPUTATION" \
    --arg pauserErc8004ReputationHasRole "$PAUSER_ERC8004_REPUTATION_HAS_ROLE" \
    --arg attesterErc8004ReputationHasRole "$ATTESTER_ERC8004_REPUTATION_HAS_ROLE" \
    --arg erc8004Attester "$ERC8004_ATTESTER_ADDRESS" \
    '{
      timestamp: $timestamp,
      network: $network,
      contracts: {
        marketCore: $marketCore,
        orderBook: $orderBook,
        collateralVault: $collateralVault,
        agentRuntime: $agentRuntime,
        agentIdentityRegistry: $identityRegistry,
        agentReputationRegistry: $reputationRegistry,
        erc8004IdentityRegistry: $erc8004IdentityRegistry,
        erc8004ReputationRegistry: $erc8004ReputationRegistry
      },
      roleChecks: {
        marketCore: {
          adminDefaultRole: ($adminHasDefault == "true"),
          marketCreatorRole: ($creatorHasRole == "true"),
          resolverRole: ($resolverHasRole == "true"),
          pauserRole: ($pauserMarketHasRole == "true")
        },
        orderBook: {
          adminDefaultRole: ($adminOrderbook == "true"),
          pauserRole: ($pauserOrderbookHasRole == "true"),
          agentRuntimeRoleForRuntime: ($runtimeCanPlaceFor == "true"),
          agentRuntimeRoleForOptionalOperator: if $optionalRuntimeOperatorHasRole == "n/a" then null else ($optionalRuntimeOperatorHasRole == "true") end
        },
        collateralVault: {
          adminDefaultRole: ($adminVault == "true"),
          operatorRole: ($operatorHasRole == "true"),
          orderBookOperatorRole: ($orderbookOperatorHasRole == "true"),
          pauserRole: ($pauserVaultHasRole == "true")
        },
        agentRuntime: {
          adminDefaultRole: ($adminRuntime == "true"),
          pauserRole: ($pauserRuntimeHasRole == "true"),
          identityRegistry: $runtimeIdentityRegistry
        },
        agentIdentityRegistry: {
          adminDefaultRole: ($adminIdentity == "true"),
          pauserRole: ($pauserIdentityHasRole == "true"),
          registrarRoleForRuntime: ($registrarIdentityHasRole == "true")
        },
        agentReputationRegistry: {
          adminDefaultRole: ($adminReputation == "true"),
          pauserRole: ($pauserReputationHasRole == "true"),
          oracleAddress: $reputationOracle,
          oracleRole: ($oracleReputationHasRole == "true")
        },
        erc8004IdentityRegistry: {
          adminDefaultRole: ($adminErc8004Identity == "true"),
          pauserRole: ($pauserErc8004IdentityHasRole == "true"),
          issuerAddress: $erc8004Issuer,
          issuerRole: ($issuerErc8004IdentityHasRole == "true")
        },
        erc8004ReputationRegistry: {
          adminDefaultRole: ($adminErc8004Reputation == "true"),
          pauserRole: ($pauserErc8004ReputationHasRole == "true"),
          attesterAddress: $erc8004Attester,
          attesterRole: ($attesterErc8004ReputationHasRole == "true")
        }
      }
    }' > "$ROLE_REPORT"

  echo "Role report: $ROLE_REPORT"
fi
