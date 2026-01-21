# PolySecure Arcium Integration Guide

> Backend Team Documentation - January 2026

## Overview

Arcium provides the confidential computing layer for PolySecure's private trading mode. This document covers integration with Arcium's mainnet infrastructure for:

- Confidential balances (C-SPL tokens)
- Private order placement with hidden amounts
- Secure multi-party computation for order matching
- ZK proofs for balance verification

## Arcium Status (January 2026)

| Component | Status | Notes |
|-----------|--------|-------|
| Mainnet Alpha | **LIVE** | Launched Q4 2025 |
| Full Mainnet + TGE | **Q1 2026** | Imminent |
| C-SPL (Devnet) | **Available** | Confidential SPL tokens |
| Arcis SDK | **Stable** | Rust DSL for MPC programs |
| MXE Infrastructure | **Operational** | Multiple node operators |

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        USER (Private Mode)                          │
│                                                                     │
│  - Generates ElGamal keypair                                        │
│  - Encrypts order amounts client-side                               │
│  - Submits encrypted orders with ZK proofs                          │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    POLYSECURE BACKEND                               │
│                                                                     │
│  - Validates ZK proofs (range proofs, balance proofs)               │
│  - Manages encrypted order book                                     │
│  - Coordinates MXE computations for matching                        │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    ARCIUM NETWORK                                   │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                     MXE (Execution Environment)              │   │
│  │                                                              │   │
│  │  - Private order matching computation                        │   │
│  │  - Confidential settlement amounts                           │   │
│  │  - Multi-party computation across nodes                      │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                     SOLANA PROGRAM                           │   │
│  │                                                              │   │
│  │  - MXE orchestration                                         │   │
│  │  - Computation scheduling                                    │   │
│  │  - Result verification                                       │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│               SOLANA (Settlement Layer)                             │
│                                                                     │
│  - C-SPL confidential transfers                                     │
│  - Token-2022 with confidential extensions                          │
│  - Encrypted balance updates                                        │
└─────────────────────────────────────────────────────────────────────┘
```

## Key Concepts

### 1. MXE (Multiparty Execution Environment)

MXEs are virtual machines for defining MPC computations. Each MXE:

- Combines MPC, FHE, and ZK proofs
- Runs across multiple Arcium nodes
- Provides configurable trust assumptions
- Guarantees privacy if at least 1 node is honest

### 2. Arcis (Rust DSL)

Arcis is Arcium's domain-specific language for writing MPC programs:

```rust
use arcium_sdk::prelude::*;

// Masked types maintain confidentiality
fn private_order_match(
    buy_price: MaskedU64,
    buy_quantity: MaskedU64,
    sell_price: MaskedU64,
    sell_quantity: MaskedU64,
) -> (MaskedU64, MaskedBool) {
    // Computation happens on encrypted values
    let prices_match = buy_price.gte(&sell_price);
    let fill_quantity = buy_quantity.min(&sell_quantity);

    (fill_quantity, prices_match)
}
```

### 3. C-SPL (Confidential SPL Tokens)

C-SPL combines:
- SPL Token standard
- Token-2022 extensions
- Confidential Transfer Extension
- Arcium MPC layer

## Integration Steps

### Step 1: SDK Setup

Add Arcium SDK to your project:

```toml
# Cargo.toml
[dependencies]
arcium-sdk = "0.5"  # Check for latest version
arcium-client = "0.5"
```

### Step 2: Initialize Arcium Client

```rust
use arcium_client::{ArciumClient, Config};

pub struct PrivacyService {
    arcium: ArciumClient,
    mxe_id: Pubkey,
}

impl PrivacyService {
    pub async fn new(rpc_url: &str, mxe_id: Pubkey) -> Result<Self> {
        let config = Config::mainnet();  // or Config::devnet()
        let arcium = ArciumClient::new(rpc_url, config).await?;

        Ok(Self { arcium, mxe_id })
    }
}
```

### Step 3: Create Private Account

Users need an Arcium-enabled account for private trading:

```rust
use arcium_sdk::crypto::{ElGamalKeypair, encrypt};

