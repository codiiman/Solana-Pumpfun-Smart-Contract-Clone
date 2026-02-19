use anchor_lang::prelude::*;
use anchor_spl::token_2022::spl_token_2022::extension::metadata_pointer::MetadataPointer;
use anchor_spl::token_2022::spl_token_2022::extension::ExtensionType;
use anchor_spl::token_2022::spl_token_2022::extension::StateWithExtensionsMut;
use anchor_spl::token_2022::spl_token_2022::state::Mint;
use anchor_spl::token_2022::spl_token_metadata_interface::instruction::{
    CreateMetadataAccountV3, CreateMetadataAccountV3InstructionArgs,
};
use anchor_spl::token_2022::spl_token_metadata_interface::state::TokenMetadata;
use anchor_spl::token_2022::{Token2022, TokenAccount, Mint as TokenMint};
use anchor_spl::token::{self, Mint, TokenAccount as TokenAccountOld};
use crate::constants::*;

/// Global configuration account storing protocol-wide settings
#[account]
#[derive(Default)]
pub struct GlobalConfig {
    /// Authority that can update protocol parameters
    pub authority: Pubkey,
    /// Treasury PDA that receives protocol fees
    pub treasury: Pubkey,
    /// Protocol fee in basis points (e.g., 50 = 0.5%)
    pub protocol_fee_bps: u16,
    /// Token creation fee in lamports
    pub creation_fee: u64,
    /// Total number of tokens created
    pub total_tokens_created: u64,
    /// Bump seed for treasury PDA
    pub treasury_bump: u8,
}

impl GlobalConfig {
    pub const SIZE: usize = 8 + // discriminator
        32 + // authority
        32 + // treasury
        2 +  // protocol_fee_bps
        8 +  // creation_fee
        8 +  // total_tokens_created
        1;   // treasury_bump

    pub fn initialize(
        &mut self,
        authority: Pubkey,
        treasury: Pubkey,
        treasury_bump: u8,
    ) {
        self.authority = authority;
        self.treasury = treasury;
        self.protocol_fee_bps = PROTOCOL_FEE_BPS;
        self.creation_fee = CREATION_FEE;
        self.total_tokens_created = 0;
        self.treasury_bump = treasury_bump;
    }
}

/// Bonding curve account storing state for each token's bonding curve
#[account]
pub struct BondingCurve {
    /// Token mint address
    pub mint: Pubkey,
    /// Creator of the token
    pub creator: Pubkey,
    /// Virtual SOL reserve (starts at INITIAL_VIRTUAL_SOL_RESERVE)
    pub virtual_sol_reserve: u64,
    /// Virtual token reserve (starts at INITIAL_VIRTUAL_TOKEN_RESERVE)
    pub virtual_token_reserve: u64,
    /// Real SOL accumulated from buys (sent to treasury initially, then used for LP)
    pub real_sol_reserve: u64,
    /// Total tokens sold (minted and sold)
    pub tokens_sold: u64,
    /// Whether the bonding curve has been completed/graduated
    pub completed: bool,
    /// Timestamp when curve was created
    pub created_at: i64,
    /// Timestamp when curve was completed (if applicable)
    pub completed_at: Option<i64>,
    /// Bump seed for this bonding curve PDA
    pub bump: u8,
}

impl BondingCurve {
    pub const SIZE: usize = 8 + // discriminator
        32 + // mint
        32 + // creator
        8 +  // virtual_sol_reserve
        8 +  // virtual_token_reserve
        8 +  // real_sol_reserve
        8 +  // tokens_sold
        1 +  // completed
        8 +  // created_at
        9 +  // completed_at (Option<i64>)
        1;   // bump

    pub fn initialize(
        &mut self,
        mint: Pubkey,
        creator: Pubkey,
        bump: u8,
        clock: &Clock,
    ) {
        self.mint = mint;
        self.creator = creator;
        self.virtual_sol_reserve = INITIAL_VIRTUAL_SOL_RESERVE;
        self.virtual_token_reserve = INITIAL_VIRTUAL_TOKEN_RESERVE;
        self.real_sol_reserve = 0;
        self.tokens_sold = 0;
        self.completed = false;
        self.created_at = clock.unix_timestamp;
        self.completed_at = None;
        self.bump = bump;
    }

    /// Update reserves after a buy operation
    pub fn update_after_buy(&mut self, sol_in: u64, tokens_out: u64) {
        self.virtual_sol_reserve = self.virtual_sol_reserve
            .checked_add(sol_in)
            .expect("Math overflow");
        self.virtual_token_reserve = self.virtual_token_reserve
            .checked_sub(tokens_out)
            .expect("Math overflow");
        self.real_sol_reserve = self.real_sol_reserve
            .checked_add(sol_in)
            .expect("Math overflow");
        self.tokens_sold = self.tokens_sold
            .checked_add(tokens_out)
            .expect("Math overflow");
    }

    /// Update reserves after a sell operation
    pub fn update_after_sell(&mut self, tokens_in: u64, sol_out: u64) {
        self.virtual_sol_reserve = self.virtual_sol_reserve
            .checked_sub(sol_out)
            .expect("Math overflow");
        self.virtual_token_reserve = self.virtual_token_reserve
            .checked_add(tokens_in)
            .expect("Math overflow");
        self.real_sol_reserve = self.real_sol_reserve
            .checked_sub(sol_out)
            .expect("Math overflow");
        self.tokens_sold = self.tokens_sold
            .checked_sub(tokens_in)
            .expect("Math overflow");
    }

    /// Mark bonding curve as completed
    pub fn complete(&mut self, clock: &Clock) {
        self.completed = true;
        self.completed_at = Some(clock.unix_timestamp);
    }
}
