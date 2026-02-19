use anchor_lang::prelude::*;
use anchor_spl::token_2022::{Token2022, Mint, TokenAccount};
use anchor_spl::associated_token::AssociatedToken;

use crate::state::*;
use crate::errors::PumpFunError;
use crate::constants::*;

/// Buy tokens from the bonding curve
/// 
/// Formula: Uses constant product (x * y = k) where:
/// - x = virtual SOL reserve
/// - y = virtual token reserve
/// - k = constant product
/// 
/// When buying: SOL in → tokens out
/// New reserves: (x + sol_in) * (y - tokens_out) = k
/// 
/// Accounts:
/// - buyer: Token buyer (signer, pays SOL)
/// - bonding_curve: Bonding curve state account
/// - mint: Token mint account
/// - buyer_token_account: Buyer's token account (receives tokens)
/// - global_config: Global protocol configuration
/// - treasury: Treasury account (receives protocol fees)
/// - token_program: Token-2022 program
/// - associated_token_program: Associated Token program
/// - system_program: System program
#[derive(Accounts)]
pub struct Buy<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

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
        associated_token::owner = buyer,
    )]
    pub buyer_token_account: Account<'info, TokenAccount>,

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
    ctx: Context<Buy>,
    sol_in: u64,
    min_tokens_out: u64,
) -> Result<()> {
    Buy::execute(ctx, sol_in, min_tokens_out)
}

impl<'info> Buy<'info> {
    fn execute(
        ctx: Context<Buy>,
        sol_in: u64,
        min_tokens_out: u64,
    ) -> Result<()> {
        let bonding_curve = &mut ctx.accounts.bonding_curve;
        let buyer = &ctx.accounts.buyer;
        let clock = Clock::get()?;

        // Validate input
        require!(sol_in >= MIN_SOL_AMOUNT, PumpFunError::MinSolAmountNotMet);
        require!(sol_in > 0, PumpFunError::InvalidAmount);

        // Calculate tokens out using bonding curve formula
        let tokens_out = calculate_tokens_out(
            sol_in,
            bonding_curve.virtual_sol_reserve,
            bonding_curve.virtual_token_reserve,
        )?;

        // Slippage check
        require!(
            tokens_out >= min_tokens_out,
            PumpFunError::SlippageExceeded
        );

        // Calculate protocol fee
        let protocol_fee = (sol_in as u128)
            .checked_mul(PROTOCOL_FEE_BPS as u128)
            .ok_or(PumpFunError::MathOverflow)?
            .checked_div(10000)
            .ok_or(PumpFunError::MathOverflow)? as u64;

        let sol_after_fee = sol_in
            .checked_sub(protocol_fee)
            .ok_or(PumpFunError::MathOverflow)?;

        // Transfer SOL from buyer to bonding curve (virtual reserve update)
        // In reality, SOL goes to treasury/accumulates for LP
        **buyer.to_account_info().try_borrow_mut_lamports()? -= sol_in;
        **ctx.accounts.treasury.to_account_info().try_borrow_mut_lamports()? += sol_after_fee;
        **ctx.accounts.treasury.to_account_info().try_borrow_mut_lamports()? += protocol_fee;

        // Mint tokens to buyer
        // Note: The bonding_curve PDA should be set as the mint authority
        // This allows the program to mint tokens when users buy
        let seeds = &[
            b"bonding_curve",
            bonding_curve.mint.as_ref(),
            &[bonding_curve.bump],
        ];
        let signer = &[&seeds[..]];

        let cpi_accounts = anchor_spl::token_2022::MintTo {
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.buyer_token_account.to_account_info(),
            authority: ctx.accounts.bonding_curve.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(
            cpi_program,
            cpi_accounts,
            signer,
        );
        anchor_spl::token_2022::mint_to(cpi_ctx, tokens_out)?;

        // Update bonding curve state
        bonding_curve.update_after_buy(sol_in, tokens_out);

        // Check if curve is complete
        let is_complete = is_complete(bonding_curve.virtual_sol_reserve);
        if is_complete {
            bonding_curve.complete(&clock);
        }

        // Emit buy event
        emit!(TokenBought {
            mint: bonding_curve.mint,
            buyer: buyer.key(),
            sol_in,
            tokens_out,
            virtual_sol_reserve: bonding_curve.virtual_sol_reserve,
            virtual_token_reserve: bonding_curve.virtual_token_reserve,
            completed: is_complete,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }
}

#[event]
pub struct TokenBought {
    pub mint: Pubkey,
    pub buyer: Pubkey,
    pub sol_in: u64,
    pub tokens_out: u64,
    pub virtual_sol_reserve: u64,
    pub virtual_token_reserve: u64,
    pub completed: bool,
    pub timestamp: i64,
}