pub async fn create_private_account(
    &self,
    user_wallet: &Keypair,
) -> Result<PrivateAccountInfo> {
    // Generate ElGamal keypair for user
    let elgamal_keypair = ElGamalKeypair::new();

    // Create account on-chain
    let tx = self.arcium
        .create_confidential_account(
            user_wallet,
            &elgamal_keypair.public_key(),
            self.mxe_id,
        )
        .await?;

    Ok(PrivateAccountInfo {
        address: tx.account_address,
        elgamal_pubkey: elgamal_keypair.public_key(),
        // Store keypair securely client-side!
    })
}
```

### Step 4: Private Deposit

Deposit funds into confidential account:

```rust
pub async fn private_deposit(
    &self,
    user: &Keypair,
    amount: u64,
    elgamal_keypair: &ElGamalKeypair,
) -> Result<Signature> {
    // Encrypt the amount
    let encrypted_amount = encrypt(
        amount,
        &elgamal_keypair.public_key(),
    )?;

    // Generate range proof (amount is valid, non-negative)
    let range_proof = generate_range_proof(
        amount,
        elgamal_keypair,
    )?;

    // Submit deposit transaction
    let tx = self.arcium
        .confidential_deposit(
            user,
            encrypted_amount,
            range_proof,
        )
        .await?;

    Ok(tx.signature)
}
```

### Step 5: Place Private Order

```rust
use arcium_sdk::commitments::{PedersenCommitment, commit};

pub struct PrivateOrderRequest {
    pub market_id: String,
    pub side: OrderSide,
    pub outcome: Outcome,
    pub encrypted_price: EncryptedValue,
    pub encrypted_quantity: EncryptedValue,
    pub price_commitment: PedersenCommitment,
    pub quantity_commitment: PedersenCommitment,
    pub range_proof: RangeProof,
}

pub async fn place_private_order(
    &self,
    user: &Keypair,
    market_id: &str,
    side: OrderSide,
    outcome: Outcome,
    price: u64,      // in basis points (1-9999)
    quantity: u64,
    elgamal_keypair: &ElGamalKeypair,
) -> Result<PrivateOrderResponse> {
    // Encrypt price and quantity
    let encrypted_price = encrypt(price, &elgamal_keypair.public_key())?;
    let encrypted_quantity = encrypt(quantity, &elgamal_keypair.public_key())?;

    // Create Pedersen commitments for matching
    let (price_commitment, price_blinding) = commit(price)?;
    let (quantity_commitment, quantity_blinding) = commit(quantity)?;

    // Generate proofs
    let range_proof = generate_order_proof(
        price,
        quantity,
        &price_blinding,
        &quantity_blinding,
        elgamal_keypair,
    )?;

    // Submit to Solana program
    let order_id = self.submit_private_order(
        user,
        market_id,
        side,
        outcome,
        encrypted_price,
        encrypted_quantity,
        price_commitment,
        quantity_commitment,
        range_proof,
    ).await?;

    Ok(PrivateOrderResponse {
        order_id,
        status: OrderStatus::Open,
    })
}
```

### Step 6: Private Order Matching (MXE)

The matching computation runs inside Arcium MXE:

```rust
// This runs inside the MXE - Arcis program
use arcium_sdk::prelude::*;

#[arcium_program]
pub mod private_matcher {
    use super::*;

    /// Match two private orders
    /// Returns: (should_match, fill_quantity, fill_price)
    pub fn match_orders(
        buy_price: MaskedU64,
        buy_quantity: MaskedU64,
        sell_price: MaskedU64,
        sell_quantity: MaskedU64,
    ) -> (MaskedBool, MaskedU64, MaskedU64) {
        // Check if prices cross (buy >= sell)
        let prices_match = buy_price.gte(&sell_price);

        // Calculate fill quantity (min of both)
        let fill_quantity = buy_quantity.min(&sell_quantity);

        // Fill price = midpoint (or use sell price for price-time priority)
        let fill_price = sell_price;

        (prices_match, fill_quantity, fill_price)
    }

    /// Verify user has sufficient balance for order
    pub fn verify_balance(
        encrypted_balance: MaskedU64,
        order_cost: MaskedU64,
    ) -> MaskedBool {
        encrypted_balance.gte(&order_cost)
    }
}
```

### Step 7: Settlement

After MXE matching, settle on-chain:

```rust
pub async fn settle_private_trade(
    &self,
    mxe_result: MxeComputationResult,
    buy_order: &PrivateOrder,
    sell_order: &PrivateOrder,
) -> Result<Signature> {
    // Verify MXE computation result
    let verified = self.arcium
        .verify_computation(mxe_result.clone())
        .await?;

    if !verified {
        return Err(Error::InvalidMxeResult);
    }

    // Extract encrypted settlement amounts
    let settlement_data = mxe_result.output;

    // Execute confidential transfer via C-SPL
    let tx = self.execute_confidential_settlement(
        buy_order,
        sell_order,
        settlement_data,
    ).await?;

    Ok(tx.signature)
}
```

## Token-2022 Confidential Transfers

For simpler privacy (without full Arcium MPC), use native Token-2022:

```rust
use spl_token_2022::extension::confidential_transfer::*;

