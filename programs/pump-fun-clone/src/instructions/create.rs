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
use anchor_spl::associated_token::AssociatedToken;

use crate::state::*;
use crate::errors::PumpFunError;
use crate::constants::*;

/// Creates a new token with Token-2022, metadata, and initializes bonding curve
/// 
/// Accounts:
/// - creator: Token creator (signer, pays creation fee)
/// - mint: New token mint (Token-2022 with metadata extension)
/// - metadata: Token metadata account
/// - bonding_curve: Bonding curve state account (PDA)
/// - global_config: Global protocol configuration
/// - treasury: Treasury account (receives creation fee)
/// - token_program: Token-2022 program
/// - associated_token_program: Associated Token program
/// - metadata_program: Token Metadata program
/// - system_program: System program
/// - rent: Rent sysvar
#[derive(Accounts)]
pub struct Create<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    /// Token mint account (Token-2022 with metadata pointer extension)
    /// CHECK: Validated by Token-2022 program
    #[account(mut)]
    pub mint: UncheckedAccount<'info>,

    /// Token metadata account
    /// CHECK: Validated by metadata program
    #[account(mut)]
    pub metadata: UncheckedAccount<'info>,

    /// Bonding curve state account
    #[account(
        init,
        payer = creator,
        space = BondingCurve::SIZE,
        seeds = [b"bonding_curve", mint.key().as_ref()],
        bump
    )]
    pub bonding_curve: Account<'info, BondingCurve>,

    /// Global configuration account
    #[account(
        seeds = [b"global_config"],
        bump
    )]
    pub global_config: Account<'info, GlobalConfig>,

    /// Treasury account (receives creation fee)
    /// CHECK: Validated by seeds
    #[account(
        mut,
        seeds = [b"treasury", global_config.key().as_ref()],
        bump = global_config.treasury_bump
    )]
    pub treasury: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    /// CHECK: Token Metadata program
    pub metadata_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK: Rent sysvar
    pub rent: UncheckedAccount<'info>,
}

pub fn handler(
    ctx: Context<Create>,
    name: String,
    symbol: String,
    uri: String,
) -> Result<()> {
    Create::execute(ctx, name, symbol, uri)
}

impl<'info> Create<'info> {
    fn execute(
        ctx: Context<Create>,
        name: String,
        symbol: String,
        uri: String,
    ) -> Result<()> {
        let creator = &ctx.accounts.creator;
        let mint = &ctx.accounts.mint;
        let bonding_curve = &mut ctx.accounts.bonding_curve;
        let global_config = &ctx.accounts.global_config;
        let clock = Clock::get()?;

        // Verify creation fee payment
        require!(
            creator.lamports() >= global_config.creation_fee,
            PumpFunError::InsufficientCreationFee
        );

        // Transfer creation fee to treasury
        **ctx.accounts.creator.to_account_info().try_borrow_mut_lamports()? -= global_config.creation_fee;
        **ctx.accounts.treasury.to_account_info().try_borrow_mut_lamports()? += global_config.creation_fee;

        // Initialize bonding curve
        let bump = ctx.bumps.get("bonding_curve").unwrap();
        bonding_curve.initialize(mint.key(), creator.key(), *bump, &clock);

        // Emit create event
        emit!(TokenCreated {
            mint: mint.key(),
            creator: creator.key(),
            name: name.clone(),
            symbol: symbol.clone(),
            timestamp: clock.unix_timestamp,
        });

        // Note: Token mint and metadata initialization should be done via CPI
        // or in a separate instruction. For simplicity, we assume the mint
        // is already initialized with Token-2022 and metadata pointer extension.
        // In production, you would use CPI to:
        // 1. Initialize mint with Token-2022
        // 2. Set metadata pointer extension
        // 3. Create metadata account with name, symbol, uri

        Ok(())
    }
}

#[event]
pub struct TokenCreated {
    pub mint: Pubkey,
    pub creator: Pubkey,
    pub name: String,
    pub symbol: String,
    pub timestamp: i64,
}
