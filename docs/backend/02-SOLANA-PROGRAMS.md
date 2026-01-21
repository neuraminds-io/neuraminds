# PolySecure Solana Programs Specification

> Backend Team Documentation - January 2026

## Overview

Three Anchor programs power PolySecure's on-chain logic:

1. **polysecure-market** - Market creation, resolution, and lifecycle
2. **polysecure-orderbook** - Order placement, settlement, and payouts
3. **polysecure-privacy** - Arcium integration for confidential operations

## Program 1: Market Factory (`polysecure-market`)

### State Accounts

```rust
#[account]
pub struct Market {
    /// Unique market identifier
    pub market_id: String,           // 64 bytes max

    /// Market question/title
    pub question: String,            // 256 bytes max

    /// Market description
    pub description: String,         // 1024 bytes max

    /// Market category (politics, sports, crypto, etc.)
    pub category: String,            // 32 bytes max

    /// Market creator
    pub authority: Pubkey,

    /// Resolution oracle
    pub oracle: Pubkey,

    /// YES outcome token mint
    pub yes_mint: Pubkey,

    /// NO outcome token mint
    pub no_mint: Pubkey,

    /// Collateral vault (USDC)
    pub vault: Pubkey,

    /// Collateral mint (USDC)
    pub collateral_mint: Pubkey,

    /// Market status
    pub status: MarketStatus,

    /// Resolution deadline (Unix timestamp)
    pub resolution_deadline: i64,

    /// Trading end time
    pub trading_end: i64,

    /// Resolved outcome (None until resolved)
    pub resolved_outcome: Option<Outcome>,

    /// Total collateral deposited
    pub total_collateral: u64,

    /// Fee basis points (e.g., 100 = 1%)
    pub fee_bps: u16,

    /// Bump seed for PDA
    pub bump: u8,

    /// Creation timestamp
    pub created_at: i64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum MarketStatus {
    Active,      // Trading open
    Paused,      // Trading temporarily halted
    Closed,      // Trading ended, awaiting resolution
    Resolved,    // Outcome determined
    Cancelled,   // Market cancelled, refunds available
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum Outcome {
    Yes,
    No,
}
```

### Instructions

#### `create_market`

Creates a new prediction market.

```rust
#[derive(Accounts)]
#[instruction(market_id: String)]
pub struct CreateMarket<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = 8 + Market::INIT_SPACE,
        seeds = [b"market", market_id.as_bytes()],
        bump
    )]
    pub market: Account<'info, Market>,

    #[account(
        init,
        payer = authority,
        mint::decimals = 6,
        mint::authority = market,
        seeds = [b"outcome", market.key().as_ref(), b"yes"],
        bump
    )]
    pub yes_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = authority,
        mint::decimals = 6,
        mint::authority = market,
        seeds = [b"outcome", market.key().as_ref(), b"no"],
        bump
    )]
    pub no_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = authority,
        token::mint = collateral_mint,
        token::authority = market,
        seeds = [b"vault", market.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, TokenAccount>,

    pub collateral_mint: Account<'info, Mint>,  // USDC

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn create_market(
    ctx: Context<CreateMarket>,
    market_id: String,
    question: String,
    description: String,
    category: String,
    oracle: Pubkey,
    resolution_deadline: i64,
    trading_end: i64,
    fee_bps: u16,
) -> Result<()>
```

#### `resolve_market`

Resolves market with final outcome. Only callable by oracle.

```rust
#[derive(Accounts)]
pub struct ResolveMarket<'info> {
    #[account(
        constraint = oracle.key() == market.oracle @ ErrorCode::UnauthorizedOracle
    )]
    pub oracle: Signer<'info>,

    #[account(
        mut,
        constraint = market.status == MarketStatus::Closed @ ErrorCode::MarketNotClosed
    )]
    pub market: Account<'info, Market>,
}

pub fn resolve_market(
    ctx: Context<ResolveMarket>,
    outcome: Outcome,
) -> Result<()>
```

