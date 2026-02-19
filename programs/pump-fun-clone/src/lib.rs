use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod state;
pub mod instructions;

use instructions::*;

declare_id!("PumpFunClone1111111111111111111111111");

#[program]
pub mod pump_fun_clone {
    use super::*;

    /// Initialize the global configuration
    /// 
    /// This should be called once by the protocol authority to set up
    /// the global configuration account.
    pub fn initialize(ctx: Context<Initialize>, authority: Pubkey) -> Result<()> {
        instructions::initialize::handler(ctx, authority)
    }

    /// Create a new token with bonding curve
    /// 
    /// Creates a Token-2022 mint with metadata and initializes the bonding curve.
    pub fn create(
        ctx: Context<Create>,
        name: String,
        symbol: String,
        uri: String,
    ) -> Result<()> {
        instructions::create::handler(ctx, name, symbol, uri)
    }

    /// Buy tokens from the bonding curve
    /// 
    /// Exchanges SOL for tokens using the constant product bonding curve formula.
    pub fn buy(
        ctx: Context<Buy>,
        sol_in: u64,
        min_tokens_out: u64,
    ) -> Result<()> {
        instructions::buy::handler(ctx, sol_in, min_tokens_out)
    }

    /// Sell tokens back to the bonding curve
    /// 
    /// Exchanges tokens for SOL using the constant product bonding curve formula.
    pub fn sell(
        ctx: Context<Sell>,
        tokens_in: u64,
        min_sol_out: u64,
    ) -> Result<()> {
        instructions::sell::handler(ctx, tokens_in, min_sol_out)
    }

    /// Complete/graduate the bonding curve to a DEX pool
    /// 
    /// Marks the curve as complete when threshold is reached. In production,
    /// this would trigger DEX pool creation.
    pub fn complete(ctx: Context<Complete>) -> Result<()> {
        instructions::complete::handler(ctx)
    }
}

// Re-export for external use
pub use state::*;
pub use errors::*;
pub use constants::*;
