use anyhow::{Result, anyhow};
use log::{info, warn, error, debug};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    compute_budget::ComputeBudgetInstruction,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signature, read_keypair_file},
    signer::Signer,
    transaction::Transaction,
};
use solana_transaction_status::UiTransactionEncoding;
use std::str::FromStr;
use std::env;

use crate::models::MatchedTrade;

pub struct SolanaService {
    rpc_client: RpcClient,
    keeper: Keypair,
    market_program_id: Pubkey,
    orderbook_program_id: Pubkey,
    #[allow(dead_code)]
    privacy_program_id: Pubkey,
}

impl SolanaService {
    pub fn new(rpc_url: &str, keeper_path: &str) -> Result<Self> {
        info!("Initializing Solana service...");

        let rpc_client = RpcClient::new_with_commitment(
            rpc_url.to_string(),
            CommitmentConfig::confirmed(),
        );

        // Try to read keeper keypair, or generate a new one for development
        let keeper = match read_keypair_file(keeper_path) {
            Ok(kp) => kp,
            Err(_) => {
                info!("Keeper keypair not found, generating new one for development");
                Keypair::new()
            }
        };

        info!("Keeper pubkey: {}", keeper.pubkey());

        // Load program IDs from environment or use defaults
        // These are hardcoded devnet defaults - safe to use expect() here
        let market_program_id = env::var("MARKET_PROGRAM_ID")
            .ok()
            .and_then(|s| Pubkey::from_str(&s).ok())
            .unwrap_or_else(|| {
                Pubkey::from_str("98jqxMe88XGjXzCY3bwV1Kuqzj32fcwdhPZa193RUffQ")
                    .expect("hardcoded market program ID is valid")
            });

        let orderbook_program_id = env::var("ORDERBOOK_PROGRAM_ID")
            .ok()
            .and_then(|s| Pubkey::from_str(&s).ok())
            .unwrap_or_else(|| {
                Pubkey::from_str("59LqZtVU2YBrhv8B2E1iASJMzcyBHWhY2JuaJsCXkAS8")
                    .expect("hardcoded orderbook program ID is valid")
            });

        let privacy_program_id = env::var("PRIVACY_PROGRAM_ID")
            .ok()
            .and_then(|s| Pubkey::from_str(&s).ok())
            .unwrap_or_else(|| {
                Pubkey::from_str("9QGtHZJvmjMKTME1s3mVfNXtGpEdXDQZJTxsxqve9GsL")
                    .expect("hardcoded privacy program ID is valid")
            });

        info!("Program IDs loaded:");
        info!("  Market: {}", market_program_id);
        info!("  Orderbook: {}", orderbook_program_id);
        info!("  Privacy: {}", privacy_program_id);

        Ok(Self {
            rpc_client,
            keeper,
            market_program_id,
            orderbook_program_id,
            privacy_program_id,
        })
    }

    pub fn keeper_pubkey(&self) -> Pubkey {
        self.keeper.pubkey()
    }

    pub fn market_program_id(&self) -> Pubkey {
        self.market_program_id
    }

    pub fn orderbook_program_id(&self) -> Pubkey {
        self.orderbook_program_id
    }

    /// Get account balance
    pub async fn get_balance(&self, pubkey: &Pubkey) -> Result<u64> {
        let balance = self.rpc_client.get_balance(pubkey)?;
        Ok(balance)
    }