#### `cancel_market`

Cancels market (emergency only). Enables refunds.

```rust
pub fn cancel_market(ctx: Context<CancelMarket>) -> Result<()>
```

---

## Program 2: Order Book (`polysecure-orderbook`)

### State Accounts

```rust
#[account]
pub struct Order {
    /// Order owner
    pub owner: Pubkey,

    /// Associated market
    pub market: Pubkey,

    /// Order side (Buy YES, Buy NO, Sell YES, Sell NO)
    pub side: OrderSide,

    /// Outcome being traded
    pub outcome: Outcome,

    /// Price in basis points (0-10000 = 0-100%)
    pub price_bps: u16,

    /// Original quantity
    pub original_quantity: u64,

    /// Remaining unfilled quantity
    pub remaining_quantity: u64,

    /// Order status
    pub status: OrderStatus,

    /// Order type
    pub order_type: OrderType,

    /// Timestamp
    pub created_at: i64,

    /// Expiration (0 = no expiry)
    pub expires_at: i64,

    /// Bump seed
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum OrderStatus {
    Open,
    PartiallyFilled,
    Filled,
    Cancelled,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum OrderType {
    Limit,
    Market,
    // Future: StopLoss, TakeProfit
}

#[account]
pub struct Position {
    /// Position owner
    pub owner: Pubkey,

    /// Associated market
    pub market: Pubkey,

    /// YES tokens held
    pub yes_balance: u64,

    /// NO tokens held
    pub no_balance: u64,

    /// Total USDC deposited
    pub collateral_deposited: u64,

    /// Total USDC withdrawn (from sales/redemptions)
    pub collateral_withdrawn: u64,

    /// Bump seed
    pub bump: u8,
}
```

### Instructions

#### `place_order`

Places a new order. Validates and locks collateral.

```rust
#[derive(Accounts)]
pub struct PlaceOrder<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        constraint = market.status == MarketStatus::Active @ ErrorCode::MarketNotActive
    )]
    pub market: Account<'info, Market>,

    #[account(
        init_if_needed,
        payer = owner,
        space = 8 + Order::INIT_SPACE,
        seeds = [b"order", market.key().as_ref(), &order_id.to_le_bytes()],
        bump
    )]
    pub order: Account<'info, Order>,

    #[account(
        init_if_needed,
        payer = owner,
        space = 8 + Position::INIT_SPACE,
        seeds = [b"position", market.key().as_ref(), owner.key().as_ref()],
        bump
    )]
    pub position: Account<'info, Position>,

    #[account(
        mut,
        constraint = user_collateral.owner == owner.key(),
        constraint = user_collateral.mint == market.collateral_mint
    )]
    pub user_collateral: Account<'info, TokenAccount>,

    #[account(mut)]
    pub vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn place_order(
    ctx: Context<PlaceOrder>,
    order_id: u64,
    side: OrderSide,
    outcome: Outcome,
    price_bps: u16,
    quantity: u64,
    order_type: OrderType,
    expires_at: i64,
) -> Result<()>
```

#### `settle_trade`

Settles a matched trade. Called by backend after matching.

```rust
#[derive(Accounts)]
pub struct SettleTrade<'info> {
    /// Backend authority (keeper)
    #[account(
        constraint = authority.key() == KEEPER_PUBKEY @ ErrorCode::UnauthorizedKeeper
    )]
    pub authority: Signer<'info>,

    #[account(mut)]
    pub market: Account<'info, Market>,

    #[account(mut)]
    pub buy_order: Account<'info, Order>,

    #[account(mut)]
    pub sell_order: Account<'info, Order>,

    #[account(mut)]
    pub buyer_position: Account<'info, Position>,

    #[account(mut)]
    pub seller_position: Account<'info, Position>,

    #[account(mut)]
    pub vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub yes_mint: Account<'info, Mint>,

    #[account(mut)]
    pub no_mint: Account<'info, Mint>,

    // Token accounts for buyer/seller
    // ...

    pub token_program: Program<'info, Token>,
}

pub fn settle_trade(
    ctx: Context<SettleTrade>,
    fill_quantity: u64,
    fill_price_bps: u16,
) -> Result<()>
```

