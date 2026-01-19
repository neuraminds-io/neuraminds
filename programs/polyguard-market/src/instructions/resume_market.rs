use anchor_lang::prelude::*;
use crate::state::{Market, MarketStatus};
use crate::errors::MarketError;

#[derive(Accounts)]
pub struct ResumeMarket<'info> {
    #[account(
        constraint = authority.key() == market.authority @ MarketError::UnauthorizedAuthority
    )]
    pub authority: Signer<'info>,

    #[account(
        mut,
        constraint = market.status == MarketStatus::Paused @ MarketError::MarketNotPaused
    )]
    pub market: Account<'info, Market>,
}

pub fn handler(ctx: Context<ResumeMarket>) -> Result<()> {
    let clock = Clock::get()?;
    let market = &mut ctx.accounts.market;

    // Check if trading period hasn't ended
    if clock.unix_timestamp >= market.trading_end {
        market.status = MarketStatus::Closed;
    } else {
        market.status = MarketStatus::Active;
    }

    emit!(MarketResumed {
        market: market.key(),
        new_status: market.status,
        resumed_at: clock.unix_timestamp,
    });

    Ok(())
}

#[event]
pub struct MarketResumed {
    pub market: Pubkey,
    pub new_status: MarketStatus,
    pub resumed_at: i64,
}