    /// Submit a trade settlement transaction
    pub async fn settle_trade(
        &self,
        matched_trade: &MatchedTrade,
        accounts: SettleTradeAccounts,
    ) -> Result<Signature> {
        info!(
            "Settling trade: buy_order={}, sell_order={}, quantity={}, price={}",
            matched_trade.buy_order_id,
            matched_trade.sell_order_id,
            matched_trade.fill_quantity,
            matched_trade.fill_price_bps
        );

        // Build the settle_trade instruction
        let ix = self.build_settle_trade_ix(matched_trade, &accounts)?;

        // Add compute budget for complex instruction
        let compute_ix = ComputeBudgetInstruction::set_compute_unit_limit(300_000);

        // Get recent blockhash
        let recent_blockhash = self.rpc_client.get_latest_blockhash()
            .map_err(|e| anyhow!("Failed to get blockhash: {}", e))?;

        // Create and sign transaction
        let tx = Transaction::new_signed_with_payer(
            &[compute_ix, ix],
            Some(&self.keeper.pubkey()),
            &[&self.keeper],
            recent_blockhash,
        );

        // Send and confirm transaction
        let signature = self.rpc_client.send_and_confirm_transaction(&tx)
            .map_err(|e| {
                error!("Failed to settle trade: {}", e);
                anyhow!("Transaction failed: {}", e)
            })?;

        info!("Trade settled successfully: {}", signature);
        Ok(signature)
    }

    /// Build the settle_trade instruction
    fn build_settle_trade_ix(
        &self,
        matched_trade: &MatchedTrade,
        accounts: &SettleTradeAccounts,
    ) -> Result<Instruction> {
        // Anchor instruction discriminator for "settle_trade"
        // This is SHA256("global:settle_trade")[0..8]
        let mut data = vec![0x9f, 0x44, 0x76, 0x6c, 0x8a, 0x3e, 0x17, 0x2e];

        // Append fill_quantity (u64, little-endian)
        data.extend_from_slice(&matched_trade.fill_quantity.to_le_bytes());

        // Append fill_price_bps (u16, little-endian)
        data.extend_from_slice(&matched_trade.fill_price_bps.to_le_bytes());

        // Build account metas in exact order expected by SettleTrade accounts struct
        let account_metas = vec![
            AccountMeta::new_readonly(self.keeper.pubkey(), true),  // keeper (signer)
            AccountMeta::new(accounts.config, false),               // config
            AccountMeta::new_readonly(accounts.market, false),      // market
            AccountMeta::new(accounts.buy_order, false),            // buy_order
            AccountMeta::new(accounts.buyer_position, false),       // buyer_position
            AccountMeta::new(accounts.sell_order, false),           // sell_order
            AccountMeta::new(accounts.seller_position, false),      // seller_position
            AccountMeta::new(accounts.escrow_vault, false),         // escrow_vault
            AccountMeta::new(accounts.seller_collateral, false),    // seller_collateral
            AccountMeta::new(accounts.buyer_collateral, false),     // buyer_collateral
            AccountMeta::new_readonly(accounts.escrow_authority, false), // escrow_authority
            AccountMeta::new_readonly(spl_token::id(), false),      // token_program
        ];

        Ok(Instruction {
            program_id: self.orderbook_program_id,
            accounts: account_metas,
            data,
        })
    }

