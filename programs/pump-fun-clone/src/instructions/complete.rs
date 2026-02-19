use anchor_lang::prelude::*;

use crate::state::*;
use crate::errors::PumpFunError;
use crate::constants::*;

/// Complete/graduate the bonding curve to a DEX liquidity pool
/// 
/// This instruction can be called by anyone once the bonding curve reaches
/// the completion threshold. It marks the curve as complete and prepares
/// for DEX pool creation.
/// 
/// In a full implementation, this would:
/// 1. Create a Raydium/PumpSwap liquidity pool
/// 2. Add liquidity using accumulated SOL + reserved tokens
/// 3. Burn or lock LP tokens
/// 4. Disable further trading on the bonding curve
/// 
/// For now, this is a stub that marks completion and emits an event.
/// 
/// Accounts:
/// - completer: Anyone can call this (signer)
/// - bonding_curve: Bonding curve state account
/// - mint: Token mint account
/// - global_config: Global protocol configuration
#[derive(Accounts)]
pub struct Complete<'info> {
    pub completer: Signer<'info>,

    #[account(
        mut,
        seeds = [b"bonding_curve", bonding_curve.mint.as_ref()],
        bump = bonding_curve.bump,
        constraint = !bonding_curve.completed @ PumpFunError::AlreadyCompleted,
        constraint = is_complete(bonding_curve.virtual_sol_reserve) @ PumpFunError::NotCompleted
    )]
    pub bonding_curve: Account<'info, BondingCurve>,

    /// CHECK: Token mint validated by constraint
    #[account(
        constraint = mint.key() == bonding_curve.mint @ PumpFunError::InvalidTokenMint
    )]
    pub mint: UncheckedAccount<'info>,

    #[account(
        seeds = [b"global_config"],
        bump
    )]
    pub global_config: Account<'info, GlobalConfig>,
}

pub fn handler(ctx: Context<Complete>) -> Result<()> {
    Complete::execute(ctx)
}

impl<'info> Complete<'info> {
    fn execute(ctx: Context<Complete>) -> Result<()> {
        let bonding_curve = &mut ctx.accounts.bonding_curve;
        let clock = Clock::get()?;

        // Verify completion threshold is met
        require!(
            is_complete(bonding_curve.virtual_sol_reserve),
            PumpFunError::NotCompleted
        );

        // Mark as completed
        bonding_curve.complete(&clock);

        // Emit completion event
        // In production, this would trigger DEX pool creation via CPI
        emit!(CurveCompleted {
            mint: bonding_curve.mint,
            creator: bonding_curve.creator,
            virtual_sol_reserve: bonding_curve.virtual_sol_reserve,
            virtual_token_reserve: bonding_curve.virtual_token_reserve,
            real_sol_reserve: bonding_curve.real_sol_reserve,
            tokens_sold: bonding_curve.tokens_sold,
            completed_at: bonding_curve.completed_at.unwrap(),
            timestamp: clock.unix_timestamp,
        });

        // TODO: In production, add CPI calls here to:
        // 1. Create Raydium/PumpSwap pool
        // 2. Add liquidity: real_sol_reserve SOL + RESERVED_TOKENS tokens
        // 3. Burn or lock LP tokens
        // 4. Transfer ownership/disable bonding curve

        Ok(())
    }
}

#[event]
pub struct CurveCompleted {
    pub mint: Pubkey,
    pub creator: Pubkey,
    pub virtual_sol_reserve: u64,
    pub virtual_token_reserve: u64,
    pub real_sol_reserve: u64,
    pub tokens_sold: u64,
    pub completed_at: i64,
    pub timestamp: i64,
}
