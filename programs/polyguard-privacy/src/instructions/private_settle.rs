use anchor_lang::prelude::*;
use crate::state::{PrivacyConfig, PrivateAccount, PrivateOrder, PrivateSettlement};
use crate::errors::PrivacyError;

#[derive(Accounts)]
#[instruction(buy_order_id: u64, sell_order_id: u64)]
pub struct PrivateSettle<'info> {
    /// MXE authority (Arcium verifier)
    #[account(mut)]
    pub mxe_authority: Signer<'info>,

    #[account(
        mut,
        seeds = [PrivacyConfig::SEED_PREFIX],
        bump = config.bump,
        constraint = mxe_authority.key() == config.mxe_authority @ PrivacyError::UnauthorizedMxeAuthority
    )]
    pub config: Box<Account<'info, PrivacyConfig>>,

    /// CHECK: Market account
    pub market: UncheckedAccount<'info>,

    // Buy order (boxed to reduce stack)
    #[account(
        mut,
        seeds = [
            PrivateOrder::SEED_PREFIX,
            market.key().as_ref(),
            &buy_order_id.to_le_bytes()
        ],
        bump,
        constraint = buy_order.side == PrivateOrder::SIDE_BUY @ PrivacyError::InvalidOrderSide,
        constraint = buy_order.status == PrivateOrder::STATUS_OPEN @ PrivacyError::MxeComputationFailed
    )]
    pub buy_order: Box<Account<'info, PrivateOrder>>,

    #[account(
        mut,
        seeds = [PrivateAccount::SEED_PREFIX, buy_order.owner.as_ref()],
        bump
    )]
    pub buyer_account: Box<Account<'info, PrivateAccount>>,

    // Sell order (boxed to reduce stack)
    #[account(
        mut,
        seeds = [
            PrivateOrder::SEED_PREFIX,
            market.key().as_ref(),
            &sell_order_id.to_le_bytes()
        ],
        bump,
        constraint = sell_order.side == PrivateOrder::SIDE_SELL @ PrivacyError::InvalidOrderSide,
        constraint = sell_order.status == PrivateOrder::STATUS_OPEN @ PrivacyError::MxeComputationFailed
    )]
    pub sell_order: Box<Account<'info, PrivateOrder>>,

    #[account(
        mut,
        seeds = [PrivateAccount::SEED_PREFIX, sell_order.owner.as_ref()],
        bump
    )]
    pub seller_account: Box<Account<'info, PrivateAccount>>,

    // Settlement record (boxed to reduce stack)
    #[account(
        init,
        payer = mxe_authority,
        space = 8 + PrivateSettlement::INIT_SPACE,
        seeds = [
            PrivateSettlement::SEED_PREFIX,
            buy_order.key().as_ref(),
            sell_order.key().as_ref()
        ],
        bump
    )]
    pub settlement: Box<Account<'info, PrivateSettlement>>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<PrivateSettle>,
    _buy_order_id: u64,
    _sell_order_id: u64,
    mxe_result: [u8; 256],
    settlement_proof: [u8; 128],
) -> Result<()> {
    // Validate MXE result (in production, verify cryptographic proof)
    require!(
        mxe_result.iter().any(|&b| b != 0),
        PrivacyError::InvalidMxeResult
    );

    // Validate settlement proof
    require!(
        settlement_proof.iter().any(|&b| b != 0),
        PrivacyError::InvalidSettlementProof
    );

    // Validate orders are for same market and outcome
    require!(
        ctx.accounts.buy_order.market == ctx.accounts.sell_order.market,
        PrivacyError::MxeComputationFailed
    );
    require!(
        ctx.accounts.buy_order.outcome == ctx.accounts.sell_order.outcome,
        PrivacyError::MxeComputationFailed
    );

    let clock = Clock::get()?;

    // Initialize settlement record
    let settlement = &mut ctx.accounts.settlement;
    settlement.buy_order = ctx.accounts.buy_order.key();
    settlement.sell_order = ctx.accounts.sell_order.key();
    settlement.market = ctx.accounts.market.key();
    settlement.mxe_result = mxe_result;
    settlement.settlement_proof = settlement_proof;
    settlement.encrypted_fill_quantity = [0u8; 64];
    settlement.encrypted_fill_price = [0u8; 64];
    settlement.status = PrivateSettlement::STATUS_COMPLETED;
    settlement.bump = ctx.bumps.settlement;
    settlement.settled_at = clock.unix_timestamp;

    // Update orders
    let buy_order = &mut ctx.accounts.buy_order;
    buy_order.status = PrivateOrder::STATUS_FILLED;
    buy_order.settled_at = clock.unix_timestamp;

    let sell_order = &mut ctx.accounts.sell_order;
    sell_order.status = PrivateOrder::STATUS_FILLED;
    sell_order.settled_at = clock.unix_timestamp;

    // Update accounts
    let buyer_account = &mut ctx.accounts.buyer_account;
    buyer_account.private_settlement_count = buyer_account.private_settlement_count
        .checked_add(1)
        .ok_or(PrivacyError::ArithmeticOverflow)?;
    buyer_account.last_activity = clock.unix_timestamp;

    let seller_account = &mut ctx.accounts.seller_account;
    seller_account.private_settlement_count = seller_account.private_settlement_count
        .checked_add(1)
        .ok_or(PrivacyError::ArithmeticOverflow)?;
    seller_account.last_activity = clock.unix_timestamp;

    // Update config
    let config = &mut ctx.accounts.config;
    config.total_private_settlements = config.total_private_settlements
        .checked_add(1)
        .ok_or(PrivacyError::ArithmeticOverflow)?;

    emit!(PrivateTradeSettled {
        settlement: settlement.key(),
        buy_order: ctx.accounts.buy_order.key(),
        sell_order: ctx.accounts.sell_order.key(),
        market: settlement.market,
        buyer: ctx.accounts.buy_order.owner,
        seller: ctx.accounts.sell_order.owner,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

#[event]
pub struct PrivateTradeSettled {
    pub settlement: Pubkey,
    pub buy_order: Pubkey,
    pub sell_order: Pubkey,
    pub market: Pubkey,
    pub buyer: Pubkey,
    pub seller: Pubkey,
    pub timestamp: i64,
}