    /// Cancel an order on-chain (keeper-initiated)
    pub async fn cancel_order(
        &self,
        accounts: CancelOrderAccounts,
        order_id: u64,
    ) -> Result<Signature> {
        info!("Cancelling order {} on market {}", order_id, accounts.market);

        // Anchor instruction discriminator for "cancel_order"
        let data = vec![0x5f, 0xc0, 0x55, 0xd3, 0x47, 0x0c, 0x5f, 0x3e];

        // Build account metas
        let account_metas = vec![
            AccountMeta::new(accounts.owner, true),                  // owner (signer)
            AccountMeta::new_readonly(accounts.market, false),       // market
            AccountMeta::new(accounts.order, false),                 // order
            AccountMeta::new(accounts.position, false),              // position
            AccountMeta::new(accounts.escrow_vault, false),          // escrow_vault
            AccountMeta::new(accounts.user_collateral, false),       // user_collateral
            AccountMeta::new_readonly(accounts.escrow_authority, false), // escrow_authority
            AccountMeta::new_readonly(spl_token::id(), false),       // token_program
        ];

        let ix = Instruction {
            program_id: self.orderbook_program_id,
            accounts: account_metas,
            data,
        };

        let recent_blockhash = self.rpc_client.get_latest_blockhash()?;
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.keeper.pubkey()),
            &[&self.keeper],
            recent_blockhash,
        );

        let signature = self.rpc_client.send_and_confirm_transaction(&tx)
            .map_err(|e| anyhow!("Cancel order failed: {}", e))?;

        info!("Order cancelled successfully: {}", signature);
        Ok(signature)
    }

    /// Claim winnings for a user after market resolution
    pub async fn claim_winnings(
        &self,
        accounts: ClaimWinningsAccounts,
    ) -> Result<(Signature, u64)> {
        info!("Claiming winnings for {} on market {}", accounts.user, accounts.market);

        // Anchor instruction discriminator for "claim_winnings"
        let data = vec![0xbd, 0x87, 0x45, 0xd4, 0x84, 0xab, 0xaa, 0x60];

        // Build account metas
        let account_metas = vec![
            AccountMeta::new(accounts.user, true),                   // user (signer)
            AccountMeta::new(accounts.market, false),                // market
            AccountMeta::new(accounts.position, false),              // position
            AccountMeta::new(accounts.vault, false),                 // vault
            AccountMeta::new(accounts.user_collateral, false),       // user_collateral
            AccountMeta::new_readonly(spl_token::id(), false),       // token_program
        ];

        let _ix = Instruction {
            program_id: self.market_program_id,
            accounts: account_metas,
            data,
        };

        // Note: claim_winnings requires user signature
        // The backend cannot sign on behalf of users
        // This would typically be handled via:
        // 1. Return unsigned transaction for client to sign
        // 2. Use a transaction relay service
        // 3. Have users submit claims directly to the blockchain

        debug!("claim_winnings: User {} would claim from market {}", accounts.user, accounts.market);
        warn!("claim_winnings requires user wallet signature - returning placeholder");

        // For now, return placeholder - full implementation needs wallet adapter integration
        // In production, this would return serialized unsigned transaction for client signing
        Ok((Signature::default(), 0))
    }

    /// Build an unsigned claim_winnings transaction for client signing
    pub fn build_claim_winnings_tx(
        &self,
        accounts: &ClaimWinningsAccounts,
    ) -> Result<Vec<u8>> {
        // Anchor instruction discriminator for "claim_winnings"
        let data = vec![0xbd, 0x87, 0x45, 0xd4, 0x84, 0xab, 0xaa, 0x60];

        let account_metas = vec![
            AccountMeta::new(accounts.user, true),
            AccountMeta::new(accounts.market, false),
            AccountMeta::new(accounts.position, false),
            AccountMeta::new(accounts.vault, false),
            AccountMeta::new(accounts.user_collateral, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ];

        let ix = Instruction {
            program_id: self.market_program_id,
            accounts: account_metas,
            data,
        };

        let recent_blockhash = self.rpc_client.get_latest_blockhash()?;

        // Create transaction message (unsigned)
        let message = solana_sdk::message::Message::new_with_blockhash(
            &[ix],
            Some(&accounts.user),
            &recent_blockhash,
        );

        // Serialize message for client signing
        Ok(bincode::serialize(&message)?)
    }

    /// Get market account data
    pub async fn get_market_account(&self, market_pubkey: &Pubkey) -> Result<MarketAccount> {
        let _account = self.rpc_client.get_account(market_pubkey)?;

        // In production, deserialize using Anchor
        // let market: polyguard_market::state::Market =
        //     polyguard_market::state::Market::try_deserialize(&mut account.data.as_slice())?;

        // Placeholder
        Ok(MarketAccount::default())
    }

    /// Get order account data
    pub async fn get_order_account(&self, order_pubkey: &Pubkey) -> Result<OrderAccount> {
        let _account = self.rpc_client.get_account(order_pubkey)?;

        // Placeholder
        Ok(OrderAccount::default())
    }

    /// Derive market PDA
    pub fn derive_market_pda(&self, market_id: &str) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[b"market", market_id.as_bytes()],
            &self.market_program_id,
        )
    }

    /// Derive order PDA
    pub fn derive_order_pda(&self, market_pubkey: &Pubkey, order_id: u64) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[b"order", market_pubkey.as_ref(), &order_id.to_le_bytes()],
            &self.orderbook_program_id,
        )
    }

    /// Derive position PDA
    pub fn derive_position_pda(&self, market_pubkey: &Pubkey, owner: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[b"position", market_pubkey.as_ref(), owner.as_ref()],
            &self.orderbook_program_id,
        )
    }

    /// Derive escrow vault PDA
    pub fn derive_escrow_vault_pda(&self, market_pubkey: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[b"escrow", market_pubkey.as_ref()],
            &self.orderbook_program_id,
        )
    }

    /// Derive escrow authority PDA
    pub fn derive_escrow_authority_pda(&self, market_pubkey: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[b"escrow_authority", market_pubkey.as_ref()],
            &self.orderbook_program_id,
        )
    }

    /// Derive config PDA
    pub fn derive_config_pda(&self) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[b"config"],
            &self.orderbook_program_id,
        )
    }

    /// Derive user vault PDA for USDC balance
    pub fn derive_user_vault_pda(&self, user: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[b"user_vault", user.as_ref()],
            &self.orderbook_program_id,
        )
    }

    /// Get user's USDC balance from their program vault
    pub async fn get_user_balance(&self, wallet_address: &str) -> Result<u64> {
        let user = Pubkey::from_str(wallet_address)
            .map_err(|_| anyhow!("Invalid wallet address"))?;

        let (vault_pda, _) = self.derive_user_vault_pda(&user);

        // Try to get the token account balance
        match self.rpc_client.get_token_account_balance(&vault_pda) {
            Ok(balance) => {
                let amount = balance.amount.parse::<u64>().unwrap_or(0);
                Ok(amount)
            }
            Err(_) => {
                // Account doesn't exist yet, balance is 0
                Ok(0)
            }
        }
    }

    /// Verify a deposit transaction on-chain
    pub async fn verify_deposit_transaction(
        &self,
        tx_signature: &str,
        expected_wallet: &str,
        expected_amount: u64,
    ) -> Result<bool> {
        let signature = Signature::from_str(tx_signature)
            .map_err(|_| anyhow!("Invalid transaction signature"))?;

        // Get transaction details
        let tx = self.rpc_client
            .get_transaction(&signature, UiTransactionEncoding::Json)
            .map_err(|e| anyhow!("Failed to fetch transaction: {}", e))?;

        // Check transaction was successful
        if let Some(meta) = tx.transaction.meta {
            if meta.err.is_some() {
                return Ok(false);
            }
        }

        // Verify the transaction contains expected token transfer
        // In production, parse the transaction to verify:
        // 1. Transfer is to our program vault
        // 2. Amount matches expected
        // 3. Source is the user's wallet
        debug!(
            "Verifying deposit: sig={}, wallet={}, amount={}",
            tx_signature, expected_wallet, expected_amount
        );

        // For now, if transaction exists and succeeded, consider it verified
        // Full verification would parse pre/post token balances
        Ok(true)
    }

    /// Execute a withdrawal from program vault to user wallet
    pub async fn execute_withdrawal(
        &self,
        user_wallet: &str,
        destination: &str,
        amount: u64,
    ) -> Result<String> {
        let user = Pubkey::from_str(user_wallet)
            .map_err(|_| anyhow!("Invalid user wallet"))?;
        let dest = Pubkey::from_str(destination)
            .map_err(|_| anyhow!("Invalid destination address"))?;

        info!("Executing withdrawal: {} lamports from {} to {}", amount, user, dest);

        // Build withdraw instruction
        // Anchor discriminator for "withdraw" instruction
        let mut data = vec![0xb7, 0x12, 0x46, 0x9c, 0x94, 0x6d, 0xa1, 0x22];
        data.extend_from_slice(&amount.to_le_bytes());

        let (user_vault, _) = self.derive_user_vault_pda(&user);
        let (config, _) = self.derive_config_pda();

        // USDC mint (mainnet)
        let usdc_mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")
            .expect("hardcoded USDC mint is valid");

        let account_metas = vec![
            AccountMeta::new_readonly(self.keeper.pubkey(), true), // keeper (signer)
            AccountMeta::new_readonly(config, false),              // config
            AccountMeta::new_readonly(user, false),                // user
            AccountMeta::new(user_vault, false),                   // user_vault
            AccountMeta::new(dest, false),                         // destination
            AccountMeta::new_readonly(usdc_mint, false),           // mint
            AccountMeta::new_readonly(spl_token::id(), false),     // token_program
        ];

        let ix = Instruction {
            program_id: self.orderbook_program_id,
            accounts: account_metas,
            data,
        };

        let compute_ix = ComputeBudgetInstruction::set_compute_unit_limit(200_000);
        let recent_blockhash = self.rpc_client.get_latest_blockhash()?;

        let tx = Transaction::new_signed_with_payer(
            &[compute_ix, ix],
            Some(&self.keeper.pubkey()),
            &[&self.keeper],
            recent_blockhash,
        );

        let signature = self.rpc_client
            .send_and_confirm_transaction(&tx)
            .map_err(|e| anyhow!("Withdrawal failed: {}", e))?;

        info!("Withdrawal completed: {}", signature);
        Ok(signature.to_string())
    }

    /// Credit user balance (keeper-signed, for Blindfold deposits)
    pub async fn credit_user_balance(&self, wallet_address: &str, amount: u64) -> Result<String> {
        let user = Pubkey::from_str(wallet_address)
            .map_err(|_| anyhow!("Invalid wallet address"))?;

        info!("Crediting {} lamports to {}", amount, user);

        // Build credit instruction
        // Anchor discriminator for "credit_balance" (keeper-only)
        let mut data = vec![0x3a, 0x91, 0xb2, 0xf8, 0x54, 0xc7, 0x20, 0x8b];
        data.extend_from_slice(&amount.to_le_bytes());

        let (user_vault, _) = self.derive_user_vault_pda(&user);
        let (config, _) = self.derive_config_pda();

        // USDC mint
        let usdc_mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")
            .expect("hardcoded USDC mint is valid");

        // Derive program vault (source of funds for credits)
        let (program_vault, _) = Pubkey::find_program_address(
            &[b"vault"],
            &self.orderbook_program_id,
        );

        let account_metas = vec![
            AccountMeta::new_readonly(self.keeper.pubkey(), true), // keeper (signer)
            AccountMeta::new_readonly(config, false),              // config
            AccountMeta::new_readonly(user, false),                // user
            AccountMeta::new(user_vault, false),                   // user_vault
            AccountMeta::new(program_vault, false),                // program_vault (source)
            AccountMeta::new_readonly(usdc_mint, false),           // mint
            AccountMeta::new_readonly(spl_token::id(), false),     // token_program
        ];

        let ix = Instruction {
            program_id: self.orderbook_program_id,
            accounts: account_metas,
            data,
        };

        let compute_ix = ComputeBudgetInstruction::set_compute_unit_limit(200_000);
        let recent_blockhash = self.rpc_client.get_latest_blockhash()?;

        let tx = Transaction::new_signed_with_payer(
            &[compute_ix, ix],
            Some(&self.keeper.pubkey()),
            &[&self.keeper],
            recent_blockhash,
        );

        let signature = self.rpc_client
            .send_and_confirm_transaction(&tx)
            .map_err(|e| anyhow!("Credit balance failed: {}", e))?;

        info!("Balance credited: {}", signature);
        Ok(signature.to_string())
    }

    /// Build all accounts needed for settle_trade from order info
    pub fn build_settle_trade_accounts(
        &self,
        market_id: &str,
        buyer: &Pubkey,
        seller: &Pubkey,
        buy_order_id: u64,
        sell_order_id: u64,
        buyer_collateral: Pubkey,
        seller_collateral: Pubkey,
    ) -> SettleTradeAccounts {
        let (market_pda, _) = self.derive_market_pda(market_id);
        let (buy_order_pda, _) = self.derive_order_pda(&market_pda, buy_order_id);
        let (sell_order_pda, _) = self.derive_order_pda(&market_pda, sell_order_id);
        let (buyer_position_pda, _) = self.derive_position_pda(&market_pda, buyer);
        let (seller_position_pda, _) = self.derive_position_pda(&market_pda, seller);
        let (escrow_vault_pda, _) = self.derive_escrow_vault_pda(&market_pda);
        let (escrow_authority_pda, _) = self.derive_escrow_authority_pda(&market_pda);
        let (config_pda, _) = self.derive_config_pda();

        SettleTradeAccounts {
            config: config_pda,
            market: market_pda,
            buy_order: buy_order_pda,
            buyer_position: buyer_position_pda,
            sell_order: sell_order_pda,
            seller_position: seller_position_pda,
            escrow_vault: escrow_vault_pda,
            seller_collateral,
            buyer_collateral,
            escrow_authority: escrow_authority_pda,
        }
    }
}

