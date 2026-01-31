#!/bin/bash
set -euo pipefail

# Polyguard Mainnet Initialization
# Run after deploy-mainnet.sh to initialize program state

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

# Load program IDs from deploy output
MARKET_PROGRAM_ID="${MARKET_PROGRAM_ID:-$(cat target/deploy/market-program-id.txt 2>/dev/null || echo '')}"
ORDERBOOK_PROGRAM_ID="${ORDERBOOK_PROGRAM_ID:-$(cat target/deploy/orderbook-program-id.txt 2>/dev/null || echo '')}"

check_prereqs() {
    [[ -n "$MARKET_PROGRAM_ID" ]] || error "MARKET_PROGRAM_ID not set"
    [[ -n "$ORDERBOOK_PROGRAM_ID" ]] || error "ORDERBOOK_PROGRAM_ID not set"
    [[ -f "$KEYPAIR" ]] || error "Keypair not found: $KEYPAIR"

    log "Market Program: $MARKET_PROGRAM_ID"
    log "Orderbook Program: $ORDERBOOK_PROGRAM_ID"
}

# Initialize orderbook config with keeper authority
init_orderbook_config() {
    log "Initializing orderbook config..."

    local keeper_pubkey=$(solana-keygen pubkey "$KEYPAIR")

    # Use anchor to call initialize_config
    npx ts-node --esm << EOF
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, Keypair } from "@solana/web3.js";
import fs from "fs";

const keypairData = JSON.parse(fs.readFileSync("$KEYPAIR", "utf8"));
const wallet = Keypair.fromSecretKey(new Uint8Array(keypairData));

const connection = new anchor.web3.Connection("$RPC_URL", "confirmed");
const provider = new anchor.AnchorProvider(
    connection,
    new anchor.Wallet(wallet),
    { commitment: "confirmed" }
);
anchor.setProvider(provider);

const programId = new PublicKey("$ORDERBOOK_PROGRAM_ID");

// Load IDL
const idl = JSON.parse(fs.readFileSync("target/idl/polyguard_orderbook.json", "utf8"));
const program = new Program(idl, programId, provider);

async function main() {
    const [configPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("orderbook_config")],
        programId
    );

    console.log("Config PDA:", configPda.toBase58());

    try {
        const tx = await program.methods
            .initializeConfig(wallet.publicKey)
            .accounts({
                authority: wallet.publicKey,
                config: configPda,
                systemProgram: anchor.web3.SystemProgram.programId,
            })
            .rpc();

        console.log("Initialized orderbook config:", tx);
    } catch (e: any) {
        if (e.message?.includes("already in use")) {
            console.log("Config already initialized");
        } else {
            throw e;
        }
    }
}

main().catch(console.error);
EOF

    log "Orderbook config initialized"
}

# Initialize oracle registry
init_oracle_registry() {
    log "Initializing oracle registry..."

    npx ts-node --esm << EOF
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, Keypair } from "@solana/web3.js";
import fs from "fs";

const keypairData = JSON.parse(fs.readFileSync("$KEYPAIR", "utf8"));
const wallet = Keypair.fromSecretKey(new Uint8Array(keypairData));

const connection = new anchor.web3.Connection("$RPC_URL", "confirmed");
const provider = new anchor.AnchorProvider(
    connection,
    new anchor.Wallet(wallet),
    { commitment: "confirmed" }
);
anchor.setProvider(provider);

const programId = new PublicKey("$MARKET_PROGRAM_ID");

// Load IDL
const idl = JSON.parse(fs.readFileSync("target/idl/polyguard_market.json", "utf8"));
const program = new Program(idl, programId, provider);

async function main() {
    const [registryPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("oracle_registry")],
        programId
    );

    console.log("Oracle Registry PDA:", registryPda.toBase58());

    try {
        const tx = await program.methods
            .initializeOracleRegistry(true) // enforce validation
            .accounts({
                authority: wallet.publicKey,
                registry: registryPda,
                systemProgram: anchor.web3.SystemProgram.programId,
            })
            .rpc();

        console.log("Initialized oracle registry:", tx);
    } catch (e: any) {
        if (e.message?.includes("already in use")) {
            console.log("Oracle registry already initialized");
        } else {
            throw e;
        }
    }
}

main().catch(console.error);
EOF

    log "Oracle registry initialized"
}

# Output environment file for backend
generate_env_file() {
    log "Generating environment file..."

    cat > .env.mainnet << EOF
# Polyguard Mainnet Configuration
# Generated: $(date -u +"%Y-%m-%dT%H:%M:%SZ")

# Solana
SOLANA_RPC_URL=$RPC_URL
SOLANA_WS_URL=$(echo "$RPC_URL" | sed 's/https/wss/' | sed 's/http/ws/')

# Program IDs
MARKET_PROGRAM_ID=$MARKET_PROGRAM_ID
ORDERBOOK_PROGRAM_ID=$ORDERBOOK_PROGRAM_ID

# Keeper (set to your backend service keypair)
KEEPER_KEYPAIR=$(cat "$KEYPAIR" | jq -c '.')

# Database (update with your production values)
DATABASE_URL=postgres://user:pass@host:5432/polyguard

# Redis (update with your production values)
REDIS_URL=redis://host:6379

# JWT (generate a secure random secret)
JWT_SECRET=$(openssl rand -hex 32)

# API
API_HOST=0.0.0.0
API_PORT=8080
RUST_LOG=info,polyguard=debug
EOF

    log "Generated .env.mainnet"
    warn "Update DATABASE_URL, REDIS_URL, and KEEPER_KEYPAIR before deploying backend"
}

main() {
    echo ""
    echo "==========================================="
    echo "  Polyguard Mainnet Initialization"
    echo "==========================================="
    echo ""

    check_prereqs

    init_orderbook_config
    init_oracle_registry
    generate_env_file

    echo ""
    echo "==========================================="
    echo "  Initialization Complete!"
    echo "==========================================="
    echo ""
    log "Next steps:"
    log "  1. Update .env.mainnet with production database/redis"
    log "  2. Deploy backend: docker build -t polyguard/api . && docker push"
    log "  3. Apply k8s manifests: kubectl apply -f infra/k8s/"
    log "  4. Deploy frontend to Vercel/Cloudflare"
    log "  5. Create first market!"
    echo ""
}

main "$@"
