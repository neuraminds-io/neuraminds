use anchor_lang::prelude::*;
use crate::state::{PrivacyConfig, PrivateAccount, PrivateOrder};
use crate::errors::PrivacyError;

#[derive(Accounts)]
pub struct PlacePrivateOrder<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [PrivacyConfig::SEED_PREFIX],
        bump = config.bump,
        constraint = config.enabled @ PrivacyError::PrivateAccountNotInitialized
    )]
    pub config: Account<'info, PrivacyConfig>,

    /// CHECK: Market account
    pub market: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [PrivateAccount::SEED_PREFIX, owner.key().as_ref()],
        bump = private_account.bump,
        constraint = private_account.owner == owner.key() @ PrivacyError::UnauthorizedAdmin,
        constraint = private_account.is_active @ PrivacyError::PrivateAccountNotInitialized
    )]
    pub private_account: Account<'info, PrivateAccount>,

    #[account(
        init,
        payer = owner,
        space = 8 + PrivateOrder::INIT_SPACE,
        seeds = [
            PrivateOrder::SEED_PREFIX,
            market.key().as_ref(),
            &config.total_private_orders.to_le_bytes()
        ],
        bump
    )]
    pub private_order: Account<'info, PrivateOrder>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<PlacePrivateOrder>,
    side: u8,
    outcome: u8,
    price_commitment: [u8; 32],
    quantity_commitment: [u8; 32],
    range_proof: [u8; 128],
) -> Result<()> {
    // Validate side
    require!(
        side == PrivateOrder::SIDE_BUY || side == PrivateOrder::SIDE_SELL,
        PrivacyError::InvalidOrderSide
    );

    // Validate outcome
    require!(
        outcome == PrivateOrder::OUTCOME_YES || outcome == PrivateOrder::OUTCOME_NO,
        PrivacyError::InvalidOutcomeType
    );

    // Validate commitments are not empty
    require!(
        price_commitment.iter().any(|&b| b != 0),
        PrivacyError::InvalidPriceCommitment
    );
    require!(
        quantity_commitment.iter().any(|&b| b != 0),
        PrivacyError::InvalidQuantityCommitment
    );

    // Validate range proof (placeholder)
    require!(
        range_proof.iter().any(|&b| b != 0),
        PrivacyError::InvalidRangeProof
    );

    let clock = Clock::get()?;
    let order_id = ctx.accounts.config.total_private_orders;

    // Initialize private order
    let private_order = &mut ctx.accounts.private_order;
    private_order.owner = ctx.accounts.owner.key();
    private_order.market = ctx.accounts.market.key();
    private_order.order_id = order_id;
    private_order.side = side;
    private_order.outcome = outcome;
    private_order.price_commitment = price_commitment;
    private_order.quantity_commitment = quantity_commitment;
    private_order.range_proof = range_proof;
    private_order.status = PrivateOrder::STATUS_OPEN;
    private_order.price_hint_bps = 0; // Would be set by client for MVP matching
    private_order.quantity_hint = 0;
    private_order.bump = ctx.bumps.private_order;
    private_order.created_at = clock.unix_timestamp;
    private_order.settled_at = 0;

    // Update config
    let config = &mut ctx.accounts.config;
    config.total_private_orders = config.total_private_orders
        .checked_add(1)
        .ok_or(PrivacyError::ArithmeticOverflow)?;

    // Update private account
    let private_account = &mut ctx.accounts.private_account;
    private_account.private_order_count = private_account.private_order_count
        .checked_add(1)
        .ok_or(PrivacyError::ArithmeticOverflow)?;
    private_account.last_activity = clock.unix_timestamp;

    emit!(PrivateOrderPlaced {
        order_id,
        order: private_order.key(),
        market: private_order.market,
        owner: private_order.owner,
        side,
        outcome,
        price_commitment,
        quantity_commitment,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

#[event]
pub struct PrivateOrderPlaced {
    pub order_id: u64,
    pub order: Pubkey,
    pub market: Pubkey,
    pub owner: Pubkey,
    pub side: u8,
    pub outcome: u8,
    pub price_commitment: [u8; 32],
    pub quantity_commitment: [u8; 32],
    pub timestamp: i64,
}
