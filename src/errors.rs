use anchor_lang::prelude::error_code;

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid input")]
    InvalidInput,
    #[msg("Invalid proof")]
    InvalidProof,
    #[msg("Invalid token mint")]
    InvalidTokenMint,
    #[msg("Invalid deposit, too few tokens")]
    TooFewTokensSupplied,
    #[msg("Pool received X or Y token quantity is 0")]
    ReceivedZeroTokens,
    #[msg("Slippage tolerance exceeded")]
    SlippageExceeded,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Math underflow")]
    MathUnderflow,
    #[msg("Unable to create Groth16Verifier")]
    InvalidGroth16Verifier,
    #[msg("Invalid token order")]
    InvalidTokenOrder,
    #[msg("Invalid swap amount")]
    InvalidSwapAmount,
    #[msg("Invalid LP mint")]
    InvalidLpMint,
    #[msg("Invalid metadata account")]
    InvalidMetadataAccount,
    #[msg("Pool reserve and public signals mismatch")]
    PublicSignalAndPoolReserveMismatch,
    #[msg("Proof input not equal to pool input")]
    PoolInputAmountMismatch,
    #[msg("Proof amount received exceeds pool output")]
    PoolOutputAmountTooLow,
    #[msg("Unable to parse public signals")]
    InvalidPublicSignals,
    #[msg("LP mint already initialized")]
    LpMintAlreadyInitialized,
    #[msg("Liquidity too low")]
    LiquidityTooLow,
    #[msg("Invalid transfer calculation")]
    TransferFeeCalculateNotMatch,
    #[msg("Config is already initialized")]
    ConfigAlreadyExists,
    #[msg("Invalid admin address")]
    InvalidAdmin,
    #[msg("Insufficient SOL balance for WSOL deposit")]
    InsufficientSolBalance,
    #[msg("Order expired")]
    OrderExpired,
    #[msg("Order still valid")]
    OrderStillValid,
    #[msg("AMM is halted")]
    AmmHalted,
    #[msg("Order data doesn't match")]
    OrderDataMismatch,
    #[msg("Order already exists")]
    OrderAlreadyExists,
    #[msg("Liquidity tokens did not yield any pair tokens")]
    ZeroTokenOutput,
    #[msg("Insufficient pool token X balance")]
    InsufficientPoolTokenXBalance,
    #[msg("Insufficient pool token Y balance")]
    InsufficientPoolTokenYBalance,
    #[msg("Trade too big, exceeds max rate tolerance")]
    TradeTooBig,
    #[msg("Input amount too small")]
    InputAmountTooSmall,
    #[msg("Output is zero")]
    OutputIsZero,
    #[msg("Invalid associated token program")]
    InvalidAssociatedTokenProgram,
    #[msg("User token account X is uninitialized")]
    UserTokenAccountXUninitialized,
    #[msg("User token account Y is uninitialized")]
    UserTokenAccountYUninitialized,
    #[msg("Caller token account WSOL is uninitialized")]
    CallerTokenAccountWSolUninitialized
}
