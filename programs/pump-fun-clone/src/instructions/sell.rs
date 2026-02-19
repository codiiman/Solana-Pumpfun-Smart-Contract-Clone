use anchor_lang::prelude::*;
use anchor_spl::token_2022::{Token2022, Mint, TokenAccount};
use anchor_spl::associated_token::AssociatedToken;

use crate::state::*;
use crate::errors::PumpFunError;
use crate::constants::*;

/// Sell tokens back to the bonding curve
/// 
/// Formula: Uses constant product (x * y = k) where:
/// - x = virtual SOL reserve
/// - y = virtual token reserve
/// - k = constant product
/// 
/// When selling: tokens in → SOL out
/// New reserves: (x - sol_out) * (y + tokens_in) = k
/// 
/// Accounts:
/// - seller: Token seller (signer, burns tokens)
/// - bonding_curve: Bonding curve state account
/// - mint: Token mint account
/// - seller_token_account: Seller's token account (tokens burned from here)
/// - global_config: Global protocol configuration
/// - treasury: Treasury account (receives protocol fees)
/// - token_program: Token-2022 program
/// - associated_token_program: Associated Token program
/// - system_program: System program
#[derive(Accounts)]
pub struct Sell<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,

    #[account(
        mut,
        seeds = [b"bonding_curve", bonding_curve.mint.as_ref()],
        bump = bonding_curve.bump,
        constraint = !bonding_curve.completed @ PumpFunError::AlreadyCompleted
    )]
    pub bonding_curve: Account<'info, BondingCurve>,

    #[account(
        mut,
        constraint = mint.key() == bonding_curve.mint @ PumpFunError::InvalidTokenMint
    )]
    pub mint: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::owner = seller,
    )]
    pub seller_token_account: Account<'info, TokenAccount>,

    #[account(
        seeds = [b"global_config"],
        bump
    )]
    pub global_config: Account<'info, GlobalConfig>,

    /// CHECK: Treasury PDA validated by seeds
    #[account(
        mut,
        seeds = [b"treasury", global_config.key().as_ref()],
        bump = global_config.treasury_bump
    )]
    pub treasury: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<Sell>,
    tokens_in: u64,
    min_sol_out: u64,
) -> Result<()> {
    Sell::execute(ctx, tokens_in, min_sol_out)
}

impl<'info> Sell<'info> {
    fn execute(
        ctx: Context<Sell>,
        tokens_in: u64,
        min_sol_out: u64,
    ) -> Result<()> {
        let bonding_curve = &mut ctx.accounts.bonding_curve;
        let seller = &ctx.accounts.seller;
        let clock = Clock::get()?;

        // Validate input
        require!(tokens_in > 0, PumpFunError::InvalidAmount);

        // Check seller has enough tokens
        require!(
            ctx.accounts.seller_token_account.amount >= tokens_in,
            PumpFunError::InvalidAmount
        );

        // Calculate SOL out using bonding curve formula
        let sol_out = calculate_sol_out(
            tokens_in,
            bonding_curve.virtual_sol_reserve,
            bonding_curve.virtual_token_reserve,
        )?;

        // Slippage check
        require!(
            sol_out >= min_sol_out,
            PumpFunError::SlippageExceeded
        );

        // Calculate protocol fee
        let protocol_fee = (sol_out as u128)
            .checked_mul(PROTOCOL_FEE_BPS as u128)
            .ok_or(PumpFunError::MathOverflow)?
            .checked_div(10000)
            .ok_or(PumpFunError::MathOverflow)? as u64;

        let sol_after_fee = sol_out
            .checked_sub(protocol_fee)
            .ok_or(PumpFunError::MathOverflow)?;

        // Burn tokens from seller
        let cpi_accounts = anchor_spl::token_2022::Burn {
            mint: ctx.accounts.mint.to_account_info(),
            from: ctx.accounts.seller_token_account.to_account_info(),
            authority: seller.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        anchor_spl::token_2022::burn(cpi_ctx, tokens_in)?;

        // Transfer SOL to seller (from treasury/real reserves)
        // In reality, this comes from accumulated SOL reserves
        require!(
            ctx.accounts.treasury.lamports() >= sol_after_fee,
            PumpFunError::InsufficientLiquidity
        );

        **ctx.accounts.treasury.to_account_info().try_borrow_mut_lamports()? -= sol_after_fee;
        **seller.to_account_info().try_borrow_mut_lamports()? += sol_after_fee;

        // Protocol fee stays in treasury
        // (already accounted for in sol_after_fee calculation)

        // Update bonding curve state
        bonding_curve.update_after_sell(tokens_in, sol_out);

        // Emit sell event
        emit!(TokenSold {
            mint: bonding_curve.mint,
            seller: seller.key(),
            tokens_in,
            sol_out: sol_after_fee,
            virtual_sol_reserve: bonding_curve.virtual_sol_reserve,
            virtual_token_reserve: bonding_curve.virtual_token_reserve,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }
}

#[event]
pub struct TokenSold {
    pub mint: Pubkey,
    pub seller: Pubkey,
    pub tokens_in: u64,
    pub sol_out: u64,
    pub virtual_sol_reserve: u64,
    pub virtual_token_reserve: u64,
    pub timestamp: i64,
}
