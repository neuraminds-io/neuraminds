use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Market {
    /// Unique market identifier
    #[max_len(64)]
    pub market_id: String,

    /// Market question/title
    #[max_len(256)]
    pub question: String,

    /// Market description
    #[max_len(512)]
    pub description: String,

    /// Market category
    #[max_len(32)]
    pub category: String,

    /// Market creator/authority
    pub authority: Pubkey,

    /// Resolution oracle
    pub oracle: Pubkey,

    /// YES outcome token mint
    pub yes_mint: Pubkey,

    /// NO outcome token mint
    pub no_mint: Pubkey,

    /// Collateral vault
    pub vault: Pubkey,

    /// Collateral mint (e.g., USDC)
    pub collateral_mint: Pubkey,

    /// Market status
    pub status: MarketStatus,

    /// Resolution deadline (Unix timestamp)
    pub resolution_deadline: i64,

    /// Trading end time (Unix timestamp)
    pub trading_end: i64,

    /// Resolved outcome (0 = unresolved, 1 = Yes, 2 = No)
    pub resolved_outcome: u8,

    /// Total collateral deposited
    pub total_collateral: u64,

    /// Total YES tokens minted
    pub total_yes_supply: u64,

    /// Total NO tokens minted
    pub total_no_supply: u64,

    /// Fee in basis points (100 = 1%)
    pub fee_bps: u16,

    /// Accumulated fees
    pub accumulated_fees: u64,

    /// Bump seed for PDA
    pub bump: u8,

    /// YES mint bump
    pub yes_mint_bump: u8,

    /// NO mint bump
    pub no_mint_bump: u8,

    /// Vault bump
    pub vault_bump: u8,

    /// Creation timestamp
    pub created_at: i64,

    /// Resolution timestamp
    pub resolved_at: i64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum MarketStatus {
    /// Trading is open
    Active,
    /// Trading temporarily halted
    Paused,
    /// Trading ended, awaiting resolution
    Closed,
    /// Outcome determined, claims available
    Resolved,
    /// Market cancelled, refunds available
    Cancelled,
}

impl Default for MarketStatus {
    fn default() -> Self {
        MarketStatus::Active
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum Outcome {
    Yes,
    No,
}

impl Market {
    pub const SEED_PREFIX: &'static [u8] = b"market";
    pub const YES_MINT_SEED: &'static [u8] = b"yes_mint";
    pub const NO_MINT_SEED: &'static [u8] = b"no_mint";
    pub const VAULT_SEED: &'static [u8] = b"vault";

    pub fn is_trading_active(&self, current_time: i64) -> bool {
        self.status == MarketStatus::Active && current_time < self.trading_end
    }

    pub fn can_resolve(&self, current_time: i64) -> bool {
        self.status == MarketStatus::Closed && current_time >= self.resolution_deadline
    }
}
