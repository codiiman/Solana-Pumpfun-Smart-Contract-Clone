use anchor_lang::prelude::*;
use crate::errors::PumpFunError;

/// Total token supply: 1 billion tokens with 6 decimals
pub const TOTAL_SUPPLY: u64 = 1_000_000_000_000_000; // 1B * 10^6

/// Tokens available on bonding curve (793M tokens)
/// Remaining tokens are reserved for liquidity pool
pub const CURVE_TOKENS: u64 = 793_000_000_000_000; // 793M * 10^6

/// Reserved tokens for liquidity pool (207M tokens)
pub const RESERVED_TOKENS: u64 = 207_000_000_000_000; // 207M * 10^6

/// Initial virtual SOL reserve (starting point for bonding curve)
/// This determines the starting price
pub const INITIAL_VIRTUAL_SOL_RESERVE: u64 = 30_000_000_000; // 30 SOL in lamports

/// Initial virtual token reserve (starting point for bonding curve)
/// This is set to match CURVE_TOKENS initially
pub const INITIAL_VIRTUAL_TOKEN_RESERVE: u64 = CURVE_TOKENS;

/// Target virtual market cap at graduation (in lamports)
/// ~69k-100k USD equivalent, using ~$150/SOL = ~460-666 SOL
/// We use 500 SOL as a reasonable target
pub const TARGET_VIRTUAL_MC: u64 = 500_000_000_000; // 500 SOL in lamports

/// Protocol fee basis points (0.5% = 50 bps)
pub const PROTOCOL_FEE_BPS: u16 = 50;

/// Token creation fee (0.02 SOL)
pub const CREATION_FEE: u64 = 20_000_000; // 0.02 SOL in lamports

/// Minimum SOL amount for buy/sell operations (0.001 SOL)
pub const MIN_SOL_AMOUNT: u64 = 1_000_000; // 0.001 SOL

/// Slippage tolerance basis points (5% default)
pub const DEFAULT_SLIPPAGE_BPS: u16 = 500;

/// Calculate the constant product k = x * y
/// where x = virtual SOL reserve, y = virtual token reserve
#[inline]
pub fn calculate_k(sol_reserve: u64, token_reserve: u64) -> u128 {
    (sol_reserve as u128) * (token_reserve as u128)
}

/// Calculate tokens out given SOL in using constant product formula
/// Formula: tokens_out = (token_reserve * sol_in * (10000 - fee_bps)) / ((sol_reserve + sol_in) * 10000)
/// This maintains k = (sol_reserve + sol_in) * (token_reserve - tokens_out)
pub fn calculate_tokens_out(sol_in: u64, sol_reserve: u64, token_reserve: u64) -> Result<u64> {
    require!(sol_in > 0, PumpFunError::InvalidAmount);
    require!(sol_reserve > 0, PumpFunError::InvalidReserves);
    require!(token_reserve > 0, PumpFunError::InvalidReserves);

    let k = calculate_k(sol_reserve, token_reserve);
    let new_sol_reserve = sol_reserve.checked_add(sol_in).ok_or(PumpFunError::MathOverflow)?;
    
    // Calculate new token reserve: k / new_sol_reserve
    let new_token_reserve = (k / (new_sol_reserve as u128)) as u64;
    
    // Tokens out = old reserve - new reserve
    let tokens_out = token_reserve
        .checked_sub(new_token_reserve)
        .ok_or(PumpFunError::InsufficientLiquidity)?;

    // Apply protocol fee: reduce tokens out by fee percentage
    let fee_amount = (tokens_out as u128)
        .checked_mul(PROTOCOL_FEE_BPS as u128)
        .ok_or(PumpFunError::MathOverflow)?
        .checked_div(10000)
        .ok_or(PumpFunError::MathOverflow)?;
    
    let tokens_out_after_fee = tokens_out
        .checked_sub(fee_amount as u64)
        .ok_or(PumpFunError::MathOverflow)?;

    Ok(tokens_out_after_fee)
}

/// Calculate SOL out given tokens in using constant product formula
/// Formula: sol_out = (sol_reserve * tokens_in * (10000 - fee_bps)) / ((token_reserve + tokens_in) * 10000)
pub fn calculate_sol_out(tokens_in: u64, sol_reserve: u64, token_reserve: u64) -> Result<u64> {
    require!(tokens_in > 0, PumpFunError::InvalidAmount);
    require!(sol_reserve > 0, PumpFunError::InvalidReserves);
    require!(token_reserve > 0, PumpFunError::InvalidReserves);

    let k = calculate_k(sol_reserve, token_reserve);
    let new_token_reserve = token_reserve
        .checked_add(tokens_in)
        .ok_or(PumpFunError::MathOverflow)?;
    
    // Calculate new SOL reserve: k / new_token_reserve
    let new_sol_reserve = (k / (new_token_reserve as u128)) as u64;
    
    // SOL out = old reserve - new reserve
    let sol_out = sol_reserve
        .checked_sub(new_sol_reserve)
        .ok_or(PumpFunError::InsufficientLiquidity)?;

    // Apply protocol fee: reduce SOL out by fee percentage
    let fee_amount = (sol_out as u128)
        .checked_mul(PROTOCOL_FEE_BPS as u128)
        .ok_or(PumpFunError::MathOverflow)?
        .checked_div(10000)
        .ok_or(PumpFunError::MathOverflow)?;
    
    let sol_out_after_fee = sol_out
        .checked_sub(fee_amount as u64)
        .ok_or(PumpFunError::MathOverflow)?;

    Ok(sol_out_after_fee)
}

/// Check if bonding curve has reached completion threshold
/// Completion happens when virtual SOL reserve reaches target market cap
pub fn is_complete(sol_reserve: u64) -> bool {
    sol_reserve >= TARGET_VIRTUAL_MC
}