pub async fn setup_confidential_mint(
    &self,
    authority: &Keypair,
) -> Result<Pubkey> {
    // Create mint with confidential transfer extension
    let mint = create_mint_with_extensions(
        &self.rpc_client,
        authority,
        6,  // decimals
        &[ExtensionType::ConfidentialTransfer],
    ).await?;

    // Configure confidential transfer
    configure_confidential_transfer(
        &self.rpc_client,
        &mint,
        authority,
        ConfidentialTransferConfig {
            auto_approve: true,
            auditor: None,  // Optional compliance auditor
        },
    ).await?;

    Ok(mint)
}

pub async fn confidential_transfer(
    &self,
    sender: &Keypair,
    recipient: Pubkey,
    amount: u64,
    sender_elgamal: &ElGamalKeypair,
) -> Result<Signature> {
    // Generate transfer proof
    let proof = generate_transfer_proof(
        amount,
        sender_elgamal,
    )?;

    // Execute confidential transfer
    let sig = transfer_confidential(
        &self.rpc_client,
        sender,
        recipient,
        amount,
        proof,
    ).await?;

    Ok(sig)
}
```

## Security Considerations

### 1. Key Management

```rust
// NEVER store ElGamal private keys on server
// Keys should be generated and stored client-side

// For recovery, use deterministic derivation from wallet
pub fn derive_elgamal_keypair(
    wallet_keypair: &Keypair,
    domain: &str,
) -> ElGamalKeypair {
    let seed = derive_seed(
        wallet_keypair,
        domain,
        "polysecure-elgamal-v1",
    );
    ElGamalKeypair::from_seed(&seed)
}
```

### 2. Proof Verification

Always verify proofs server-side before accepting orders:

```rust
pub fn verify_order_proof(
    order: &PrivateOrderRequest,
) -> Result<bool> {
    // Verify range proof (price in valid range)
    let price_valid = verify_range_proof(
        &order.price_commitment,
        &order.range_proof.price_proof,
        1,     // min
        9999,  // max
    )?;

    // Verify quantity proof (positive, within limits)
    let quantity_valid = verify_range_proof(
        &order.quantity_commitment,
        &order.range_proof.quantity_proof,
        1,
        MAX_ORDER_QUANTITY,
    )?;

    Ok(price_valid && quantity_valid)
}
```

### 3. MXE Trust Assumptions

Configure appropriate trust level:

```rust
pub enum TrustModel {
    /// Privacy guaranteed if 1 node is honest
    /// Can detect and identify cheaters
    Cerberus,

    /// Higher performance, different tradeoffs
    Manticore,
}

// For financial applications, use Cerberus
let mxe_config = MxeConfig {
    trust_model: TrustModel::Cerberus,
    min_nodes: 5,
    threshold: 3,  // 3-of-5 threshold
};
```

## Testing

### Devnet Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_private_order_flow() {
        // Use Arcium devnet
        let client = ArciumClient::new(
            "https://api.devnet.solana.com",
            Config::devnet(),
        ).await.unwrap();

        // Create test keypairs
        let user = Keypair::new();
        let elgamal = ElGamalKeypair::new();

        // Airdrop SOL for fees
        airdrop(&client, &user.pubkey(), 1_000_000_000).await;

        // Test private account creation
        let account = create_private_account(&client, &user, &elgamal)
            .await
            .unwrap();

        // Test private deposit
        let deposit_sig = private_deposit(
            &client,
            &user,
            1000_000_000,  // 1000 USDC
            &elgamal,
        ).await.unwrap();

        // Verify deposit
        let balance = get_confidential_balance(&client, &account)
            .await
            .unwrap();

        assert!(balance.is_encrypted());
    }
}
```

### Local MXE Testing

For development, Arcium provides local MXE simulation:

```bash
# Start local Arcium node
arcium-local-node --config dev.toml

# Run tests against local node
cargo test --features local-mxe
```

## Monitoring & Observability

```rust
// Log MXE computation metrics
pub fn log_mxe_computation(
    computation_id: &str,
    duration_ms: u64,
    nodes_participated: usize,
    success: bool,
) {
    metrics::histogram!(
        "arcium.mxe.computation_duration_ms",
        duration_ms as f64,
    );

    metrics::counter!(
        "arcium.mxe.computations_total",
        1,
        "success" => success.to_string(),
    );
}
```

## Resources

- [Arcium Documentation](https://docs.arcium.com)
- [Arcium SDK GitHub](https://github.com/arcium-network/arcium-sdk)
- [Arcis Language Guide](https://docs.arcium.com/arcis)
- [C-SPL Specification](https://docs.arcium.com/c-spl)
- [Token-2022 Confidential Transfers](https://spl.solana.com/confidential-token)
- [Helius Privacy Guide](https://www.helius.dev/blog/solana-privacy)