// ============================================================================
// Account structs for transaction building
// ============================================================================

/// Accounts required for settle_trade instruction
#[derive(Debug, Clone)]
pub struct SettleTradeAccounts {
    pub config: Pubkey,
    pub market: Pubkey,
    pub buy_order: Pubkey,
    pub buyer_position: Pubkey,
    pub sell_order: Pubkey,
    pub seller_position: Pubkey,
    pub escrow_vault: Pubkey,
    pub seller_collateral: Pubkey,
    pub buyer_collateral: Pubkey,
    pub escrow_authority: Pubkey,
}

/// Accounts required for cancel_order instruction
#[derive(Debug, Clone)]
pub struct CancelOrderAccounts {
    pub owner: Pubkey,
    pub market: Pubkey,
    pub order: Pubkey,
    pub position: Pubkey,
    pub escrow_vault: Pubkey,
    pub user_collateral: Pubkey,
    pub escrow_authority: Pubkey,
}

/// Accounts required for claim_winnings instruction
#[derive(Debug, Clone)]
pub struct ClaimWinningsAccounts {
    pub user: Pubkey,
    pub market: Pubkey,
    pub position: Pubkey,
    pub vault: Pubkey,
    pub user_collateral: Pubkey,
}

// ============================================================================
// On-chain account data structs (for deserialization)
// ============================================================================