#### `cancel_order`

Cancels an open order. Releases locked collateral.

```rust
pub fn cancel_order(ctx: Context<CancelOrder>) -> Result<()>
```

#### `claim_winnings`

Claims winnings after market resolution.

```rust
#[derive(Accounts)]
pub struct ClaimWinnings<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        constraint = market.status == MarketStatus::Resolved @ ErrorCode::MarketNotResolved
    )]
    pub market: Account<'info, Market>,

    #[account(
        mut,
        seeds = [b"position", market.key().as_ref(), owner.key().as_ref()],
        bump = position.bump
    )]
    pub position: Account<'info, Position>,

    #[account(mut)]
    pub vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user_collateral: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user_yes_tokens: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user_no_tokens: Account<'info, TokenAccount>,

    #[account(mut)]
    pub yes_mint: Account<'info, Mint>,

    #[account(mut)]
    pub no_mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
}

pub fn claim_winnings(ctx: Context<ClaimWinnings>) -> Result<()>
```

---

## Program 3: Privacy Layer (`polysecure-privacy`)

### Integration with Arcium

This program integrates with Arcium's MXE for confidential operations.

```rust
use arcium_sdk::prelude::*;

#[account]
pub struct PrivateAccount {
    /// Account owner
    pub owner: Pubkey,

    /// Encrypted balance (Arcium ciphertext)
    pub encrypted_balance: [u8; 64],

    /// Associated MXE
    pub mxe_id: Pubkey,

    /// ElGamal public key for this account
    pub elgamal_pubkey: [u8; 32],

    /// Bump seed
    pub bump: u8,
}

#[account]
pub struct PrivateOrder {
    /// Order owner
    pub owner: Pubkey,

    /// Market
    pub market: Pubkey,

    /// Encrypted price commitment
    pub price_commitment: [u8; 32],

    /// Encrypted quantity commitment
    pub quantity_commitment: [u8; 32],

    /// ZK proof of valid order
    pub proof: [u8; 128],

    /// Order side (public - needed for matching)
    pub side: OrderSide,

    /// Outcome (public - needed for matching)
    pub outcome: Outcome,

    /// Status
    pub status: OrderStatus,

    /// Bump
    pub bump: u8,
}
```

### Instructions

#### `create_private_account`

Creates a confidential account linked to Arcium MXE.

```rust
pub fn create_private_account(
    ctx: Context<CreatePrivateAccount>,
    elgamal_pubkey: [u8; 32],
) -> Result<()>
```

#### `private_deposit`

Deposits funds confidentially using C-SPL.

```rust
pub fn private_deposit(
    ctx: Context<PrivateDeposit>,
    encrypted_amount: [u8; 64],
    range_proof: [u8; 128],
) -> Result<()>
```

#### `private_withdraw`

Withdraws funds with ZK proof of sufficient balance.

```rust
pub fn private_withdraw(
    ctx: Context<PrivateWithdraw>,
    encrypted_amount: [u8; 64],
    balance_proof: [u8; 128],
) -> Result<()>
```

#### `place_private_order`

Places an order with hidden price/quantity.

```rust
pub fn place_private_order(
    ctx: Context<PlacePrivateOrder>,
    price_commitment: [u8; 32],
    quantity_commitment: [u8; 32],
    range_proof: [u8; 128],
    side: OrderSide,
    outcome: Outcome,
) -> Result<()>
```

#### `private_settle`

Settles a matched private trade via Arcium MXE.

```rust
pub fn private_settle(
    ctx: Context<PrivateSettle>,
    mxe_computation_result: [u8; 256],
    settlement_proof: [u8; 128],
) -> Result<()>
```

