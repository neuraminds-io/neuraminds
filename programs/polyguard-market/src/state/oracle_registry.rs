use anchor_lang::prelude::*;

/// Maximum number of approved oracles in the registry
pub const MAX_ORACLES: usize = 32;

/// Oracle Registry - stores list of approved oracles for market resolution
#[account]
#[derive(InitSpace)]
pub struct OracleRegistry {
    /// Registry authority (can add/remove oracles)
    pub authority: Pubkey,

    /// List of approved oracle pubkeys
    #[max_len(32)]
    pub oracles: Vec<Pubkey>,

    /// Whether to enforce oracle validation (can be disabled for testing)
    pub enforce_validation: bool,

    /// Bump seed for PDA
    pub bump: u8,
}

impl OracleRegistry {
    pub const SEED_PREFIX: &'static [u8] = b"oracle_registry";

    /// Check if an oracle is approved
    pub fn is_approved(&self, oracle: &Pubkey) -> bool {
        if !self.enforce_validation {
            return true;
        }
        self.oracles.contains(oracle)
    }

    /// Add an oracle to the registry
    pub fn add_oracle(&mut self, oracle: Pubkey) -> Result<()> {
        require!(
            self.oracles.len() < MAX_ORACLES,
            OracleRegistryError::RegistryFull
        );
        require!(
            !self.oracles.contains(&oracle),
            OracleRegistryError::OracleAlreadyRegistered
        );
        self.oracles.push(oracle);
        Ok(())
    }

    /// Remove an oracle from the registry
    pub fn remove_oracle(&mut self, oracle: &Pubkey) -> Result<()> {
        let idx = self.oracles.iter().position(|o| o == oracle)
            .ok_or(OracleRegistryError::OracleNotFound)?;
        self.oracles.remove(idx);
        Ok(())
    }
}

#[error_code]
pub enum OracleRegistryError {
    #[msg("Oracle registry is full")]
    RegistryFull,

    #[msg("Oracle is already registered")]
    OracleAlreadyRegistered,

    #[msg("Oracle not found in registry")]
    OracleNotFound,

    #[msg("Unauthorized: only registry authority")]
    UnauthorizedAuthority,

    #[msg("Oracle not approved for market resolution")]
    OracleNotApproved,
}
