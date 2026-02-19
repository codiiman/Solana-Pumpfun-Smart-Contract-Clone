use anchor_lang::prelude::*;

use crate::state::*;
use crate::errors::PumpFunError;
use crate::constants::*;

/// Initialize the global configuration account
/// 
/// This must be called once by the protocol authority to set up
/// the global configuration and treasury PDA.
/// 
/// Accounts:
/// - authority: Protocol authority (signer)
/// - global_config: Global configuration account (PDA)
/// - treasury: Treasury account (PDA)
/// - system_program: System program
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = GlobalConfig::SIZE,
        seeds = [b"global_config"],
        bump
    )]
    pub global_config: Account<'info, GlobalConfig>,

    /// CHECK: Treasury PDA validated by seeds
    #[account(
        mut,
        seeds = [b"treasury", global_config.key().as_ref()],
        bump
    )]
    pub treasury: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Initialize>, authority: Pubkey) -> Result<()> {
    let global_config = &mut ctx.accounts.global_config;
    let treasury_bump = ctx.bumps.get("treasury").copied().unwrap();

    global_config.initialize(authority, ctx.accounts.treasury.key(), treasury_bump);

    Ok(())
}