---

## Error Codes

```rust
#[error_code]
pub enum ErrorCode {
    #[msg("Market is not active")]
    MarketNotActive,

    #[msg("Market is not closed")]
    MarketNotClosed,

    #[msg("Market is not resolved")]
    MarketNotResolved,

    #[msg("Unauthorized oracle")]
    UnauthorizedOracle,

    #[msg("Unauthorized keeper")]
    UnauthorizedKeeper,

    #[msg("Invalid price (must be 1-9999 bps)")]
    InvalidPrice,

    #[msg("Invalid quantity")]
    InvalidQuantity,

    #[msg("Insufficient collateral")]
    InsufficientCollateral,

    #[msg("Insufficient balance")]
    InsufficientBalance,

    #[msg("Order expired")]
    OrderExpired,

    #[msg("Order already filled")]
    OrderAlreadyFilled,

    #[msg("Order already cancelled")]
    OrderAlreadyCancelled,

    #[msg("Invalid proof")]
    InvalidProof,

    #[msg("MXE computation failed")]
    MxeComputationFailed,

    #[msg("Trading has ended for this market")]
    TradingEnded,

    #[msg("Resolution deadline not reached")]
    ResolutionDeadlineNotReached,

    #[msg("Arithmetic overflow")]
    ArithmeticOverflow,
}
```

---

## Events

```rust
#[event]
pub struct MarketCreated {
    pub market_id: String,
    pub market: Pubkey,
    pub authority: Pubkey,
    pub question: String,
    pub resolution_deadline: i64,
}

#[event]
pub struct MarketResolved {
    pub market: Pubkey,
    pub outcome: Outcome,
    pub resolved_at: i64,
}

#[event]
pub struct OrderPlaced {
    pub order: Pubkey,
    pub market: Pubkey,
    pub owner: Pubkey,
    pub side: OrderSide,
    pub outcome: Outcome,
    pub price_bps: u16,
    pub quantity: u64,
}

#[event]
pub struct TradeFilled {
    pub market: Pubkey,
    pub buy_order: Pubkey,
    pub sell_order: Pubkey,
    pub price_bps: u16,
    pub quantity: u64,
    pub timestamp: i64,
}

#[event]
pub struct WinningsClaimed {
    pub market: Pubkey,
    pub owner: Pubkey,
    pub amount: u64,
}
```

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::prelude::*;

    #[test]
    fn test_create_market() { /* ... */ }

    #[test]
    fn test_place_order_valid() { /* ... */ }

    #[test]
    fn test_place_order_invalid_price() { /* ... */ }

    #[test]
    fn test_settle_trade() { /* ... */ }

    #[test]
    fn test_claim_winnings_yes() { /* ... */ }

    #[test]
    fn test_claim_winnings_no() { /* ... */ }
}
```

### Integration Tests

Use `anchor test` with local validator:

```bash
anchor test --skip-local-validator  # Uses Devnet
anchor test                          # Spins up local validator
```

### Bankrun Tests (Recommended 2026)

Use Surfpool for mainnet-fork testing:

```bash
surfpool test --fork mainnet
```

---

## Deployment

### Devnet

```bash
anchor build
anchor deploy --provider.cluster devnet
```

### Mainnet

```bash
# Build with verifiable flag
anchor build --verifiable

# Deploy
anchor deploy --provider.cluster mainnet

# Verify
anchor verify <PROGRAM_ID>
```

---

## Security Checklist

- [ ] All arithmetic uses `checked_*` operations
- [ ] All account constraints properly validated
- [ ] No unchecked external CPIs
- [ ] Owner checks on all token accounts
- [ ] Signer checks on authority accounts
- [ ] PDA seeds are deterministic and unique
- [ ] Events emitted for all state changes
- [ ] Upgrade authority secured (multisig)
- [ ] Audit completed before mainnet
