use anchor_lang::prelude::error_code;

#[error_code]
pub enum ErrorCode {
    #[msg("Math lib: Math overflow")]
    MathLibMathOverflow,
    #[msg("Math lib: Insufficient pool token X balance")]
    MathLibInsufficientPoolTokenXBalance,
    #[msg("Math lib: Insufficient pool token Y balance")]
    MathLibInsufficientPoolTokenYBalance,
    #[msg("Math lib: Trade too big, exceeds max rate tolerance")]
    MathLibTradeTooBig,
    #[msg("Math lib: Input amount too small")]
    MathLibInputAmountTooSmall,
}
