use anchor_lang::prelude::*;

#[error_code]
pub enum PumpFunError {
    #[msg("Invalid amount: must be greater than zero")]
    InvalidAmount,

    #[msg("Invalid reserves: reserves must be greater than zero")]
    InvalidReserves,

    #[msg("Math overflow occurred")]
    MathOverflow,

    #[msg("Insufficient liquidity in bonding curve")]
    InsufficientLiquidity,

    #[msg("Bonding curve already completed")]
    AlreadyCompleted,

    #[msg("Bonding curve not yet completed")]
    NotCompleted,

    #[msg("Slippage tolerance exceeded")]
    SlippageExceeded,

    #[msg("Invalid token metadata")]
    InvalidMetadata,

    #[msg("Unauthorized: invalid authority")]
    Unauthorized,

    #[msg("Token creation fee insufficient")]
    InsufficientCreationFee,

    #[msg("Minimum SOL amount not met")]
    MinSolAmountNotMet,

    #[msg("Invalid token mint")]
    InvalidTokenMint,

    #[msg("Invalid token account")]
    InvalidTokenAccount,

    #[msg("Token account is not empty")]
    TokenAccountNotEmpty,
}