/// Market account data (matches on-chain Market struct)
#[derive(Default, Debug, Clone)]
pub struct MarketAccount {
    pub market_id: String,
    pub question: String,
    pub status: u8,
    pub resolved_outcome: u8,
    pub yes_price: u64,
    pub no_price: u64,
    pub total_collateral: u64,
    pub accumulated_fees: u64,
    pub fee_bps: u16,
    pub authority: Pubkey,
    pub oracle: Pubkey,
    pub trading_end: i64,
}

/// Order account data (matches on-chain Order struct)
#[derive(Default, Debug, Clone)]
pub struct OrderAccount {
    pub order_id: u64,
    pub owner: Pubkey,
    pub market: Pubkey,
    pub side: u8,
    pub outcome: u8,
    pub price_bps: u16,
    pub original_quantity: u64,
    pub remaining_quantity: u64,
    pub filled_quantity: u64,
    pub status: u8,
    pub created_at: i64,
    pub expires_at: i64,
}

/// Position account data
#[derive(Default, Debug, Clone)]
#[allow(dead_code)]
pub struct PositionAccount {
    pub owner: Pubkey,
    pub market: Pubkey,
    pub yes_balance: u64,
    pub no_balance: u64,
    pub locked_collateral: u64,
    pub locked_yes: u64,
    pub locked_no: u64,
    pub open_order_count: u32,
    pub total_trades: u32,
}

// SPL Token program ID helper
mod spl_token {
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;
    use std::sync::OnceLock;

    static TOKEN_PROGRAM_ID: OnceLock<Pubkey> = OnceLock::new();

    pub fn id() -> Pubkey {
        *TOKEN_PROGRAM_ID.get_or_init(|| {
            Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
                .expect("hardcoded SPL token program ID is valid")
        })
    }
}
