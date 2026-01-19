use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::Market;
use crate::errors::MarketError;

#[derive(Accounts)]
pub struct WithdrawFees<'info> {
    /// Market authority (only authority can withdraw fees)
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        constraint = market.authority == authority.key() @ MarketError::UnauthorizedOracle,
        constraint = market.accumulated_fees > 0 @ MarketError::NoFeesToWithdraw
    )]
    pub market: Account<'info, Market>,

    #[account(
        mut,
        seeds = [Market::VAULT_SEED, market.key().as_ref()],
        bump = market.vault_bump
    )]
    pub vault: Account<'info, TokenAccount>,

    /// Treasury account to receive fees
    #[account(
        mut,
        constraint = treasury.mint == market.collateral_mint @ MarketError::InvalidCollateral
    )]
    pub treasury: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

/// Withdraw accumulated fees to the protocol treasury
///
/// Security:
/// - Only market authority can withdraw fees
/// - Cannot withdraw more than accumulated_fees
/// - Updates accumulated_fees to prevent double withdrawal
pub fn handler(ctx: Context<WithdrawFees>, amount: Option<u64>) -> Result<()> {
    let market = &ctx.accounts.market;

    // Determine amount to withdraw (all fees if not specified)
    let withdraw_amount = amount.unwrap_or(market.accumulated_fees);

    require!(withdraw_amount > 0, MarketError::InvalidAmount);
    require!(
        withdraw_amount <= market.accumulated_fees,
        MarketError::InsufficientFees
    );

    // Transfer fees from vault to treasury
    let seeds = &[
        Market::SEED_PREFIX,
        market.market_id.as_bytes(),
        &[market.bump],
    ];
    let signer_seeds = &[&seeds[..]];

    let transfer_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.vault.to_account_info(),
            to: ctx.accounts.treasury.to_account_info(),
            authority: ctx.accounts.market.to_account_info(),
        },
        signer_seeds,
    );
    token::transfer(transfer_ctx, withdraw_amount)?;

    // Update market state
    let market = &mut ctx.accounts.market;
    market.accumulated_fees = market
        .accumulated_fees
        .checked_sub(withdraw_amount)
        .ok_or(MarketError::ArithmeticOverflow)?;

    // Also decrease total_collateral since fees are part of vault balance
    market.total_collateral = market
        .total_collateral
        .checked_sub(withdraw_amount)
        .ok_or(MarketError::ArithmeticOverflow)?;

    emit!(FeesWithdrawn {
        market: market.key(),
        authority: ctx.accounts.authority.key(),
        treasury: ctx.accounts.treasury.key(),
        amount: withdraw_amount,
        remaining_fees: market.accumulated_fees,
    });

    Ok(())
}

#[event]
pub struct FeesWithdrawn {
    pub market: Pubkey,
    pub authority: Pubkey,
    pub treasury: Pubkey,
    pub amount: u64,
    pub remaining_fees: u64,
}
